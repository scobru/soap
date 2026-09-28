//! Soap desktop app: open a voice recording, clean it with Clear on this
//! computer, compare before and after, and save the clean version.
//! `soap-app --clean <in> <out.wav>` does the same without a window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod jobs;
mod player;
mod ui;

use soap::state::Loudness;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "Usage:
  soap-app [file]                      open the app (optionally with a recording)
  soap-app --clean <input> <output.wav> [options]
Options for --clean:
  --strength <0-100>                   blend with the original (default 100)
  --loudness <podcast|stream|tv|off>   LUFS target -19, -14, -23 or none (default podcast)
  --stereo                             keep stereo instead of mono
  --rate <48000|44100>                 output sample rate (default 48000)
  --float                              write 32-bit float instead of 16-bit";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--clean") => {
            attach_console();
            match cli(&args[1..]) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("Error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("--version") => {
            attach_console();
            println!("Soap {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help" | "-h") => {
            attach_console();
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(flag) if flag.starts_with('-') => {
            attach_console();
            eprintln!("Unknown option {flag}\n{USAGE}");
            ExitCode::from(2)
        }
        file => match gui(file.map(PathBuf::from)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Soap could not open its window: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

fn gui(open: Option<PathBuf>) -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Soap")
        .with_app_id("soap")
        .with_inner_size([720.0, 840.0])
        .with_min_inner_size([560.0, 560.0])
        .with_drag_and_drop(true);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/soap-256.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Soap", options, Box::new(|cc| Ok(Box::new(ui::SoapApp::new(cc, open)))))
}

fn cli(args: &[String]) -> Result<(), String> {
    let mut settings = jobs::Settings::default();
    let mut format = audio::WavFormat::Pcm16;
    let mut files = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().cloned().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--strength" => {
                let percent: f32 = value("--strength")?.parse().map_err(|_| "--strength takes 0-100")?;
                settings.strength = (percent / 100.0).clamp(0.0, 1.0);
            }
            "--loudness" => {
                settings.loudness = match value("--loudness")?.as_str() {
                    "podcast" => Loudness::ApplePodcasts,
                    "stream" => Loudness::Streaming,
                    "tv" => Loudness::Broadcast,
                    "off" => Loudness::Off,
                    other => return Err(format!("unknown loudness {other}")),
                }
            }
            "--rate" => settings.output_rate = value("--rate")?.parse().map_err(|_| "--rate takes 48000 or 44100")?,
            "--stereo" => settings.stereo = true,
            "--float" => format = audio::WavFormat::Float32,
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}\n{USAGE}")),
            file => files.push(PathBuf::from(file)),
        }
    }
    let [input, output] = files.as_slice() else {
        return Err(format!("--clean needs an input and an output file\n{USAGE}"));
    };
    let cleaned = jobs::clean_file(input, output, &settings, format)?;
    let seconds = cleaned.channels[0].len() as f64 / cleaned.rate;
    let finite = cleaned.channels.iter().flatten().all(|s| s.is_finite());
    eprintln!(
        "Saved {}: {seconds:.1} s at {:.0} Hz, {:.1}x realtime, input {} LUFS, output peak {} dBTP",
        output.display(),
        cleaned.rate,
        cleaned.realtime_factor,
        cleaned.input_lufs.map_or("-".into(), |v| format!("{v:.1}")),
        cleaned.true_peak_dbfs.map_or("-".into(), |v| format!("{v:.1}")),
    );
    if !finite {
        return Err("the clean audio contains invalid samples".into());
    }
    Ok(())
}

/// A GUI-subsystem exe on Windows has no console: borrow the terminal it was
/// started from, so command-line output shows up.
fn attach_console() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::{AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE};
        // Keep output that is already redirected (to a file or a pipe).
        if GetStdHandle(STD_ERROR_HANDLE).is_null() {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}
