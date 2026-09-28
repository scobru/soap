//! Everything slow runs on one worker thread: decoding, the model download
//! and Clear itself. The UI sends jobs and polls for events.

use serde::{Deserialize, Serialize};
use soap::engine::{ClearModel, EnhanceOptions};
use soap::state::{Loudness, RenderSettings};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::audio::{self, Channels};

/// Columns of the waveform overviews.
const PEAK_BUCKETS: usize = 2048;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Blend between the original (0) and fully cleaned (1).
    pub strength: f32,
    pub loudness: Loudness,
    pub ceiling_db: f32,
    pub max_gain_db: f32,
    pub stereo: bool,
    pub output_rate: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            strength: 1.0,
            loudness: Loudness::ApplePodcasts,
            ceiling_db: -1.5,
            max_gain_db: 9.0,
            stereo: false,
            output_rate: 48_000,
        }
    }
}

impl Settings {
    pub fn options(&self, channels: usize) -> EnhanceOptions {
        RenderSettings {
            strength: self.strength,
            loudness: self.loudness,
            ceiling_db: self.ceiling_db,
            max_gain_db: self.max_gain_db,
            stereo: self.stereo,
        }
        .to_options(self.output_rate as f32, channels)
    }
}

/// One version of the audio as the UI shows and plays it.
pub struct View {
    /// At the playback device's rate (shared with the source when it matches).
    pub playback: Arc<Channels>,
    pub peaks: Vec<[f32; 2]>,
}

impl View {
    fn new(channels: &Arc<Channels>, rate: f64, playback_rate: Option<u32>) -> Self {
        let playback = match playback_rate {
            Some(to) if to as f64 != rate => Arc::new(channels.iter().map(|c| audio::resample(c, rate, to as f64)).collect()),
            _ => channels.clone(),
        };
        Self { peaks: audio::peaks(channels, PEAK_BUCKETS), playback }
    }
}

pub struct Source {
    pub path: PathBuf,
    pub name: String,
    /// At most the first two channels of the file: Clear takes mono or stereo.
    pub channels: Arc<Channels>,
    pub file_channels: usize,
    pub rate: f64,
    pub duration: f64,
    pub view: View,
}

pub struct Cleaned {
    pub channels: Arc<Channels>,
    pub rate: f64,
    pub settings: Settings,
    pub input_lufs: Option<f64>,
    pub true_peak_dbfs: Option<f64>,
    pub realtime_factor: f64,
    pub view: View,
}

enum Job {
    Load(PathBuf),
    Clean(Arc<Source>, Settings),
}

pub enum Event {
    Busy(String),
    Loaded(Result<Arc<Source>, String>),
    Cleaned(Result<Arc<Cleaned>, String>),
}

pub struct Jobs {
    jobs: Sender<Job>,
    events: Receiver<Event>,
}

impl Jobs {
    /// `wake` is called after every event so the UI repaints and picks it up.
    pub fn spawn(playback_rate: Option<u32>, wake: impl Fn() + Send + 'static) -> Self {
        let (jobs, job_rx) = channel();
        let (event_tx, events) = channel();
        std::thread::Builder::new()
            .name("soap-worker".into())
            .spawn(move || {
                let mut worker = Worker { model: None, playback_rate };
                let send = |event: Event| {
                    let _ = event_tx.send(event);
                    wake();
                };
                for job in job_rx {
                    match job {
                        Job::Load(path) => {
                            send(Event::Busy("Opening…".into()));
                            send(Event::Loaded(worker.load(&path).map(Arc::new)));
                        }
                        Job::Clean(source, settings) => {
                            let result = worker.clean(&source, &settings, &|step| send(Event::Busy(step.into())));
                            send(Event::Cleaned(result.map(Arc::new)));
                        }
                    }
                }
            })
            .expect("could not start the worker thread");
        Self { jobs, events }
    }

    pub fn load(&self, path: PathBuf) {
        let _ = self.jobs.send(Job::Load(path));
    }

    pub fn clean(&self, source: Arc<Source>, settings: Settings) {
        let _ = self.jobs.send(Job::Clean(source, settings));
    }

    pub fn poll(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

struct Worker {
    model: Option<ClearModel>,
    playback_rate: Option<u32>,
}

impl Worker {
    fn load(&self, path: &Path) -> Result<Source, String> {
        let (mut channels, rate) = audio::decode(path)?;
        let file_channels = channels.len();
        channels.truncate(2);
        let channels = Arc::new(channels);
        Ok(Source {
            path: path.to_path_buf(),
            name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            duration: channels[0].len() as f64 / rate,
            file_channels,
            view: View::new(&channels, rate, self.playback_rate),
            channels,
            rate,
        })
    }

    fn model(&mut self, progress: &dyn Fn(&str)) -> Result<&ClearModel, String> {
        if self.model.is_none() {
            progress("Starting Clear…");
            let model = ClearModel::open()?;
            if !model.is_downloaded() {
                progress("Downloading the model (first time only)…");
                model.download()?;
            }
            self.model = Some(model);
        }
        Ok(self.model.as_ref().unwrap())
    }

    fn clean(&mut self, source: &Source, settings: &Settings, progress: &dyn Fn(&str)) -> Result<Cleaned, String> {
        let playback_rate = self.playback_rate;
        let model = self.model(progress)?;
        progress("Washing…");
        let result = model.enhance(&source.channels, source.rate, &settings.options(source.channels.len()))?;
        let realtime_factor = result.realtime_factor();
        let channels = Arc::new(result.channels);
        Ok(Cleaned {
            view: View::new(&channels, result.sample_rate, playback_rate),
            realtime_factor,
            input_lufs: result.measured_lufs,
            true_peak_dbfs: result.measured_true_peak_dbfs,
            rate: result.sample_rate,
            settings: settings.clone(),
            channels,
        })
    }
}

/// `soap-app --clean`: the same pipeline without a window.
pub fn clean_file(input: &Path, output: &Path, settings: &Settings, format: audio::WavFormat) -> Result<Cleaned, String> {
    let mut worker = Worker { model: None, playback_rate: None };
    eprintln!("Opening {}", input.display());
    let source = worker.load(input)?;
    let cleaned = worker.clean(&source, settings, &|step| eprintln!("{step}"))?;
    audio::write_wav(output, &cleaned.channels, cleaned.rate, format)?;
    Ok(cleaned)
}
