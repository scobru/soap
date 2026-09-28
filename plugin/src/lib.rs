use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, Mutex};

mod editor;
pub mod engine;
mod state;
mod worker;

use state::{BlockHeader, CleanTake, Command, Loudness, Monitor, Phase, RenderSettings, Shared, Status, TakeMeta};

/// Seconds of audio the capture ring holds between worker drains.
const RING_SECONDS: f32 = 4.0;

pub struct ClearVoice {
    params: Arc<ClearVoiceParams>,
    shared: Arc<Shared>,
    worker_rx: Option<mpsc::Receiver<Command>>,
    worker: Option<std::thread::JoinHandle<()>>,

    headers: Option<rtrb::Producer<BlockHeader>>,
    data: Option<rtrb::Producer<f32>>,
    takes: Option<rtrb::Consumer<Option<Arc<CleanTake>>>>,
    retired: Option<rtrb::Producer<Arc<CleanTake>>>,
    take: Option<Arc<CleanTake>>,
    in_pass: bool,
    sample_rate: f32,
}

#[derive(Params)]
pub struct ClearVoiceParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[persist = "take"]
    pub take: Arc<Mutex<Option<TakeMeta>>>,

    #[id = "monitor"]
    pub monitor: EnumParam<Monitor>,

    #[id = "strength"]
    pub strength: FloatParam,

    #[id = "loudness"]
    pub loudness: EnumParam<Loudness>,

    #[id = "ceiling"]
    pub ceiling: FloatParam,

    #[id = "max-gain"]
    pub max_gain: FloatParam,

    #[id = "stereo"]
    pub stereo: BoolParam,

    #[id = "auto"]
    pub auto_process: BoolParam,
}

impl Default for ClearVoiceParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(520, 600),
            take: Arc::new(Mutex::new(None)),
            monitor: EnumParam::new("Monitor", Monitor::Clean),
            strength: FloatParam::new("Strength", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            loudness: EnumParam::new("Loudness (LUFS)", Loudness::ApplePodcasts),
            ceiling: FloatParam::new("True peak ceiling", -1.5, FloatRange::Linear { min: -12.0, max: 0.0 })
                .with_step_size(0.1)
                .with_unit(" dBTP"),
            max_gain: FloatParam::new("Max gain", 9.0, FloatRange::Linear { min: 0.0, max: 30.0 })
                .with_step_size(0.5)
                .with_unit(" dB"),
            stereo: BoolParam::new("Keep stereo", false),
            auto_process: BoolParam::new("Clean when capture stops", true),
        }
    }
}

impl ClearVoiceParams {
    pub fn render_settings(&self) -> RenderSettings {
        RenderSettings {
            strength: self.strength.value(),
            loudness: self.loudness.value(),
            ceiling_db: self.ceiling.value(),
            max_gain_db: self.max_gain.value(),
            stereo: self.stereo.value(),
        }
    }
}

impl Default for ClearVoice {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            params: Arc::new(ClearVoiceParams::default()),
            shared: Arc::new(Shared {
                armed: Default::default(),
                overflow: Default::default(),
                no_position: Default::default(),
                status: Mutex::new(Status { phase: Phase::Empty, captured_sec: 0.0, has_capture: false }),
                commands: tx,
            }),
            worker_rx: Some(rx),
            worker: None,
            headers: None,
            data: None,
            takes: None,
            retired: None,
            take: None,
            in_pass: false,
            sample_rate: 48_000.0,
        }
    }
}

impl Plugin for ClearVoice {
    const NAME: &'static str = "Clear Voice";
    const VENDOR: &'static str = "Clear Voice";
    const URL: &'static str = "https://desertant.com/models/clear/";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.shared.clone())
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let channels = audio_io_layout.main_output_channels.map_or(2, |c| c.get() as usize);
        self.sample_rate = buffer_config.sample_rate;

        let ring = (buffer_config.sample_rate * RING_SECONDS) as usize * channels;
        let (headers_tx, headers_rx) = rtrb::RingBuffer::new(8192);
        let (data_tx, data_rx) = rtrb::RingBuffer::new(ring);
        let (takes_tx, takes_rx) = rtrb::RingBuffer::new(8);
        let (retired_tx, retired_rx) = rtrb::RingBuffer::new(8);
        self.headers = Some(headers_tx);
        self.data = Some(data_tx);
        self.takes = Some(takes_rx);
        self.retired = Some(retired_tx);
        self.in_pass = false;

        if let Some(rx) = self.worker_rx.take() {
            self.worker = Some(worker::Worker::spawn(self.shared.clone(), self.params.clone(), rx));
        }
        self.shared.send(Command::Reset {
            headers: headers_rx,
            data: data_rx,
            takes: takes_tx,
            retired: retired_rx,
            channels,
            sample_rate: buffer_config.sample_rate,
        });
        if let Some(meta) = self.params.take.lock().unwrap().clone() {
            self.shared.send(Command::Restore(meta));
        }
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if let Some(takes) = &mut self.takes {
            while let Ok(next) = takes.pop() {
                if let Some(old) = std::mem::replace(&mut self.take, next) {
                    // Hand the old take back so it is freed off the audio thread.
                    if let Some(Err(rtrb::PushError::Full(old))) = self.retired.as_mut().map(|r| r.push(old)) {
                        std::mem::forget(old);
                    }
                }
            }
        }

        let transport = context.transport();
        let playing = transport.playing;
        let pos = transport.pos_samples();
        let armed = self.shared.armed.load(Ordering::Acquire);

        if armed && playing {
            match pos {
                Some(pos) => {
                    self.capture_block(buffer, pos);
                    self.in_pass = true;
                }
                None => self.shared.no_position.store(true, Ordering::Relaxed),
            }
        } else if self.in_pass {
            if let Some(headers) = &mut self.headers {
                let _ = headers.push(BlockHeader { pos: 0, frames: 0 });
            }
            self.in_pass = false;
        }

        // While capturing the input passes through untouched.
        if armed || !playing || self.params.monitor.value() != Monitor::Clean {
            return ProcessStatus::Normal;
        }
        let (Some(pos), Some(take)) = (pos, &self.take) else {
            return ProcessStatus::Normal;
        };
        if (take.sample_rate - self.sample_rate).abs() > 0.5 {
            return ProcessStatus::Normal;
        }
        play_take(buffer.as_slice(), take, pos);
        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {
        self.in_pass = false;
    }
}

impl ClearVoice {
    fn capture_block(&mut self, buffer: &Buffer, pos: i64) {
        let (Some(headers), Some(data)) = (&mut self.headers, &mut self.data) else { return };
        let frames = buffer.samples();
        let slices = buffer.as_slice_immutable();
        let channels = slices.len();
        let n = frames * channels;
        if headers.slots() == 0 || data.slots() < n {
            self.shared.overflow.store(true, Ordering::Relaxed);
            return;
        }
        let Ok(chunk) = data.write_chunk_uninit(n) else { return };
        chunk.fill_from_iter((0..frames).flat_map(|i| slices.iter().map(move |ch| ch[i])));
        let _ = headers.push(BlockHeader { pos, frames: frames as u32 });
    }
}

/// Overwrite the part of the block (starting at timeline `pos`) that the take
/// covers. A mono take feeds every output channel.
fn play_take(outputs: &mut [&mut [f32]], take: &CleanTake, pos: i64) {
    let Some(frames) = outputs.first().map(|o| o.len() as i64) else { return };
    let len = take.channels[0].len() as i64;
    let first = (take.start - pos).max(0);
    let last = (take.start + len - pos).min(frames);
    for (c, out) in outputs.iter_mut().enumerate() {
        let src = &take.channels[c.min(take.channels.len() - 1)];
        for i in first..last {
            out[i as usize] = src[(pos + i - take.start) as usize];
        }
    }
}

impl Drop for ClearVoice {
    fn drop(&mut self) {
        self.shared.send(Command::Quit);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl ClapPlugin for ClearVoice {
    const CLAP_ID: &'static str = "com.clear-voice.clear-voice";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Offline voice cleanup (denoise, dereverb, loudness) with the Desert Ant Labs Clear model");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Restoration,
        ClapFeature::Mono,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for ClearVoice {
    const VST3_CLASS_ID: [u8; 16] = *b"ClearVoiceDALv01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Restoration];
}

nih_export_clap!(ClearVoice);
nih_export_vst3!(ClearVoice);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_replaces_only_the_covered_samples() {
        let take = CleanTake { start: 10, sample_rate: 48_000.0, channels: vec![vec![1.0, 2.0, 3.0]] };
        let (mut l, mut r) = ([0.5f32; 4], [0.5f32; 4]);
        // Block covers timeline 8..12: samples 10 and 11 come from the take.
        play_take(&mut [&mut l, &mut r], &take, 8);
        assert_eq!(l, [0.5, 0.5, 1.0, 2.0]);
        assert_eq!(r, l);

        let mut m = [0.5f32; 4];
        play_take(&mut [&mut m], &take, 12);
        assert_eq!(m, [3.0, 0.5, 0.5, 0.5]);

        let mut m = [0.5f32; 4];
        play_take(&mut [&mut m], &take, 100);
        assert_eq!(m, [0.5; 4]);
    }
}
