//! Audio files in and out, and the small DSP the app needs around Clear:
//! resampling for the playback device and waveform overviews.

use std::path::Path;

/// Planar audio: one `Vec` per channel.
pub type Channels = Vec<Vec<f32>>;

/// Decode any file Symphonia understands (WAV, AIFF, FLAC, MP3, AAC/M4A,
/// ALAC, Ogg Vorbis, ...) to planar f32 at its own sample rate.
pub fn decode(path: &Path) -> Result<(Channels, f64), String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::errors::Error;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let file = std::fs::File::open(path).map_err(|e| format!("Could not open {name}: {e}"))?;
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = symphonia::default::get_probe()
        .format(&hint, stream, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|_| format!("{name} is not an audio file Soap can read."))?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| format!("{name} has no audio track."))?;
    let track_id = track.id;
    let mut rate = track.codec_params.sample_rate;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Soap can't decode the audio in {name} ({e})."))?;

    let mut channels: Channels = Vec::new();
    let mut buffer: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(Error::ResetRequired) => break,
            Err(e) => return Err(format!("Could not read {name}: {e}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            // A damaged packet: skip it, like players do.
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("Could not decode {name}: {e}")),
        };
        let spec = *decoded.spec();
        let n = spec.channels.count();
        rate.get_or_insert(spec.rate);
        if channels.is_empty() {
            channels = vec![Vec::new(); n];
        }
        if buffer.as_ref().is_none_or(|b| b.capacity() < decoded.capacity() * n) {
            buffer = Some(SampleBuffer::new(decoded.capacity() as u64, spec));
        }
        let buffer = buffer.as_mut().unwrap();
        buffer.copy_interleaved_ref(decoded);
        for frame in buffer.samples().chunks_exact(n) {
            for (channel, &sample) in channels.iter_mut().zip(frame) {
                channel.push(sample);
            }
        }
    }
    let rate = rate.ok_or_else(|| format!("{name} doesn't say its sample rate."))?;
    if channels.first().is_none_or(Vec::is_empty) {
        return Err(format!("{name} contains no audio."));
    }
    Ok((channels, rate as f64))
}

/// Cubic (Catmull-Rom) resampling. Good enough for monitoring; exports always
/// use Clear's own output rate and are never resampled here.
pub fn resample(input: &[f32], from: f64, to: f64) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let step = from / to;
    let len = (input.len() as f64 / step).round() as usize;
    let at = |i: isize| input[i.clamp(0, input.len() as isize - 1) as usize];
    (0..len)
        .map(|n| {
            let x = n as f64 * step;
            let i = x.floor() as isize;
            let t = (x - i as f64) as f32;
            let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
            p1 + 0.5 * t * (p2 - p0 + t * (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3 + t * (3.0 * (p1 - p2) + p3 - p0)))
        })
        .collect()
}

/// Min/max of all channels over `buckets` equal slices, for drawing.
pub fn peaks(channels: &[Vec<f32>], buckets: usize) -> Vec<[f32; 2]> {
    let len = channels.first().map_or(0, Vec::len);
    if len == 0 {
        return Vec::new();
    }
    let buckets = buckets.min(len);
    (0..buckets)
        .map(|b| {
            let (start, end) = (b * len / buckets, ((b + 1) * len / buckets).max(b * len / buckets + 1));
            channels.iter().fold([0.0f32, 0.0f32], |[lo, hi], ch| {
                ch[start..end].iter().fold([lo, hi], |[lo, hi], &s| [lo.min(s), hi.max(s)])
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WavFormat {
    Pcm16,
    Float32,
}

pub fn write_wav(path: &Path, channels: &[Vec<f32>], rate: f64, format: WavFormat) -> Result<(), String> {
    let err = |e: hound::Error| format!("Could not write {}: {e}", path.display());
    let (bits, sample_format) = match format {
        WavFormat::Pcm16 => (16, hound::SampleFormat::Int),
        WavFormat::Float32 => (32, hound::SampleFormat::Float),
    };
    let spec = hound::WavSpec {
        channels: channels.len() as u16,
        sample_rate: rate.round() as u32,
        bits_per_sample: bits,
        sample_format,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(err)?;
    for i in 0..channels.first().map_or(0, Vec::len) {
        for channel in channels {
            match format {
                WavFormat::Pcm16 => writer.write_sample((channel[i].clamp(-1.0, 1.0) * 32767.0).round() as i16),
                WavFormat::Float32 => writer.write_sample(channel[i]),
            }
            .map_err(err)?;
        }
    }
    writer.finalize().map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_keeps_duration_and_shape() {
        let sine: Vec<f32> = (0..44_100).map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin()).collect();
        let out = resample(&sine, 44_100.0, 48_000.0);
        assert_eq!(out.len(), 48_000);
        let expected = (1000.0 * 440.0 * std::f32::consts::TAU / 48_000.0).sin();
        assert!((out[1000] - expected).abs() < 1e-3);
    }

    #[test]
    fn peaks_cover_every_channel() {
        let p = peaks(&[vec![0.0, 0.5, 0.0, 0.0], vec![0.0, 0.0, -0.25, 0.0]], 2);
        assert_eq!(p, vec![[0.0, 0.5], [-0.25, 0.0]]);
    }

    #[test]
    fn wav_round_trip_through_the_decoder() {
        let path = std::env::temp_dir().join(format!("soap-app-test-{}.wav", std::process::id()));
        let audio = vec![vec![0.5, -0.5, 0.25], vec![0.0, 0.125, -1.0]];
        write_wav(&path, &audio, 48_000.0, WavFormat::Float32).unwrap();
        let (decoded, rate) = decode(&path).unwrap();
        write_wav(&path, &audio, 44_100.0, WavFormat::Pcm16).unwrap();
        let (pcm, pcm_rate) = decode(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!((decoded, rate), (audio.clone(), 48_000.0));
        assert_eq!(pcm_rate, 44_100.0);
        assert!(pcm.iter().flatten().zip(audio.iter().flatten()).all(|(a, b)| (a - b).abs() < 1e-4));
    }
}
