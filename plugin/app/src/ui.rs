//! The window: open a recording, wash it, compare A/B, save the clean one.

use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke};
use serde::{Deserialize, Serialize};
use soap::state::Loudness;
use soap::theme::{self, AQUA, CORAL, HONEY, INK, MUTED, TILE};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::audio::{self, WavFormat};
use crate::jobs::{Cleaned, Event, Jobs, Settings, Source};
use crate::player::Player;

const OPEN_EXTENSIONS: &[&str] = &["wav", "wave", "aif", "aiff", "aifc", "flac", "mp3", "m4a", "mp4", "aac", "alac", "caf", "ogg", "oga", "mka", "mkv", "webm"];
const GREEN: Color32 = Color32::from_rgb(46, 170, 140);

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Prefs {
    format: WavFormat,
    last_dir: Option<PathBuf>,
}

impl Default for Prefs {
    fn default() -> Self {
        Self { format: WavFormat::Pcm16, last_dir: None }
    }
}

pub struct SoapApp {
    settings: Settings,
    prefs: Prefs,
    jobs: Jobs,
    player: Result<Player, String>,
    source: Option<Arc<Source>>,
    cleaned: Option<Arc<Cleaned>>,
    busy: Option<(String, Instant)>,
    error: Option<String>,
    notice: Option<String>,
    listen_clean: bool,
}

impl SoapApp {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        theme::install(&cc.egui_ctx);
        let player = Player::new();
        let ctx = cc.egui_ctx.clone();
        let jobs = Jobs::spawn(player.as_ref().ok().map(|p| p.rate), move || ctx.request_repaint());
        let storage = cc.storage;
        let mut app = Self {
            settings: storage.and_then(|s| eframe::get_value(s, "settings")).unwrap_or_default(),
            prefs: storage.and_then(|s| eframe::get_value(s, "prefs")).unwrap_or_default(),
            jobs,
            player,
            source: None,
            cleaned: None,
            busy: None,
            error: None,
            notice: None,
            listen_clean: false,
        };
        if let Some(path) = open {
            app.open(path);
        }
        app
    }

    fn open(&mut self, path: PathBuf) {
        if self.busy.is_some() {
            return;
        }
        self.prefs.last_dir = path.parent().map(PathBuf::from);
        self.error = None;
        self.notice = None;
        self.busy = Some(("Opening…".into(), Instant::now()));
        self.jobs.load(path);
    }

    fn pick_file(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Open a recording").add_filter("Audio", OPEN_EXTENSIONS);
        if let Some(dir) = &self.prefs.last_dir {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            self.open(path);
        }
    }

    fn clean(&mut self) {
        if let (Some(source), None) = (&self.source, &self.busy) {
            self.error = None;
            self.notice = None;
            self.busy = Some(("Washing…".into(), Instant::now()));
            self.jobs.clean(source.clone(), self.settings.clone());
        }
    }

    fn save(&mut self) {
        let (Some(source), Some(cleaned)) = (&self.source, &self.cleaned) else { return };
        let stem = source.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "voice".into());
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save the clean version")
            .add_filter("WAV", &["wav"])
            .set_file_name(format!("{stem}-clean.wav"));
        if let Some(dir) = source.path.parent() {
            dialog = dialog.set_directory(dir);
        }
        let Some(mut path) = dialog.save_file() else { return };
        if path.extension().is_none() {
            path.set_extension("wav");
        }
        match audio::write_wav(&path, &cleaned.channels, cleaned.rate, self.prefs.format) {
            Ok(()) => {
                self.error = None;
                self.notice = Some(format!("Saved {}", path.file_name().unwrap_or_default().to_string_lossy()));
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn set_listen(&mut self, clean: bool) {
        self.listen_clean = clean && self.cleaned.is_some();
        if let Ok(player) = &self.player {
            player.set_clean(self.listen_clean);
        }
    }

    fn poll(&mut self) {
        while let Some(event) = self.jobs.poll() {
            match event {
                Event::Busy(step) => {
                    if let Some((message, _)) = &mut self.busy {
                        *message = step;
                    }
                }
                Event::Loaded(result) => {
                    self.busy = None;
                    match result {
                        Ok(source) => {
                            if let Ok(player) = &self.player {
                                player.stop();
                                player.set_tracks(Some(source.view.playback.clone()), None);
                            }
                            self.source = Some(source);
                            self.cleaned = None;
                            self.set_listen(false);
                            // Opening a file is asking to wash it.
                            self.clean();
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                Event::Cleaned(result) => {
                    self.busy = None;
                    match result {
                        Ok(cleaned) => {
                            if let (Ok(player), Some(source)) = (&self.player, &self.source) {
                                player.set_tracks(Some(source.view.playback.clone()), Some(cleaned.view.playback.clone()));
                            }
                            self.cleaned = Some(cleaned);
                            self.set_listen(true);
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
            }
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        // A focused button takes Space itself: leave it the key then.
        let free = ctx.memory(|m| m.focused().is_none());
        let (space, a, b, dropped) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Space),
                i.key_pressed(egui::Key::A),
                i.key_pressed(egui::Key::B),
                i.raw.dropped_files.iter().find_map(|f| f.path.clone()),
            )
        });
        if let Some(path) = dropped {
            self.open(path);
        }
        if let (Ok(player), true, Some(_)) = (&self.player, space && free, &self.source) {
            player.toggle();
        }
        if a {
            self.set_listen(false);
        }
        if b {
            self.set_listen(true);
        }
    }

    fn status(&self) -> (String, Color32) {
        if let Some((message, _)) = &self.busy {
            (message.clone(), HONEY)
        } else if self.error.is_some() {
            ("Something went wrong".into(), CORAL)
        } else if self.cleaned.is_some() {
            ("Squeaky clean".into(), GREEN)
        } else if self.source.is_some() {
            ("Ready to wash".into(), MUTED)
        } else {
            ("Ready to lather".into(), MUTED)
        }
    }
}

impl eframe::App for SoapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        self.shortcuts(ctx);
        let playing = self.player.as_ref().is_ok_and(Player::is_playing);
        if self.busy.is_some() || playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        let time = ctx.input(|i| i.time) as f32;
        let hovering_file = ctx.input(|i| !i.raw.hovered_files.is_empty());

        egui::CentralPanel::default().frame(egui::Frame::default()).show(ctx, |ui| {
            let foam = if self.busy.is_some() { 1.0 } else if hovering_file { 0.4 } else { 0.0 };
            theme::paint_background(ui.painter(), ui.max_rect(), if foam > 0.0 { time } else { 0.0 }, foam);
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Frame::default().inner_margin(20.0).show(ui, |ui| self.draw(ui, time, hovering_file));
            });
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "settings", &self.settings);
        eframe::set_value(storage, "prefs", &self.prefs);
    }
}

impl SoapApp {
    fn draw(&mut self, ui: &mut egui::Ui, time: f32, hovering_file: bool) {
        let busy = self.busy.is_some();
        let pulse = if busy { (time * 4.0).sin() * 0.5 + 0.5 } else { 1.0 };

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
                let (text, color) = self.status();
                theme::status_pill(ui, &text, color, pulse);
            });
        });
        ui.add_space(6.0);

        theme::card(ui, |ui| match self.source.clone() {
            None => self.draw_drop_zone(ui, hovering_file),
            Some(source) => self.draw_source(ui, &source),
        });
        ui.add_space(4.0);
        theme::card(ui, |ui| self.draw_settings(ui));

        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Voice enhancement by Clear from Desert Ant Labs · Source-Available License").small().color(MUTED));
        });
    }

    fn draw_drop_zone(&mut self, ui: &mut egui::Ui, hovering_file: bool) {
        let busy = self.busy.is_some();
        ui.vertical_centered(|ui| {
            ui.add_space(18.0);
            let title = if hovering_file { "Let go to wash it" } else { "Drop a voice recording here" };
            ui.label(RichText::new(title).size(20.0).family(egui::FontFamily::Name(theme::HEADING.into())));
            ui.label(RichText::new("WAV, AIFF, FLAC, MP3, M4A/AAC, ALAC, Ogg Vorbis").color(MUTED));
            ui.add_space(8.0);
            if theme::pill_button(ui, "Open audio…", AQUA, !busy).clicked() {
                self.pick_file();
            }
            if busy {
                ui.add(egui::Spinner::new().color(AQUA));
            }
            self.draw_messages(ui);
            ui.add_space(12.0);
        });
    }

    fn draw_source(&mut self, ui: &mut egui::Ui, source: &Arc<Source>) {
        let busy = self.busy.is_some();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(&source.name).size(16.0).family(egui::FontFamily::Name(theme::HEADING.into())));
                let layout = match source.file_channels {
                    1 => "mono".to_string(),
                    2 => "stereo".to_string(),
                    n => format!("{n} channels, using the first two"),
                };
                ui.label(RichText::new(format!("{} · {} · {layout}", fmt_time(source.duration), fmt_rate(source.rate))).color(MUTED));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::ghost_button(ui, "Open another…", !busy).clicked() {
                    self.pick_file();
                }
            });
        });

        ui.horizontal(|ui| {
            let label = if self.cleaned.is_some() { "Clean again" } else { "Clean" };
            if theme::pill_button(ui, label, AQUA, !busy).clicked() {
                self.clean();
            }
            let playing = self.player.as_ref().is_ok_and(Player::is_playing);
            let play = theme::pill_button(ui, if playing { "     Pause" } else { "     Play" }, CORAL, self.player.is_ok());
            paint_transport_icon(ui, play.rect, playing);
            if play.clicked() {
                if let Ok(player) = &self.player {
                    player.toggle();
                }
            }
            if theme::ghost_button(ui, "Save WAV…", self.cleaned.is_some() && !busy).clicked() {
                self.save();
            }
            if busy {
                ui.add(egui::Spinner::new().color(AQUA));
                if let Some((_, since)) = &self.busy {
                    ui.label(RichText::new(format!("{:.0} s", since.elapsed().as_secs_f32())).color(MUTED));
                }
            }
        });

        let position = self.player.as_ref().map_or(0.0, Player::position_sec);
        let progress = (source.duration > 0.0).then(|| (position / source.duration).clamp(0.0, 1.0) as f32);
        // One scale for both, so a louder clean version looks louder, but
        // quiet recordings still fill the strip.
        let loudest = [Some(&source.view.peaks), self.cleaned.as_ref().map(|c| &c.view.peaks)]
            .into_iter()
            .flatten()
            .flatten()
            .fold(0.0f32, |m, [lo, hi]| m.max(-lo).max(*hi));
        let scale = (0.95 / loudest.max(1e-6)).clamp(1.0, 12.0);
        let mut seek = None;
        let mut listen = None;
        for (clean, label, peaks) in [
            (false, "A · Original", Some(&source.view.peaks)),
            (true, "B · Clean", self.cleaned.as_ref().map(|c| &c.view.peaks)),
        ] {
            let active = self.listen_clean == clean;
            let response = waveform(ui, label, peaks.map(Vec::as_slice), scale, progress.filter(|_| active), active);
            if let Some(fraction) = response {
                seek = Some(fraction as f64 * source.duration);
                listen = Some(clean);
            }
        }
        if let Some(clean) = listen {
            self.set_listen(clean);
        }
        if let (Some(seconds), Ok(player)) = (seek, &self.player) {
            player.seek(seconds);
        }

        ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 32.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new("Listen").color(MUTED));
            ui.add_enabled_ui(self.cleaned.is_some(), |ui| {
                if let Some(clean) = theme::choice(ui, self.listen_clean, &[(false, "A · Original"), (true, "B · Clean")]) {
                    self.set_listen(clean);
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{} / {}", fmt_time(position), fmt_time(source.duration))).monospace().color(MUTED));
            });
        });
        if let Some(cleaned) = &self.cleaned {
            if cleaned.settings != self.settings && !busy {
                ui.colored_label(HONEY, "Settings changed since the last wash: press Clean again to apply them.");
            }
            ui.horizontal_wrapped(|ui| {
                theme::stat_tile(ui, "Input loudness", &cleaned.input_lufs.map_or("–".into(), |v| format!("{v:.1} LUFS")));
                theme::stat_tile(ui, "Output peak", &cleaned.true_peak_dbfs.map_or("–".into(), |v| format!("{v:.1} dBTP")));
                theme::stat_tile(ui, "Speed", &format!("{:.0}× realtime", cleaned.realtime_factor));
            });
        }
        if let Err(e) = &self.player {
            ui.label(RichText::new(format!("Playback unavailable: {e}")).small().color(MUTED));
        }
        self.draw_messages(ui);
    }

    fn draw_messages(&self, ui: &mut egui::Ui) {
        if let Some(error) = &self.error {
            ui.colored_label(CORAL, error);
        } else if let Some(notice) = &self.notice {
            ui.colored_label(GREEN, notice);
        } else if self.busy.as_ref().is_some_and(|(m, _)| m.starts_with("Washing")) {
            ui.label(RichText::new("Scrubbing on your computer: the audio never leaves it.").color(MUTED));
        } else if self.cleaned.is_some() {
            ui.label(RichText::new("Space plays and pauses, A and B switch versions, click a waveform to jump.").color(MUTED));
        }
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        let width = (ui.available_width() - 150.0 - 96.0).max(140.0);
        let s = &mut self.settings;
        let mastering = s.loudness != Loudness::Off;
        egui::Grid::new("settings").num_columns(2).min_col_width(140.0).spacing([10.0, 12.0]).show(ui, |ui| {
            ui.label("Strength");
            ui.horizontal(|ui| {
                value_slider(ui, &mut s.strength, 0.0..=1.0, 1.0, 0.01, width);
                ui.label(format!("{:.0}%", s.strength * 100.0));
            });
            ui.end_row();
            ui.label("Loudness (LUFS)");
            if let Some(loudness) = theme::choice(
                ui,
                s.loudness,
                &[(Loudness::ApplePodcasts, "Podcast -19"), (Loudness::Streaming, "Stream -14"), (Loudness::Broadcast, "TV -23"), (Loudness::Off, "Off")],
            ) {
                s.loudness = loudness;
            }
            ui.end_row();
            ui.add_enabled(mastering, egui::Label::new("True peak ceiling"));
            ui.add_enabled_ui(mastering, |ui| {
                ui.horizontal(|ui| {
                    value_slider(ui, &mut s.ceiling_db, -12.0..=0.0, -1.5, 0.1, width);
                    ui.label(format!("{:.1} dBTP", s.ceiling_db));
                });
            });
            ui.end_row();
            ui.add_enabled(mastering, egui::Label::new("Max gain"));
            ui.add_enabled_ui(mastering, |ui| {
                ui.horizontal(|ui| {
                    value_slider(ui, &mut s.max_gain_db, 0.0..=30.0, 9.0, 0.5, width);
                    ui.label(format!("{:.1} dB", s.max_gain_db));
                });
            });
            ui.end_row();
            ui.label("Keep stereo");
            if theme::switch(ui, s.stereo).clicked() {
                s.stereo = !s.stereo;
            }
            ui.end_row();
            ui.label("Output");
            ui.horizontal(|ui| {
                if let Some(rate) = theme::choice(ui, s.output_rate, &[(48_000, "48 kHz"), (44_100, "44.1 kHz")]) {
                    s.output_rate = rate;
                }
                ui.add_space(8.0);
                if let Some(format) = theme::choice(ui, self.prefs.format, &[(WavFormat::Pcm16, "16-bit"), (WavFormat::Float32, "32-bit float")]) {
                    self.prefs.format = format;
                }
            });
            ui.end_row();
        });
    }
}

/// A theme slider over a plain value; double-click resets it.
fn value_slider(ui: &mut egui::Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>, default: f32, step: f32, width: f32) {
    let (lo, hi) = (*range.start(), *range.end());
    let (response, changed) = theme::slider(ui, (*value - lo) / (hi - lo), width);
    if response.double_clicked() {
        *value = default;
    } else if let Some(normalized) = changed {
        *value = ((lo + normalized * (hi - lo)) / step).round() * step;
    }
}

/// One version's overview with the playhead; returns where it was clicked (0..1).
fn waveform(ui: &mut egui::Ui, label: &str, peaks: Option<&[[f32; 2]]>, scale: f32, progress: Option<f32>, active: bool) -> Option<f32> {
    ui.label(RichText::new(label).small().color(if active { INK } else { MUTED }));
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 58.0), Sense::click_and_drag());
    let painter = ui.painter();
    painter.rect_filled(rect, 12.0, TILE);
    if active {
        painter.rect_stroke(rect, 12.0, Stroke::new(1.5_f32, theme::with_alpha(AQUA, 140)), egui::StrokeKind::Inside);
    }
    let Some(peaks) = peaks.filter(|p| !p.is_empty()) else {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "Not washed yet", egui::FontId::proportional(13.0), MUTED);
        return None;
    };
    let inner = rect.shrink2(egui::vec2(10.0, 6.0));
    let color = if active { AQUA } else { theme::with_alpha(MUTED, 150) };
    let columns = inner.width().max(1.0) as usize;
    for x in 0..columns {
        let (from, to) = (x * peaks.len() / columns, ((x + 1) * peaks.len() / columns).max(x * peaks.len() / columns + 1).min(peaks.len()));
        let [lo, hi] = peaks[from..to].iter().fold([0.0f32, 0.0f32], |[lo, hi], p| [lo.min(p[0]), hi.max(p[1])]);
        let px = inner.left() + x as f32 + 0.5;
        let half = inner.height() / 2.0;
        let (top, bottom) = (inner.center().y - (hi * scale).min(1.0) * half, inner.center().y - (lo * scale).max(-1.0) * half);
        painter.line_segment([egui::pos2(px, top.min(inner.center().y - 0.5)), egui::pos2(px, bottom.max(inner.center().y + 0.5))], Stroke::new(1.0_f32, color));
    }
    if let Some(progress) = progress {
        let x = inner.left() + inner.width() * progress;
        painter.line_segment([egui::pos2(x, rect.top() + 3.0), egui::pos2(x, rect.bottom() - 3.0)], Stroke::new(2.0_f32, CORAL));
    }
    let clicked = response.clicked() || response.dragged();
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked
        .then(|| response.interact_pointer_pos())
        .flatten()
        .map(|pos| ((pos.x - inner.left()) / inner.width()).clamp(0.0, 1.0))
}

/// Play triangle or pause bars on the left of the Play button.
fn paint_transport_icon(ui: &egui::Ui, button: egui::Rect, playing: bool) {
    let c = button.left_center() + egui::vec2(24.0, 0.0);
    let painter = ui.painter();
    if playing {
        for dx in [-3.5, 3.5] {
            painter.rect_filled(egui::Rect::from_center_size(c + egui::vec2(dx, 0.0), egui::vec2(4.0, 13.0)), 1.5, Color32::WHITE);
        }
    } else {
        let points = vec![c + egui::vec2(-4.5, -7.0), c + egui::vec2(7.5, 0.0), c + egui::vec2(-4.5, 7.0)];
        painter.add(egui::Shape::convex_polygon(points, Color32::WHITE, Stroke::NONE));
    }
}

fn fmt_time(seconds: f64) -> String {
    let s = seconds.max(0.0).floor() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

fn fmt_rate(rate: f64) -> String {
    if rate % 1000.0 == 0.0 {
        format!("{:.0} kHz", rate / 1000.0)
    } else {
        format!("{:.1} kHz", rate / 1000.0)
    }
}
