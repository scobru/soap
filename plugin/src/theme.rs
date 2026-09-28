//! Soap's look: pastel palette, Nunito, the painted logo, bubbles and the
//! soap-bar widgets the editor is built from.

use nih_plug::prelude::{Enum, EnumParam, Param, ParamSetter};
use nih_plug_egui::egui::{
    self, epaint::Shadow, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Mesh,
    Painter, Pos2, Rect, Response, RichText, Sense, Stroke, TextStyle, Ui, Vec2,
};
use std::sync::Arc;

pub const INK: Color32 = Color32::from_rgb(30, 74, 87);
pub const MUTED: Color32 = Color32::from_rgb(106, 139, 148);
pub const AQUA: Color32 = Color32::from_rgb(76, 184, 206);
pub const MINT: Color32 = Color32::from_rgb(127, 220, 196);
pub const CORAL: Color32 = Color32::from_rgb(255, 128, 136);
pub const LAVENDER: Color32 = Color32::from_rgb(156, 140, 240);
pub const PINK: Color32 = Color32::from_rgb(255, 159, 203);
pub const HONEY: Color32 = Color32::from_rgb(232, 164, 78);
pub const TRACK: Color32 = Color32::from_rgb(218, 238, 241);
pub const TILE: Color32 = Color32::from_rgb(236, 248, 247);
const BG_TOP: Color32 = Color32::from_rgb(242, 251, 248);
const BG_BOTTOM: Color32 = Color32::from_rgb(218, 238, 243);
const BAR_LIGHT: Color32 = Color32::from_rgb(166, 240, 220);
const BAR_DARK: Color32 = Color32::from_rgb(74, 174, 216);

pub const HEADING: &str = "nunito-black";

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "nunito".into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Nunito-600.ttf"))),
    );
    fonts.font_data.insert(
        HEADING.into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Nunito-800.ttf"))),
    );
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "nunito".into());
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    fonts.families.insert(
        FontFamily::Name(HEADING.into()),
        std::iter::once(HEADING.to_string()).chain(fallback).collect(),
    );
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::light();
    let v = &mut style.visuals;
    v.override_text_color = Some(INK);
    v.panel_fill = BG_TOP;
    v.selection.bg_fill = AQUA;
    v.hyperlink_color = AQUA;
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.noninteractive] {
        w.corner_radius = CornerRadius::same(14);
    }
    v.widgets.inactive.weak_bg_fill = TRACK;
    v.widgets.inactive.bg_fill = TRACK;
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(204, 232, 236);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, AQUA);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(190, 226, 232);
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 7.0);
    style.text_styles = [
        (TextStyle::Small, FontId::proportional(11.5)),
        (TextStyle::Body, FontId::proportional(14.0)),
        (TextStyle::Button, FontId::proportional(14.5)),
        (TextStyle::Monospace, FontId::monospace(12.5)),
        (TextStyle::Heading, FontId::new(28.0, FontFamily::Name(HEADING.into()))),
    ]
    .into();
    ctx.set_style(style);
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()), mix(a.a(), b.a()))
}

pub fn with_alpha(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// Deterministic pseudo-random in 0..1, so bubbles keep their places between frames.
fn hash01(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9) ^ salt.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x % 10_000) as f32 / 10_000.0
}

/// Vertical pastel gradient with drifting bubbles. `foam` (0..1) makes more of
/// them rise, faster: the "busy" animation.
pub fn paint_background(painter: &Painter, rect: Rect, time: f32, foam: f32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), BG_TOP);
    mesh.colored_vertex(rect.right_top(), BG_TOP);
    mesh.colored_vertex(rect.right_bottom(), BG_BOTTOM);
    mesh.colored_vertex(rect.left_bottom(), BG_BOTTOM);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));

    let count = 14 + (foam * 22.0) as u32;
    for i in 0..count {
        let r = 5.0 + hash01(i, 1) * 22.0;
        let speed = (8.0 + hash01(i, 2) * 14.0) * (0.15 + foam * 3.0);
        let span = rect.height() + 2.0 * r;
        let y = rect.bottom() + r - (hash01(i, 3) * span + time * speed).rem_euclid(span);
        let x = rect.left() + hash01(i, 4) * rect.width() + (time * 0.6 + i as f32).sin() * 6.0;
        if foam == 0.0 && y < rect.top() + 110.0 {
            continue;
        }
        paint_bubble(painter, Pos2::new(x, y), r, 0.35 + 0.35 * foam);
    }
}

pub fn paint_bubble(painter: &Painter, center: Pos2, r: f32, opacity: f32) {
    let a = |x: f32| (x * opacity * 255.0).clamp(0.0, 255.0) as u8;
    painter.circle_filled(center, r, Color32::from_rgba_unmultiplied(255, 255, 255, a(0.3)));
    painter.circle_stroke(center, r * 0.86, Stroke::new(r * 0.16, with_alpha(PINK, a(0.3))));
    painter.circle_stroke(center, r, Stroke::new(1.2_f32, with_alpha(LAVENDER, a(0.85))));
    painter.circle_filled(center + Vec2::new(-r * 0.36, -r * 0.4), r * 0.2, Color32::from_rgba_unmultiplied(255, 255, 255, a(0.95)));
}

/// Points of a rounded rectangle (a convex outline), for meshes that rotate.
fn rounded_rect_points(rect: Rect, radius: f32) -> Vec<Pos2> {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let corners = [
        (rect.right_bottom() + Vec2::new(-r, -r), 0.0),
        (rect.left_bottom() + Vec2::new(r, -r), 90.0),
        (rect.left_top() + Vec2::new(r, r), 180.0),
        (rect.right_top() + Vec2::new(-r, r), 270.0),
    ];
    let mut points = Vec::with_capacity(40);
    for (c, start) in corners {
        for step in 0..=8 {
            let angle = (start + step as f32 * 90.0 / 8.0f32).to_radians();
            points.push(c + Vec2::new(angle.cos(), angle.sin()) * r);
        }
    }
    points
}

fn rotate(p: Pos2, pivot: Pos2, angle: f32) -> Pos2 {
    let (s, c) = angle.sin_cos();
    let d = p - pivot;
    pivot + Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c)
}

/// Fill a convex outline with a colour per vertex.
fn fill_convex(painter: &Painter, points: &[Pos2], color: impl Fn(Pos2) -> Color32) {
    let mut mesh = Mesh::default();
    let center = points.iter().fold(Vec2::ZERO, |acc, p| acc + p.to_vec2()) / points.len() as f32;
    mesh.colored_vertex(center.to_pos2(), color(center.to_pos2()));
    for &p in points {
        mesh.colored_vertex(p, color(p));
    }
    let n = points.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// The Soap logo (assets/logo.svg), painted so it stays crisp at any scale.
pub fn paint_logo(painter: &Painter, rect: Rect) {
    let s = rect.width() / 128.0;
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * s;
    let pivot = at(62.0, 82.0);
    let tilt = (-9.0f32).to_radians();
    let tilted = |points: Vec<Pos2>| points.into_iter().map(|p| rotate(p, pivot, tilt)).collect::<Vec<_>>();

    let bar = Rect::from_min_size(at(12.0, 56.0), Vec2::new(98.0, 54.0) * s);
    let shadow = tilted(rounded_rect_points(bar.translate(Vec2::new(0.0, 5.0 * s)), 24.0 * s));
    fill_convex(painter, &shadow, |_| Color32::from_rgba_unmultiplied(47, 143, 181, 60));
    let (tl, br) = (bar.left_top(), bar.right_bottom());
    let span = (br - tl).length_sq();
    fill_convex(painter, &tilted(rounded_rect_points(bar, 24.0 * s)), |p| {
        let q = rotate(p, pivot, -tilt);
        lerp_color(BAR_LIGHT, BAR_DARK, (q - tl).dot(br - tl) / span)
    });
    let shine = Rect::from_min_size(at(20.0, 60.0), Vec2::new(82.0, 20.0) * s);
    fill_convex(painter, &tilted(rounded_rect_points(shine, 10.0 * s)), |p| {
        let q = rotate(p, pivot, -tilt);
        let t = (q.y - shine.top()) / shine.height();
        Color32::from_rgba_unmultiplied(255, 255, 255, ((1.0 - t) * 190.0) as u8)
    });

    let stroke = Stroke::new(5.0 * s, Color32::from_rgba_unmultiplied(255, 255, 255, 235));
    for (x, top, bottom) in [(32.0, 72.0, 96.0), (42.0, 66.0, 102.0), (52.0, 74.0, 94.0), (62.0, 70.0, 98.0), (72.0, 77.0, 91.0), (82.0, 80.0, 88.0), (92.0, 83.0, 85.0)] {
        let a = rotate(at(x, top), pivot, tilt);
        let b = rotate(at(x, bottom), pivot, tilt);
        painter.line_segment([a, b], stroke);
        painter.circle_filled(a, stroke.width / 2.0, stroke.color);
        painter.circle_filled(b, stroke.width / 2.0, stroke.color);
    }
    for (x, y, r) in [(94.0, 34.0, 17.0), (62.0, 30.0, 9.0), (113.0, 58.0, 7.0)] {
        paint_bubble(painter, at(x, y), r * s, 1.0);
    }
}

/// A frosted card that groups a section.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::default()
        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 215))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(210, 236, 238)))
        .corner_radius(18.0)
        .inner_margin(16.0)
        .shadow(Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_rgba_unmultiplied(40, 120, 140, 26) })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub fn pill_button(ui: &mut Ui, text: &str, fill: Color32, enabled: bool) -> Response {
    let button = egui::Button::new(RichText::new(text).color(Color32::WHITE).size(15.5).family(FontFamily::Name(HEADING.into())))
        .fill(fill)
        .stroke(Stroke::NONE)
        .corner_radius(20.0)
        .min_size(egui::vec2(128.0, 40.0));
    ui.add_enabled(enabled, button).on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn ghost_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let button = egui::Button::new(RichText::new(text).color(MUTED))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(196, 224, 229)))
        .corner_radius(20.0)
        .min_size(egui::vec2(0.0, 40.0));
    ui.add_enabled(enabled, button)
}

/// A soap-bar slider over a normalized (0..1) value: rounded track, aqua
/// fill, a bubble for the knob. Returns the new value while it is dragged or
/// clicked; double-click is left to the caller (reset).
pub fn slider(ui: &mut Ui, normalized: f32, width: f32) -> (Response, Option<f32>) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 26.0), Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), egui::vec2(width - 24.0, 10.0));
    let changed = (response.dragged() || (response.clicked() && !response.double_clicked()))
        .then(|| response.interact_pointer_pos())
        .flatten()
        .map(|pos| ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0));
    let value = changed.unwrap_or(normalized).clamp(0.0, 1.0);

    let painter = ui.painter();
    painter.rect_filled(track, 5.0, TRACK);
    let mut filled = track;
    filled.set_width(track.width() * value);
    if value > 0.0 {
        painter.rect_filled(filled, 5.0, if response.dragged() { MINT } else { AQUA });
    }
    let knob = Pos2::new(track.left() + track.width() * value, track.center().y);
    let r = if response.hovered() || response.dragged() { 12.0 } else { 11.0 };
    painter.circle_filled(knob + Vec2::new(0.0, 1.5), r, Color32::from_rgba_unmultiplied(40, 120, 140, 40));
    painter.circle_filled(knob, r, Color32::WHITE);
    painter.circle_stroke(knob, r * 0.8, Stroke::new(2.0_f32, with_alpha(PINK, 70)));
    painter.circle_stroke(knob, r, Stroke::new(1.5_f32, AQUA));
    painter.circle_filled(knob + Vec2::new(-r * 0.35, -r * 0.35), r * 0.22, with_alpha(LAVENDER, 90));
    (response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal), changed)
}

/// [`slider`] bound to a plugin parameter, with host automation gestures.
/// Double-click resets it.
pub fn soap_slider<P: Param>(ui: &mut Ui, setter: &ParamSetter, param: &P, width: f32) -> Response {
    let (response, changed) = slider(ui, param.unmodulated_normalized_value(), width);
    if response.double_clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, param.default_plain_value());
        setter.end_set_parameter(param);
    } else if response.drag_started() || (response.clicked() && changed.is_some()) {
        setter.begin_set_parameter(param);
    }
    if let Some(value) = changed {
        setter.set_parameter_normalized(param, value);
    }
    if response.drag_stopped() || (response.clicked() && changed.is_some()) {
        setter.end_set_parameter(param);
    }
    response
}

/// Pills for a set of choices; returns the one clicked, if it changed.
pub fn choice<T: PartialEq + Copy>(ui: &mut Ui, current: T, choices: &[(T, &str)]) -> Option<T> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for &(value, label) in choices {
            let selected = current == value;
            let text = RichText::new(label).color(if selected { Color32::WHITE } else { INK });
            let button = egui::Button::new(text)
                .fill(if selected { AQUA } else { TRACK })
                .stroke(Stroke::NONE)
                .corner_radius(14.0);
            if ui.add(button).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !selected {
                picked = Some(value);
            }
        }
    });
    picked
}

/// [`choice`] over an enum parameter.
pub fn segmented<T: Enum + PartialEq + Copy + Send + Sync + 'static>(ui: &mut Ui, setter: &ParamSetter, param: &EnumParam<T>, choices: &[(T, &str)]) {
    if let Some(value) = choice(ui, param.value(), choices) {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
}

/// A switch whose knob is a little bubble; flip the value when it's clicked.
pub fn switch(ui: &mut Ui, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 26.0), Sense::click());
    let t = ui.ctx().animate_bool(response.id, on != response.clicked());
    let painter = ui.painter();
    painter.rect_filled(rect, 13.0, lerp_color(TRACK, AQUA, t));
    let knob = Pos2::new(egui::lerp(rect.left() + 13.0..=rect.right() - 13.0, t), rect.center().y);
    painter.circle_filled(knob, 10.0, Color32::WHITE);
    painter.circle_stroke(knob, 8.0, Stroke::new(1.5_f32, with_alpha(PINK, 80)));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// [`switch`] bound to a boolean parameter.
pub fn toggle(ui: &mut Ui, setter: &ParamSetter, param: &nih_plug::prelude::BoolParam) -> Response {
    let on = param.value();
    let response = switch(ui, on);
    if response.clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, !on);
        setter.end_set_parameter(param);
    }
    response
}

/// A small rounded tile for one statistic.
pub fn stat_tile(ui: &mut Ui, label: &str, value: &str) {
    egui::Frame::default().fill(TILE).corner_radius(14.0).inner_margin(egui::vec2(12.0, 8.0)).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(label).small().color(MUTED));
            ui.label(RichText::new(value).size(15.0).family(FontFamily::Name(HEADING.into())));
        });
    });
}

pub fn status_color(phase: &crate::state::Phase) -> Color32 {
    use crate::state::Phase;
    match phase {
        Phase::Empty => MUTED,
        Phase::Armed | Phase::Error(_) => CORAL,
        Phase::Captured | Phase::Busy(_) => HONEY,
        Phase::Ready => Color32::from_rgb(46, 170, 140),
    }
}

pub fn status_pill(ui: &mut Ui, text: &str, color: Color32, pulse: f32) {
    egui::Frame::default()
        .fill(with_alpha(color, 36))
        .corner_radius(14.0)
        .inner_margin(egui::vec2(12.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 7.0;
                let (dot, _) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.5, with_alpha(color, (150.0 + 105.0 * pulse) as u8));
                ui.label(RichText::new(text).color(lerp_color(color, INK, 0.35)));
            });
        });
}
