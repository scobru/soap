use nih_plug::prelude::*;
use nih_plug_egui::create_egui_editor;
use nih_plug_egui::egui::{self, Align, Layout, RichText, Sense};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::state::{takes_dir, Command, Loudness, Monitor, Phase, Shared};
use crate::theme::{self, AQUA, CORAL, HONEY, MUTED};
use crate::SoapParams;

pub fn create(params: Arc<SoapParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |ctx, _| theme::install(ctx),
        move |ctx, setter, _| {
            let status = shared.status();
            let busy = matches!(status.phase, Phase::Busy(_));
            let animating = busy || status.phase == Phase::Armed;
            // Worker progress arrives off the GUI thread; animate while busy.
            ctx.request_repaint_after(Duration::from_millis(if animating { 16 } else { 200 }));
            let time = ctx.input(|i| i.time) as f32;
            egui::CentralPanel::default().frame(egui::Frame::default()).show(ctx, |ui| {
                let foam = if busy { 1.0 } else if animating { 0.25 } else { 0.0 };
                theme::paint_background(ui.painter(), ui.max_rect(), if animating { time } else { 0.0 }, foam);
                egui::Frame::default().inner_margin(20.0).show(ui, |ui| {
                    draw(ui, setter, &params, &shared, time);
                });
            });
        },
    )
}

fn draw(ui: &mut egui::Ui, setter: &ParamSetter, params: &SoapParams, shared: &Shared, time: f32) {
    let status = shared.status();
    let take = params.take.lock().unwrap().clone();
    let busy = matches!(status.phase, Phase::Busy(_));
    let armed = status.phase == Phase::Armed;
    let pulse = if busy || armed { (time * 4.0).sin() * 0.5 + 0.5 } else { 1.0 };

    ui.horizontal(|ui| {
        let (logo, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), Sense::hover());
        theme::paint_logo(ui.painter(), logo);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(4.0);
            ui.label(RichText::new("Soap").heading());
            ui.label(RichText::new("voice cleaner: denoise, dereverb, loudness").color(MUTED));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let text = match &status.phase {
                Phase::Empty => "Ready to lather".to_string(),
                Phase::Armed => format!("Capturing {}", fmt_time(status.captured_sec)),
                Phase::Captured => format!("Captured {}", fmt_time(status.captured_sec)),
                Phase::Busy(what) => what.clone(),
                Phase::Ready => "Squeaky clean".to_string(),
                Phase::Error(_) => "Something went wrong".to_string(),
            };
            theme::status_pill(ui, &text, theme::status_color(&status.phase), pulse);
        });
    });
    ui.add_space(6.0);

    theme::card(ui, |ui| {
        ui.horizontal(|ui| {
            let capture = if armed {
                theme::pill_button(ui, "     Stop", CORAL, true)
            } else {
                theme::pill_button(ui, "     Capture", CORAL, !busy)
            };
            let dot = capture.rect.left_center() + egui::vec2(24.0, 0.0);
            if armed {
                ui.painter().circle_filled(dot, 6.0, egui::Color32::WHITE.gamma_multiply(0.4 + 0.6 * pulse));
            } else {
                ui.painter().circle_filled(dot, 6.0, egui::Color32::WHITE.gamma_multiply(if busy { 0.5 } else { 1.0 }));
            }
            if capture.clicked() {
                shared.send(if armed { Command::Disarm } else { Command::Arm });
            }

            let can_process = !busy && !armed && (status.has_capture || take.is_some());
            let label = if take.is_some() && !status.has_capture { "Clean again" } else { "Clean" };
            if theme::pill_button(ui, label, AQUA, can_process).clicked() {
                shared.send(Command::Process);
            }
            let can_discard = !busy && (take.is_some() || status.has_capture);
            if theme::ghost_button(ui, "Discard", can_discard).clicked() {
                shared.send(Command::Discard);
            }
            if busy {
                ui.add(egui::Spinner::new().color(AQUA));
            }
        });

        let hint = match &status.phase {
            Phase::Empty => "Press Capture, then play the part to clean in your DAW. When you stop, Soap washes it and plays it back in place of the original.",
            Phase::Armed => "Play the part to clean. Stop the transport (or press Stop) when you're done.",
            Phase::Captured => "Capture complete: press Clean to wash it.",
            Phase::Busy(_) => "Scrubbing in the background. The input passes through untouched meanwhile.",
            Phase::Ready => "Play the project: the clean take replaces the original over the captured region. Compare with A / B.",
            Phase::Error(e) => e.as_str(),
        };
        let color = if matches!(status.phase, Phase::Error(_)) { CORAL } else { MUTED };
        ui.label(RichText::new(hint).color(color));
        if shared.overflow.load(Ordering::Relaxed) {
            ui.colored_label(CORAL, "Part of the capture was lost (system overloaded). Try capturing again.");
        }
        if shared.no_position.load(Ordering::Relaxed) {
            ui.colored_label(CORAL, "The host doesn't report a timeline position: capture isn't possible.");
        }

        ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 32.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new("Listen").color(MUTED));
            theme::segmented(ui, setter, &params.monitor, &[(Monitor::Original, "A · Original"), (Monitor::Clean, "B · Clean")]);
        });
    });
    ui.add_space(4.0);

    theme::card(ui, |ui| {
        let slider_width = (ui.available_width() - 150.0 - 96.0).max(140.0);
        let mastering = params.loudness.value() != Loudness::Off;
        egui::Grid::new("settings").num_columns(2).min_col_width(140.0).spacing([10.0, 12.0]).show(ui, |ui| {
            ui.label("Strength");
            ui.horizontal(|ui| {
                theme::soap_slider(ui, setter, &params.strength, slider_width);
                ui.label(params.strength.to_string());
            });
            ui.end_row();
            ui.label("Loudness (LUFS)");
            theme::segmented(
                ui,
                setter,
                &params.loudness,
                &[(Loudness::ApplePodcasts, "Podcast -19"), (Loudness::Streaming, "Stream -14"), (Loudness::Broadcast, "TV -23"), (Loudness::Off, "Off")],
            );
            ui.end_row();
            ui.add_enabled(mastering, egui::Label::new("True peak ceiling"));
            ui.add_enabled_ui(mastering, |ui| {
                ui.horizontal(|ui| {
                    theme::soap_slider(ui, setter, &params.ceiling, slider_width);
                    ui.label(params.ceiling.to_string());
                });
            });
            ui.end_row();
            ui.add_enabled(mastering, egui::Label::new("Max gain"));
            ui.add_enabled_ui(mastering, |ui| {
                ui.horizontal(|ui| {
                    theme::soap_slider(ui, setter, &params.max_gain, slider_width);
                    ui.label(params.max_gain.to_string());
                });
            });
            ui.end_row();
            ui.label("Keep stereo");
            ui.horizontal(|ui| {
                theme::toggle(ui, setter, &params.stereo);
                ui.add_space(28.0);
                ui.label("Clean on stop");
                theme::toggle(ui, setter, &params.auto_process);
            });
            ui.end_row();
        });
    });

    if let Some(take) = &take {
        ui.add_space(4.0);
        theme::card(ui, |ui| {
            if take.settings != params.render_settings() && !busy {
                ui.colored_label(HONEY, "Settings changed since the last wash: press Clean again to apply them.");
            }
            ui.horizontal_wrapped(|ui| {
                theme::stat_tile(ui, "Duration", &fmt_time(take.stats.duration_sec));
                theme::stat_tile(ui, "Input loudness", &take.stats.input_lufs.map_or("–".into(), |v| format!("{v:.1} LUFS")));
                theme::stat_tile(ui, "Output peak", &take.stats.true_peak_dbfs.map_or("–".into(), |v| format!("{v:.1} dBTP")));
                theme::stat_tile(ui, "Speed", &format!("{:.0}× realtime", take.stats.realtime_factor));
            });
            ui.horizontal(|ui| {
                let name = take.clean.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                ui.label(RichText::new(name).monospace().color(MUTED));
                if ui.link(RichText::new("Open folder").color(AQUA)).clicked() {
                    let _ = open::that_detached(takes_dir());
                }
            });
        });
    }

    ui.add_space(8.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("Voice enhancement by Clear from Desert Ant Labs · Source-Available License")
                .small()
                .color(MUTED),
        );
    });
}

fn fmt_time(seconds: f64) -> String {
    let s = seconds.max(0.0).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
