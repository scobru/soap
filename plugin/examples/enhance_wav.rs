//! Run Clear on a WAV file outside a DAW, to check the native install:
//! `cargo run --release --example enhance_wav -- in.wav out.wav`

use soap::engine::{ClearModel, EnhanceOptions};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let [_, input, output] = args.as_slice() else {
        return Err("usage: enhance_wav <input.wav> <output.wav>".into());
    };

    let mut reader = hound::WavReader::open(input).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let n = spec.channels as usize;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>(),
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            reader.samples::<i32>().map(|s| s.map(|v| v as f32 * scale)).collect()
        }
    }
    .map_err(|e| e.to_string())?;
    let mut channels = vec![Vec::new(); n];
    for (i, s) in samples.into_iter().enumerate() {
        channels[i % n].push(s);
    }

    // Like the plugin (and the SDK's own Node binding), call the blocking core
    // from a worker thread rather than the main one.
    let sample_rate = spec.sample_rate as f64;
    let result = std::thread::spawn(move || -> Result<_, String> {
        eprintln!("[1/3] Loading the Clear core...");
        let model = ClearModel::open()?;
        if !model.is_downloaded() {
            eprintln!("[2/3] Downloading the Clear model...");
            model.download()?;
        } else {
            eprintln!("[2/3] Model already cached");
        }
        eprintln!("[3/3] Enhancing {:.1}s of audio...", channels[0].len() as f64 / sample_rate);
        let result = model.enhance(
            &channels,
            sample_rate,
            &EnhanceOptions {
                strength: 1.0,
                target_lufs: Some(-19.0),
                peak_ceiling_dbfs: -1.5,
                max_gain_db: 9.0,
                output_sample_rate: 48_000.0,
                mono_downmix: true,
            },
        )?;
        Ok((result, channels[0].len()))
    })
    .join()
    .map_err(|_| "the Clear worker thread panicked".to_string())??;
    let (result, input_len) = result;

    let samples_out = &result.channels[0];
    if samples_out.is_empty() || samples_out.iter().any(|s| !s.is_finite()) {
        return Err("Clear returned empty or non-finite audio".into());
    }
    let expected = (input_len as f64 * result.sample_rate / spec.sample_rate as f64) as usize;
    if samples_out.len().abs_diff(expected) > 480 {
        return Err(format!("unexpected output length {} (expected ~{expected})", samples_out.len()));
    }

    let out_spec = hound::WavSpec {
        channels: result.channels.len() as u16,
        sample_rate: result.sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(output, out_spec).map_err(|e| e.to_string())?;
    for i in 0..result.channels[0].len() {
        for ch in &result.channels {
            writer.write_sample(ch[i]).map_err(|e| e.to_string())?;
        }
    }
    writer.finalize().map_err(|e| e.to_string())?;
    eprintln!(
        "{:.1}s processed at {:.1}x realtime, input {:?} LUFS, output peak {:?} dBTP",
        result.duration_sec,
        result.realtime_factor(),
        result.measured_lufs,
        result.measured_true_peak_dbfs
    );
    Ok(())
}
