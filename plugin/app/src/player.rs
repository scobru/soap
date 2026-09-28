//! Playback on the default output device, with instant A/B switching: both
//! versions are kept at the device rate on the same timeline, and the audio
//! callback reads whichever one is selected at the shared position.

use arc_swap::ArcSwapOption;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::audio::Channels;

#[derive(Default)]
struct State {
    playing: AtomicBool,
    /// Frames at the device rate.
    position: AtomicUsize,
    clean: AtomicBool,
    original: ArcSwapOption<Channels>,
    cleaned: ArcSwapOption<Channels>,
}

impl State {
    fn current(&self) -> Option<Arc<Channels>> {
        if self.clean.load(Ordering::Relaxed) {
            self.cleaned.load_full()
        } else {
            self.original.load_full()
        }
    }
}

pub struct Player {
    _stream: cpal::Stream,
    state: Arc<State>,
    pub rate: u32,
}

impl Player {
    pub fn new() -> Result<Self, String> {
        let device = cpal::default_host().default_output_device().ok_or("No audio output device found.")?;
        let supported = device.default_output_config().map_err(|e| format!("Audio output unavailable: {e}"))?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let state = Arc::new(State::default());
        let stream = match format {
            cpal::SampleFormat::F32 => build::<f32>(&device, &config, state.clone()),
            cpal::SampleFormat::I16 => build::<i16>(&device, &config, state.clone()),
            cpal::SampleFormat::U16 => build::<u16>(&device, &config, state.clone()),
            cpal::SampleFormat::I32 => build::<i32>(&device, &config, state.clone()),
            other => Err(format!("Unsupported audio output format {other}.")),
        }?;
        stream.play().map_err(|e| format!("Could not start audio output: {e}"))?;
        Ok(Self { _stream: stream, state, rate: config.sample_rate.0 })
    }

    pub fn set_tracks(&self, original: Option<Arc<Channels>>, cleaned: Option<Arc<Channels>>) {
        self.state.original.store(original);
        self.state.cleaned.store(cleaned);
    }

    pub fn set_clean(&self, clean: bool) {
        self.state.clean.store(clean, Ordering::Relaxed);
    }

    pub fn is_playing(&self) -> bool {
        self.state.playing.load(Ordering::Relaxed)
    }

    pub fn toggle(&self) {
        if self.is_playing() {
            self.state.playing.store(false, Ordering::Relaxed);
        } else if let Some(track) = self.state.current() {
            // From the top once the end was reached.
            if self.state.position.load(Ordering::Relaxed) + 1 >= track[0].len() {
                self.state.position.store(0, Ordering::Relaxed);
            }
            self.state.playing.store(true, Ordering::Relaxed);
        }
    }

    pub fn stop(&self) {
        self.state.playing.store(false, Ordering::Relaxed);
        self.state.position.store(0, Ordering::Relaxed);
    }

    pub fn position_sec(&self) -> f64 {
        self.state.position.load(Ordering::Relaxed) as f64 / self.rate as f64
    }

    pub fn seek(&self, seconds: f64) {
        self.state.position.store((seconds.max(0.0) * self.rate as f64) as usize, Ordering::Relaxed);
    }
}

fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    state: Arc<State>,
) -> Result<cpal::Stream, String> {
    let outputs = config.channels as usize;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let track = state.current();
                let start = state.position.load(Ordering::Relaxed);
                let playing = state.playing.load(Ordering::Relaxed);
                let (Some(track), true) = (track, playing) else {
                    data.fill(T::EQUILIBRIUM);
                    return;
                };
                let len = track[0].len();
                let mut position = start;
                for frame in data.chunks_mut(outputs) {
                    if position >= len {
                        frame.fill(T::EQUILIBRIUM);
                        continue;
                    }
                    for (out, sample) in frame.iter_mut().enumerate() {
                        // Mono feeds both speakers; channels past the second stay silent.
                        let value = if out < 2 { track[out.min(track.len() - 1)][position] } else { 0.0 };
                        *sample = T::from_sample(value);
                    }
                    position += 1;
                }
                if position >= len {
                    state.playing.store(false, Ordering::Relaxed);
                }
                // Unless the UI seeked meanwhile.
                let _ = state.position.compare_exchange(start, position.min(len), Ordering::Relaxed, Ordering::Relaxed);
            },
            |e| eprintln!("Soap: audio output error: {e}"),
            None,
        )
        .map_err(|e| format!("Could not open the audio output: {e}"))
}
