use nih_plug::prelude::Enum;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};

use crate::engine::EnhanceOptions;

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Monitor {
    #[name = "Clean"]
    Clean,
    #[name = "Original"]
    Original,
}

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Loudness {
    #[name = "Podcast -19"]
    ApplePodcasts,
    #[name = "Streaming -14"]
    Streaming,
    #[name = "Broadcast -23"]
    Broadcast,
    #[name = "Off"]
    Off,
}

impl Loudness {
    pub fn lufs(self) -> Option<f64> {
        match self {
            Loudness::ApplePodcasts => Some(-19.0),
            Loudness::Streaming => Some(-14.0),
            Loudness::Broadcast => Some(-23.0),
            Loudness::Off => None,
        }
    }
}

/// The settings a clean render was made with, so the UI can say when the
/// current parameters no longer match it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSettings {
    pub strength: f32,
    pub loudness: Loudness,
    pub ceiling_db: f32,
    pub max_gain_db: f32,
    pub stereo: bool,
}

impl RenderSettings {
    pub fn to_options(&self, output_sample_rate: f32, channels: usize) -> EnhanceOptions {
        EnhanceOptions {
            strength: self.strength as f64,
            target_lufs: self.loudness.lufs(),
            peak_ceiling_dbfs: self.ceiling_db as f64,
            max_gain_db: self.max_gain_db as f64,
            output_sample_rate: output_sample_rate as f64,
            mono_downmix: !(self.stereo && channels > 1),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderStats {
    pub input_lufs: Option<f64>,
    pub true_peak_dbfs: Option<f64>,
    pub realtime_factor: f64,
    pub duration_sec: f64,
}

/// What the project saves: a pointer to the take's audio on disk, never the
/// audio itself, so plugin state stays small.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeMeta {
    pub id: String,
    /// Timeline position of the first captured sample, at `sample_rate`.
    pub start: i64,
    pub sample_rate: f32,
    pub original: PathBuf,
    pub clean: PathBuf,
    pub settings: RenderSettings,
    pub stats: RenderStats,
}

/// A rendered take the audio thread plays back in place of the input.
pub struct CleanTake {
    pub start: i64,
    pub sample_rate: f32,
    pub channels: Vec<Vec<f32>>,
}

/// One block of captured audio in the ring: `frames` interleaved frames follow
/// in the data ring. `frames == 0` marks the end of a capture pass.
#[derive(Debug, Clone, Copy)]
pub struct BlockHeader {
    pub pos: i64,
    pub frames: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Empty,
    Armed,
    Captured,
    Busy(String),
    Ready,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Status {
    pub phase: Phase,
    pub captured_sec: f64,
    /// A capture is held in memory that has not become a take yet.
    pub has_capture: bool,
}

pub enum Command {
    Reset {
        headers: rtrb::Consumer<BlockHeader>,
        data: rtrb::Consumer<f32>,
        takes: rtrb::Producer<Option<Arc<CleanTake>>>,
        retired: rtrb::Consumer<Arc<CleanTake>>,
        channels: usize,
        sample_rate: f32,
    },
    Restore(TakeMeta),
    Arm,
    Disarm,
    Process,
    Discard,
    Quit,
}

pub struct Shared {
    /// Read by the audio thread: capture incoming audio while the transport plays.
    pub armed: AtomicBool,
    /// Set by the audio thread when the capture ring was full and audio was lost.
    pub overflow: AtomicBool,
    /// Set by the audio thread when the host reports no timeline position.
    pub no_position: AtomicBool,
    pub status: Mutex<Status>,
    pub commands: mpsc::Sender<Command>,
}

impl Shared {
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    pub fn set_phase(&self, phase: Phase) {
        self.status.lock().unwrap().phase = phase;
    }
}

pub fn takes_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("remove-that-dirt")
        .join("takes")
}
