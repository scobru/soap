//! Everything that may block or allocate: draining the capture ring, running
//! Clear, reading and writing takes on disk.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::engine::ClearModel;
use crate::state::{
    takes_dir, BlockHeader, CleanTake, Command, Phase, RenderStats, Shared, TakeMeta,
};
use crate::SoapParams;

/// Longest capture kept in memory (one hour).
const MAX_CAPTURE_SEC: f64 = 3600.0;

struct Capture {
    start: i64,
    channels: Vec<Vec<f32>>,
}

struct Io {
    headers: rtrb::Consumer<BlockHeader>,
    data: rtrb::Consumer<f32>,
    takes: rtrb::Producer<Option<Arc<CleanTake>>>,
    retired: rtrb::Consumer<Arc<CleanTake>>,
    channels: usize,
    sample_rate: f32,
}

pub struct Worker {
    shared: Arc<Shared>,
    params: Arc<SoapParams>,
    io: Option<Io>,
    capture: Option<Capture>,
    model: Option<ClearModel>,
    /// Id and rate of the take the audio thread currently holds.
    loaded: Option<(String, f32)>,
    /// Between Arm and the end of that capture, so a late end-of-pass marker
    /// from the audio thread cannot finish the same capture twice.
    session: bool,
    preloaded: bool,
}

impl Worker {
    pub fn spawn(shared: Arc<Shared>, params: Arc<SoapParams>, rx: Receiver<Command>) -> std::thread::JoinHandle<()> {
        std::thread::Builder::new()
            .name("soap-worker".into())
            .spawn(move || {
                let mut worker = Worker { shared, params, io: None, capture: None, model: None, loaded: None, session: false, preloaded: false };
                worker.run(rx);
            })
            .expect("failed to spawn the Soap worker thread")
    }

    fn run(&mut self, rx: Receiver<Command>) {
        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Quit) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
            }
            self.drain();
            if let Some(io) = &mut self.io {
                // Old takes are dropped here rather than on the audio thread.
                while io.retired.pop().is_ok() {}
            }
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Reset { headers, data, takes, retired, channels, sample_rate } => {
                self.io = Some(Io { headers, data, takes, retired, channels, sample_rate });
                self.capture = None;
                self.loaded = None;
                self.session = false;
                self.shared.armed.store(false, Ordering::Release);
                self.sync_capture_flag();
                if !self.preloaded {
                    self.preloaded = true;
                    match crate::engine::preload() {
                        Ok(dir) => nih_plug::nih_log!("Soap: Clear core loaded from {}", dir.display()),
                        Err(e) => self.fail(e),
                    }
                }
            }
            Command::Restore(meta) => self.restore(meta),
            Command::Arm => {
                self.drain();
                self.capture = None;
                self.shared.overflow.store(false, Ordering::Relaxed);
                self.shared.no_position.store(false, Ordering::Relaxed);
                self.session = true;
                self.shared.armed.store(true, Ordering::Release);
                let mut status = self.shared.status.lock().unwrap();
                status.phase = Phase::Armed;
                status.captured_sec = 0.0;
                status.has_capture = false;
            }
            Command::Disarm => {
                self.shared.armed.store(false, Ordering::Release);
                self.drain();
                self.finish_capture();
            }
            Command::Process => self.process(),
            Command::Discard => {
                self.shared.armed.store(false, Ordering::Release);
                self.drain();
                self.capture = None;
                self.session = false;
                self.send_take(None);
                self.loaded = None;
                *self.params.take.lock().unwrap() = None;
                let mut status = self.shared.status.lock().unwrap();
                status.phase = Phase::Empty;
                status.captured_sec = 0.0;
                status.has_capture = false;
            }
            Command::Quit => {}
        }
    }

    fn drain(&mut self) {
        let Some(io) = &mut self.io else { return };
        let mut ended = false;
        while let Ok(header) = io.headers.pop() {
            if header.frames == 0 {
                ended = true;
                continue;
            }
            let n = header.frames as usize * io.channels;
            let Ok(chunk) = io.data.read_chunk(n) else {
                // The audio thread writes the data before its header, so this cannot happen.
                break;
            };
            let (a, b) = chunk.as_slices();
            let interleaved: Vec<f32> = a.iter().chain(b).copied().collect();
            chunk.commit_all();
            write_block(&mut self.capture, io.channels, header.pos, &interleaved);
        }
        let sample_rate = io.sample_rate as f64;
        if let Some(capture) = &self.capture {
            let captured_sec = capture.channels[0].len() as f64 / sample_rate;
            let mut status = self.shared.status.lock().unwrap();
            status.captured_sec = captured_sec;
            status.has_capture = true;
            drop(status);
            if captured_sec > MAX_CAPTURE_SEC {
                self.shared.armed.store(false, Ordering::Release);
                ended = true;
            }
        }
        if ended {
            self.finish_capture();
        }
    }

    /// A pass ended (transport stopped) or the user disarmed.
    fn finish_capture(&mut self) {
        if !self.session {
            return;
        }
        self.session = false;
        self.shared.armed.store(false, Ordering::Release);
        if self.capture.is_none() {
            let has_take = self.params.take.lock().unwrap().is_some();
            self.shared.set_phase(if has_take { Phase::Ready } else { Phase::Empty });
            return;
        }
        self.shared.set_phase(Phase::Captured);
        if self.params.auto_process.value() {
            self.process();
        }
    }

    fn process(&mut self) {
        let Some(io) = &self.io else { return };
        let sample_rate = io.sample_rate;
        let settings = self.params.render_settings();

        // A fresh capture becomes a new take; otherwise re-render the current one.
        let (id, start, channels, original_path) = if let Some(capture) = self.capture.take() {
            let id = new_take_id();
            let path = takes_dir().join(format!("{id}-original.wav"));
            if let Err(e) = write_wav(&path, &capture.channels, sample_rate) {
                self.fail(e);
                return;
            }
            (id, capture.start, capture.channels, path)
        } else if let Some(meta) = self.params.take.lock().unwrap().clone() {
            match read_wav(&meta.original) {
                Ok((channels, rate)) => {
                    let start = (meta.start as f64 * sample_rate as f64 / rate as f64).round() as i64;
                    (meta.id, start, channels, meta.original)
                }
                Err(e) => {
                    self.fail(e);
                    return;
                }
            }
        } else {
            return;
        };

        let source_rate = read_wav_rate(&original_path).unwrap_or(sample_rate);
        let result = self.model().and_then(|model| {
            model.enhance(&channels, source_rate as f64, &settings.to_options(sample_rate, channels.len()))
        });
        let result = match result {
            Ok(r) => r,
            Err(e) => {
                // Keep the capture so the user can retry without playing it again.
                if self.params.take.lock().unwrap().as_ref().map(|m| &m.id) != Some(&id) {
                    self.capture = Some(Capture { start, channels });
                    let _ = std::fs::remove_file(&original_path);
                }
                self.sync_capture_flag();
                self.fail(e);
                return;
            }
        };

        let clean = if (result.sample_rate as f32 - sample_rate).abs() > 0.5 {
            result.channels.iter().map(|c| resample_linear(c, result.sample_rate, sample_rate as f64)).collect()
        } else {
            result.channels.clone()
        };
        let clean_path = takes_dir().join(format!("{id}-clean.wav"));
        if let Err(e) = write_wav(&clean_path, &clean, sample_rate) {
            self.fail(e);
            return;
        }

        let meta = TakeMeta {
            id: id.clone(),
            start,
            sample_rate,
            original: original_path,
            clean: clean_path,
            settings,
            stats: RenderStats {
                input_lufs: result.measured_lufs,
                true_peak_dbfs: result.measured_true_peak_dbfs,
                realtime_factor: result.realtime_factor(),
                duration_sec: result.duration_sec,
            },
        };
        *self.params.take.lock().unwrap() = Some(meta);
        self.send_take(Some(CleanTake { start, sample_rate, channels: clean }));
        self.loaded = Some((id, sample_rate));
        self.sync_capture_flag();
        self.shared.set_phase(Phase::Ready);
    }

    /// Bring back the take a saved project points to.
    fn restore(&mut self, meta: TakeMeta) {
        let Some(io) = &self.io else { return };
        let sample_rate = io.sample_rate;
        if self.loaded.as_ref() == Some(&(meta.id.clone(), sample_rate)) {
            return;
        }
        match read_wav(&meta.clean) {
            Ok((channels, rate)) if (rate - sample_rate).abs() < 0.5 => {
                self.send_take(Some(CleanTake { start: meta.start, sample_rate, channels }));
                self.loaded = Some((meta.id, sample_rate));
                self.shared.set_phase(Phase::Ready);
            }
            // The project now runs at another rate: render again from the original.
            Ok(_) | Err(_) if meta.original.is_file() => {
                self.shared.set_phase(Phase::Busy("Re-rendering for the new sample rate…".into()));
                self.process();
            }
            Ok(_) | Err(_) => self.fail(format!("Take audio not found: {}", meta.clean.display())),
        }
    }

    fn model(&mut self) -> Result<&ClearModel, String> {
        if self.model.is_none() {
            self.shared.set_phase(Phase::Busy("Loading Clear…".into()));
            let model = ClearModel::open()?;
            if !model.is_downloaded() {
                self.shared.set_phase(Phase::Busy("Downloading the model (one time)…".into()));
                model.download()?;
            }
            self.model = Some(model);
        }
        self.shared.set_phase(Phase::Busy("Cleaning the voice…".into()));
        Ok(self.model.as_ref().unwrap())
    }

    fn sync_capture_flag(&self) {
        self.shared.status.lock().unwrap().has_capture = self.capture.is_some();
    }

    fn send_take(&mut self, take: Option<CleanTake>) {
        if let Some(io) = &mut self.io {
            let _ = io.takes.push(take.map(Arc::new));
        }
    }

    fn fail(&self, message: String) {
        nih_plug::nih_log!("Soap: {message}");
        self.shared.set_phase(Phase::Error(message));
    }
}

fn write_block(capture: &mut Option<Capture>, channels: usize, pos: i64, interleaved: &[f32]) {
    let frames = interleaved.len() / channels;
    let cap = capture.get_or_insert_with(|| Capture { start: pos, channels: vec![Vec::new(); channels] });
    if pos < cap.start {
        // Cycle/loop playback went back before the first captured sample.
        let shift = (cap.start - pos) as usize;
        for ch in &mut cap.channels {
            ch.splice(0..0, std::iter::repeat_n(0.0, shift));
        }
        cap.start = pos;
    }
    let offset = (pos - cap.start) as usize;
    for (c, ch) in cap.channels.iter_mut().enumerate() {
        if ch.len() < offset + frames {
            ch.resize(offset + frames, 0.0);
        }
        for i in 0..frames {
            ch[offset + i] = interleaved[i * channels + c];
        }
    }
}

fn new_take_id() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("take-{nanos:x}")
}

fn resample_linear(input: &[f32], from: f64, to: f64) -> Vec<f32> {
    let ratio = from / to;
    let len = (input.len() as f64 / ratio).round() as usize;
    (0..len)
        .map(|i| {
            let x = i as f64 * ratio;
            let j = x.floor() as usize;
            let frac = (x - j as f64) as f32;
            let a = input.get(j).copied().unwrap_or(0.0);
            let b = input.get(j + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}

fn write_wav(path: &Path, channels: &[Vec<f32>], sample_rate: f32) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("Could not write {}: {e}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| err(&e))?;
    }
    let spec = hound::WavSpec {
        channels: channels.len() as u16,
        sample_rate: sample_rate.round() as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| err(&e))?;
    for i in 0..channels[0].len() {
        for ch in channels {
            writer.write_sample(ch[i]).map_err(|e| err(&e))?;
        }
    }
    writer.finalize().map_err(|e| err(&e))
}

fn read_wav_rate(path: &Path) -> Option<f32> {
    hound::WavReader::open(path).ok().map(|r| r.spec().sample_rate as f32)
}

fn read_wav(path: &Path) -> Result<(Vec<Vec<f32>>, f32), String> {
    let err = |e: hound::Error| format!("Could not read {}: {e}", path.display());
    let mut reader = hound::WavReader::open(path).map_err(err)?;
    let spec = reader.spec();
    let n = spec.channels as usize;
    let samples: Vec<f32> = reader.samples::<f32>().collect::<Result<_, _>>().map_err(err)?;
    let mut channels = vec![Vec::with_capacity(samples.len() / n); n];
    for (i, s) in samples.into_iter().enumerate() {
        channels[i % n].push(s);
    }
    Ok((channels, spec.sample_rate as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_places_blocks_on_the_timeline() {
        let mut cap = None;
        write_block(&mut cap, 2, 100, &[1.0, -1.0, 2.0, -2.0]);
        write_block(&mut cap, 2, 104, &[5.0, -5.0]);
        let c = cap.as_ref().unwrap();
        assert_eq!(c.start, 100);
        assert_eq!(c.channels[0], vec![1.0, 2.0, 0.0, 0.0, 5.0]);
        assert_eq!(c.channels[1], vec![-1.0, -2.0, 0.0, 0.0, -5.0]);

        // A loop that jumps back before the start prepends silence.
        write_block(&mut cap, 2, 98, &[7.0, -7.0]);
        let c = cap.unwrap();
        assert_eq!(c.start, 98);
        assert_eq!(c.channels[0], vec![7.0, 0.0, 1.0, 2.0, 0.0, 0.0, 5.0]);
    }

    #[test]
    fn wav_round_trip() {
        let path = std::env::temp_dir().join(format!("soap-test-{}.wav", new_take_id()));
        write_wav(&path, &[vec![0.5, -0.5], vec![0.25, 0.0]], 44_100.0).unwrap();
        let (channels, rate) = read_wav(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(rate, 44_100.0);
        assert_eq!(channels, vec![vec![0.5, -0.5], vec![0.25, 0.0]]);
    }

    #[test]
    fn resample_keeps_duration() {
        let out = resample_linear(&vec![0.0; 48_000], 48_000.0, 44_100.0);
        assert_eq!(out.len(), 44_100);
    }
}
