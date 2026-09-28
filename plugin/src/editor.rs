use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Stroke};
use nih_plug_egui::{create_egui_editor, widgets::ParamSlider};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::state::{takes_dir, Command, Monitor, Phase, Shared};
use crate::ClearVoiceParams;

const ACCENT: Color32 = Color32::from_rgb(79, 184, 156);
const ORIGINAL: Color32 = Color32::from_rgb(217, 160, 91);
const DANGER: Color32 = Color32::from_rgb(242, 135, 127);
const MUTED: Color32 = Color32::from_rgb(156, 153, 143);

pub fn create(params: Arc<ClearVoiceParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(12.0, 6.0);
            ctx.set_style(style);
        },
        move |ctx, setter, _| {
            // Worker progress arrives off the GUI thread; keep the view fresh.
            ctx.request_repaint_after(Duration::from_millis(200));
            egui::CentralPanel::default()
                .frame(egui::Frame::default().fill(Color32::from_rgb(22, 22, 20)).inner_margin(16.0))
                .show(ctx, |ui| draw(ui, setter, &params, &shared));
        },
    )
}

fn draw(ui: &mut egui::Ui, setter: &ParamSetter, params: &ClearVoiceParams, shared: &Shared) {
    let status = shared.status();
    let take = params.take.lock().unwrap().clone();
    let busy = matches!(status.phase, Phase::Busy(_));
    let armed = status.phase == Phase::Armed;

    ui.horizontal(|ui| {
        ui.label(RichText::new("Clear Voice").size(20.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (color, text) = match &status.phase {
                Phase::Empty => (MUTED, "Nothing captured".to_string()),
                Phase::Armed => (DANGER, format!("Capturing {}", fmt_time(status.captured_sec))),
                Phase::Captured => (ORIGINAL, format!("Captured {}", fmt_time(status.captured_sec))),
                Phase::Busy(what) => (ORIGINAL, what.clone()),
                Phase::Ready => (ACCENT, "Clean track ready".to_string()),
                Phase::Error(_) => (DANGER, "Error".to_string()),
            };
            if busy {
                ui.spinner();
            }
            ui.label(RichText::new(text).color(color));
        });
    });
    ui.label(RichText::new("Denoise, dereverb and loudness for voice, offline and on-device").color(MUTED));
    ui.separator();

    ui.horizontal(|ui| {
        if armed {
            if ui.add(big_button("Stop capture", DANGER)).clicked() {
                shared.send(Command::Disarm);
            }
        } else if ui.add_enabled(!busy, big_button("Capture", DANGER)).clicked() {
            shared.send(Command::Arm);
        }
        let can_process = !busy && !armed && (status.has_capture || take.is_some());
        let label = if take.is_some() && !status.has_capture { "Clean again" } else { "Clean" };
        if ui.add_enabled(can_process, big_button(label, ACCENT)).clicked() {
            shared.send(Command::Process);
        }
        let can_discard = !busy && (take.is_some() || status.has_capture);
        if ui.add_enabled(can_discard, egui::Button::new("Discard")).clicked() {
            shared.send(Command::Discard);
        }
    });

    let hint = match &status.phase {
        Phase::Empty => "Press Capture, then play the section to clean in your DAW. When you stop, the voice is cleaned and plays back in place of the original.",
        Phase::Armed => "Play the section to clean. Stop the transport (or press Stop capture) when you're done.",
        Phase::Captured => "Capture complete: press Clean to process it.",
        Phase::Busy(_) => "Processing runs in the background; the input passes through untouched.",
        Phase::Ready => "Play the project: the clean version replaces the original over the captured region. Switch A/B to compare.",
        Phase::Error(e) => e.as_str(),
    };
    let hint_color = if matches!(status.phase, Phase::Error(_)) { DANGER } else { MUTED };
    ui.label(RichText::new(hint).color(hint_color));
    if shared.overflow.load(Ordering::Relaxed) {
        ui.colored_label(DANGER, "Part of the capture was lost (system overloaded). Try capturing again.");
    }
    if shared.no_position.load(Ordering::Relaxed) {
        ui.colored_label(DANGER, "The host doesn't report a timeline position: capture isn't possible.");
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Listen:");
        let monitor = params.monitor.value();
        for (value, text, color) in [(Monitor::Original, "A · Original", ORIGINAL), (Monitor::Clean, "B · Clean", ACCENT)] {
            let selected = monitor == value;
            let label = RichText::new(text).color(if selected { color } else { MUTED });
            if ui.selectable_label(selected, label).clicked() && !selected {
                setter.begin_set_parameter(&params.monitor);
                setter.set_parameter(&params.monitor, value);
                setter.end_set_parameter(&params.monitor);
            }
        }
    });

    ui.separator();
    egui::Grid::new("settings").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label("Strength");
        ui.add(ParamSlider::for_param(&params.strength, setter).with_width(220.0));
        ui.end_row();
        ui.label("Loudness (LUFS)");
        ui.add(ParamSlider::for_param(&params.loudness, setter).with_width(220.0));
        ui.end_row();
        ui.label("True peak ceiling");
        ui.add(ParamSlider::for_param(&params.ceiling, setter).with_width(220.0));
        ui.end_row();
        ui.label("Max gain");
        ui.add(ParamSlider::for_param(&params.max_gain, setter).with_width(220.0));
        ui.end_row();
        ui.label("Keep stereo");
        ui.add(ParamSlider::for_param(&params.stereo, setter).with_width(220.0));
        ui.end_row();
        ui.label("Clean when capture stops");
        ui.add(ParamSlider::for_param(&params.auto_process, setter).with_width(220.0));
        ui.end_row();
    });

    if let Some(take) = &take {
        if take.settings != params.render_settings() && !busy {
            ui.colored_label(ORIGINAL, "Settings changed since the last render: press Clean again to apply them.");
        }
        ui.separator();
        egui::Grid::new("stats").num_columns(4).spacing([16.0, 4.0]).show(ui, |ui| {
            for label in ["Duration", "Input loudness", "Output true peak", "Speed"] {
                ui.label(RichText::new(label).color(MUTED).small());
            }
            ui.end_row();
            ui.label(fmt_time(take.stats.duration_sec));
            ui.label(take.stats.input_lufs.map_or("–".into(), |v| format!("{v:.1} LUFS")));
            ui.label(take.stats.true_peak_dbfs.map_or("–".into(), |v| format!("{v:.1} dBTP")));
            ui.label(format!("{:.1}× realtime", take.stats.realtime_factor));
            ui.end_row();
        });
        ui.horizontal(|ui| {
            let name = take.clean.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            ui.label(RichText::new(name).monospace().color(MUTED));
            if ui.button("Open folder").clicked() {
                let _ = open::that_detached(takes_dir());
            }
        });
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.label(
            RichText::new("Voice enhancement by Clear from Desert Ant Labs · Desert Ant Labs Source-Available License")
                .small()
                .color(MUTED),
        );
    });
}

fn big_button(text: &str, color: Color32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_string()).strong().color(color))
        .stroke(Stroke::new(1.0_f32, color))
        .min_size(egui::vec2(120.0, 32.0))
}

fn fmt_time(seconds: f64) -> String {
    let s = seconds.max(0.0).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
