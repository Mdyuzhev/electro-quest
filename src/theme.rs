//! Визуальная тема «ночной учебный стенд»: тёмный корпус прибора,
//! монтажная плата и светящиеся акценты тока.

use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId,
    Frame, Label, Layout, Margin, Painter, Pos2, Rect, RichText, Sense, Shadow, Stroke, StrokeKind,
    TextStyle, Vec2, vec2,
};

pub const BG: Color32 = Color32::from_rgb(8, 11, 16);
pub const CHASSIS: Color32 = Color32::from_rgb(16, 22, 30);
pub const PANEL: Color32 = Color32::from_rgb(20, 29, 40);
pub const PANEL_2: Color32 = Color32::from_rgb(30, 42, 56);
pub const INSET: Color32 = Color32::from_rgb(8, 14, 20);
pub const LINE: Color32 = Color32::from_rgb(58, 78, 98);
pub const TEXT: Color32 = Color32::from_rgb(232, 238, 244);
pub const TEXT_DIM: Color32 = Color32::from_rgb(168, 186, 204);
pub const MUTED: Color32 = Color32::from_rgb(122, 144, 164);
pub const INK: Color32 = Color32::from_rgb(12, 18, 24);
pub const AMBER: Color32 = Color32::from_rgb(245, 186, 66);
pub const CYAN: Color32 = Color32::from_rgb(72, 214, 236);
pub const MINT: Color32 = Color32::from_rgb(62, 214, 154);
pub const CORAL: Color32 = Color32::from_rgb(255, 112, 108);
pub const COPPER: Color32 = Color32::from_rgb(224, 146, 72);
pub const PHASE: Color32 = Color32::from_rgb(232, 150, 74);
pub const NEUTRAL: Color32 = Color32::from_rgb(86, 164, 255);
pub const PROTECTIVE: Color32 = Color32::from_rgb(78, 204, 126);
pub const VIOLET: Color32 = Color32::from_rgb(176, 146, 246);

pub const PCB: Color32 = Color32::from_rgb(10, 26, 24);
pub const PCB_DOT: Color32 = Color32::from_rgb(32, 72, 64);
pub const PCB_DOT_MAJOR: Color32 = Color32::from_rgb(58, 120, 104);

pub const SAFETY_LINE: &str =
    "Учебная симуляция. Не используйте её как руководство для работы с реальной сетью 230 В.";

pub fn level_color(index: usize) -> Color32 {
    match index {
        0 => AMBER,
        1 => Color32::from_rgb(255, 138, 102),
        2 => CYAN,
        3 => VIOLET,
        _ => MINT,
    }
}

pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "noto_sans".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSans.ttf"
        ))),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "noto_sans".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "noto_sans".to_owned());
    ctx.set_fonts(fonts);

    let mut style = (*ctx.global_style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = PANEL;
    style.visuals.window_stroke = Stroke::new(1.0, LINE);
    style.visuals.window_corner_radius = CornerRadius::same(16);
    style.visuals.window_shadow = drop_shadow();
    style.visuals.popup_shadow = drop_shadow();
    style.visuals.menu_corner_radius = CornerRadius::same(10);
    style.visuals.extreme_bg_color = INSET;
    style.visuals.faint_bg_color = PANEL;
    style.visuals.code_bg_color = INSET;
    style.visuals.hyperlink_color = CYAN;
    style.visuals.warn_fg_color = AMBER;
    style.visuals.error_fg_color = CORAL;
    style.visuals.selection.bg_fill = Color32::from_rgb(28, 78, 96);
    style.visuals.selection.stroke = Stroke::new(1.0, CYAN);
    style.visuals.widgets.noninteractive = widget_look(PANEL, LINE, MUTED, 10);
    style.visuals.widgets.inactive = widget_look(PANEL_2, LINE, TEXT, 10);
    style.visuals.widgets.hovered = widget_look(Color32::from_rgb(40, 58, 76), CYAN, TEXT, 10);
    style.visuals.widgets.active = widget_look(Color32::from_rgb(28, 78, 92), CYAN, INK, 10);
    style.visuals.widgets.open = style.visuals.widgets.hovered;
    style.spacing.item_spacing = vec2(8.0, 8.0);
    style.spacing.button_padding = vec2(12.0, 7.0);
    style.spacing.interact_size = vec2(40.0, 34.0);
    style.spacing.window_margin = Margin::same(12);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.scroll.floating = false;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.handle_min_length = 24.0;
    style.spacing.scroll.bar_inner_margin = 3.0;
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(12.0));
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(15.0));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(14.0));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(22.0));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::proportional(14.0));
    ctx.set_global_style(style);
}

fn widget_look(
    fill: Color32,
    stroke: Color32,
    text: Color32,
    radius: u8,
) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::new(1.0, stroke),
        corner_radius: CornerRadius::same(radius),
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    }
}

pub fn drop_shadow() -> Shadow {
    Shadow {
        offset: [0, 10],
        blur: 22,
        spread: 0,
        color: Color32::from_black_alpha(120),
    }
}

pub fn card() -> Frame {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(14)
        .inner_margin(Margin::same(16))
        .shadow(drop_shadow())
}

pub fn chassis() -> Frame {
    Frame::new()
        .fill(CHASSIS)
        .inner_margin(Margin::same(12))
        .stroke(Stroke::new(1.0, Color32::from_rgb(32, 44, 58)))
}

pub fn inset() -> Frame {
    Frame::new()
        .fill(INSET)
        .stroke(Stroke::new(1.0, Color32::from_rgb(36, 56, 70)))
        .corner_radius(10)
        .inner_margin(Margin::same(10))
}

pub fn alert(color: Color32) -> Frame {
    Frame::new()
        .fill(Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            28,
        ))
        .stroke(Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 150),
        ))
        .corner_radius(10)
        .inner_margin(Margin::same(10))
}

pub fn lab_button(text: impl Into<String>, fill: Color32, fg: Color32) -> Button<'static> {
    Button::new(RichText::new(text.into()).size(14.0).color(fg).strong())
        .fill(fill)
        .corner_radius(10)
        .min_size(vec2(0.0, 34.0))
}

pub fn primary_button(text: impl Into<String>, fill: Color32) -> Button<'static> {
    Button::new(RichText::new(text.into()).size(16.0).color(INK).strong())
        .fill(fill)
        .corner_radius(12)
        .min_size(vec2(0.0, 44.0))
}

pub fn backdrop(painter: &Painter, rect: Rect) {
    painter.rect_filled(rect, 0.0, BG);
    let step = 56.0;
    let mut x = (rect.left() / step).floor() * step;
    let grid = Color32::from_rgba_unmultiplied(130, 170, 190, 12);
    while x < rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, grid),
        );
        x += step;
    }
    let mut y = (rect.top() / step).floor() * step;
    while y < rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(1.0, grid),
        );
        y += step;
    }
    painter.circle_filled(
        rect.left_top() + vec2(120.0, 80.0),
        220.0,
        Color32::from_rgba_unmultiplied(72, 214, 236, 16),
    );
    painter.circle_filled(
        rect.right_bottom() - vec2(80.0, 40.0),
        260.0,
        Color32::from_rgba_unmultiplied(245, 186, 66, 14),
    );
}

pub fn wrapped(ui: &mut egui::Ui, text: RichText) {
    ui.add(Label::new(text).wrap());
}

pub fn task_row(ui: &mut egui::Ui, done: bool, text: &str) {
    let width = ui.available_width();
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
        paint_mark(ui.painter(), rect, done);
        let text_w = (width - 28.0).max(40.0);
        ui.allocate_ui_with_layout(vec2(text_w, 0.0), Layout::top_down(Align::LEFT), |ui| {
            ui.set_width(text_w);
            ui.add(
                Label::new(RichText::new(text).size(13.5).color(if done {
                    TEXT
                } else {
                    TEXT_DIM
                }))
                .wrap(),
            );
        });
    });
}

fn paint_mark(painter: &Painter, rect: Rect, done: bool) {
    let box_rect = Rect::from_center_size(rect.center(), vec2(15.0, 15.0));
    if done {
        painter.rect_filled(box_rect, 4.0, MINT);
        let c = box_rect.center();
        painter.line_segment(
            [c + vec2(-4.0, 0.0), c + vec2(-1.0, 3.2)],
            Stroke::new(2.0, INK),
        );
        painter.line_segment(
            [c + vec2(-1.0, 3.2), c + vec2(4.5, -3.4)],
            Stroke::new(2.0, INK),
        );
    } else {
        painter.rect_stroke(box_rect, 4.0, Stroke::new(1.4, MUTED), StrokeKind::Inside);
    }
}

pub fn stars(ui: &mut egui::Ui, earned: u8, radius: f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = (radius * 0.45).max(4.0);
        for index in 0..3 {
            let side = radius * 2.15;
            let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
            paint_star(ui.painter(), rect.center(), radius, index < earned);
        }
    });
}

pub fn stars_centered(ui: &mut egui::Ui, earned: u8, radius: f32) {
    ui.vertical_centered(|ui| {
        let gap = (radius * 0.45).max(4.0);
        let side = radius * 2.15;
        let total = side * 3.0 + gap * 2.0;
        ui.horizontal(|ui| {
            ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
            stars(ui, earned, radius);
        });
    });
}

pub fn paint_star(painter: &Painter, center: Pos2, radius: f32, earned: bool) {
    if earned {
        painter.circle_filled(
            center,
            radius * 1.15,
            Color32::from_rgba_unmultiplied(245, 186, 66, 48),
        );
    }
    let points = (0..10)
        .map(|point| {
            let angle = -std::f32::consts::FRAC_PI_2 + point as f32 * std::f32::consts::PI / 5.0;
            let arm = if point % 2 == 0 {
                radius
            } else {
                radius * 0.46
            };
            center + Vec2::new(angle.cos() * arm, angle.sin() * arm)
        })
        .collect();
    painter.add(egui::Shape::convex_polygon(
        points,
        if earned {
            AMBER
        } else {
            Color32::from_rgb(36, 48, 62)
        },
        Stroke::new(1.4, if earned { AMBER } else { MUTED }),
    ));
}

pub fn emblem(painter: &Painter, center: Pos2, radius: f32, time: f32) {
    let pulse = 0.5 + 0.5 * (time * 1.7).sin();
    painter.circle_filled(
        center,
        radius + 10.0,
        Color32::from_rgba_unmultiplied(72, 214, 236, (22.0 + 16.0 * pulse) as u8),
    );
    painter.circle_filled(center, radius, Color32::from_rgb(12, 22, 32));
    painter.circle_stroke(center, radius - 1.5, Stroke::new(2.0, CYAN));
    painter.circle_stroke(
        center,
        radius - 7.0,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(245, 186, 66, 160)),
    );

    let left = center + vec2(-radius * 0.34, 2.0);
    painter.line_segment(
        [left + vec2(0.0, -12.0), left + vec2(0.0, 12.0)],
        Stroke::new(2.4, TEXT),
    );
    painter.line_segment(
        [left + vec2(7.0, -7.0), left + vec2(7.0, 7.0)],
        Stroke::new(4.0, TEXT),
    );
    painter.text(
        left + vec2(-12.0, -16.0),
        egui::Align2::CENTER_CENTER,
        "+",
        FontId::proportional(14.0),
        CORAL,
    );

    let lamp = center + vec2(radius * 0.28, -2.0);
    painter.circle_filled(
        lamp,
        radius * 0.22,
        Color32::from_rgba_unmultiplied(245, 186, 66, 50),
    );
    painter.circle_stroke(lamp, radius * 0.18, Stroke::new(1.8, AMBER));
    painter.line_segment(
        [left + vec2(12.0, -8.0), lamp + vec2(-radius * 0.16, -2.0)],
        Stroke::new(1.5, TEXT_DIM),
    );
    painter.line_segment(
        [left + vec2(12.0, 8.0), lamp + vec2(-radius * 0.12, 6.0)],
        Stroke::new(1.5, TEXT_DIM),
    );
}

pub fn level_glyph(painter: &Painter, rect: Rect, index: usize, color: Color32) {
    painter.rect_filled(
        rect,
        8.0,
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 36),
    );
    painter.rect_stroke(rect, 8.0, Stroke::new(1.2, color), StrokeKind::Inside);
    let c = rect.center();
    match index {
        0 => {
            painter.circle_stroke(c + vec2(0.0, -1.0), 8.0, Stroke::new(1.6, color));
            painter.line_segment(
                [c + vec2(-4.0, 1.0), c + vec2(-1.0, -3.0)],
                Stroke::new(1.4, color),
            );
            painter.line_segment(
                [c + vec2(-1.0, -3.0), c + vec2(2.0, 2.0)],
                Stroke::new(1.4, color),
            );
            painter.line_segment(
                [c + vec2(2.0, 2.0), c + vec2(5.0, -2.0)],
                Stroke::new(1.4, color),
            );
        }
        1 => {
            let body = Rect::from_center_size(c, vec2(18.0, 8.0));
            painter.rect_filled(body, 2.0, color);
            painter.line_segment(
                [c + vec2(-4.0, -4.0), c + vec2(-4.0, 4.0)],
                Stroke::new(1.5, INK),
            );
            painter.line_segment(
                [c + vec2(1.0, -4.0), c + vec2(1.0, 4.0)],
                Stroke::new(1.5, INK),
            );
        }
        2 => {
            painter.circle_stroke(c + vec2(-6.0, 0.0), 5.0, Stroke::new(1.6, color));
            painter.circle_stroke(c + vec2(6.0, 0.0), 5.0, Stroke::new(1.6, color));
            painter.line_segment(
                [c + vec2(-6.0, -7.0), c + vec2(6.0, -7.0)],
                Stroke::new(1.3, color),
            );
            painter.line_segment(
                [c + vec2(-6.0, 7.0), c + vec2(6.0, 7.0)],
                Stroke::new(1.3, color),
            );
        }
        3 => {
            painter.circle_stroke(c + vec2(-2.0, -2.0), 6.0, Stroke::new(1.6, color));
            painter.line_segment(
                [c + vec2(2.5, 2.5), c + vec2(8.0, 8.0)],
                Stroke::new(1.8, color),
            );
        }
        _ => {
            let body = Rect::from_center_size(c, vec2(10.0, 16.0));
            painter.rect_filled(body, 2.0, TEXT);
            painter.rect_filled(
                Rect::from_center_size(c + vec2(0.0, -3.0), vec2(6.0, 5.0)),
                1.5,
                color,
            );
        }
    }
}

pub fn centered_column(ui: &mut egui::Ui, max_width: f32, add: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width().min(max_width).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
        ui.vertical(|ui| {
            ui.set_width(width);
            add(ui);
        });
    });
}

pub fn fill_button(
    ui: &mut egui::Ui,
    text: &str,
    fill: Color32,
    text_color: Color32,
    height: f32,
) -> egui::Response {
    let width = ui.available_width().min(340.0);
    ui.vertical_centered(|ui| {
        ui.add_sized(
            [width, height],
            lab_button(text, fill, text_color)
                .min_size(vec2(0.0, height))
                .wrap(),
        )
    })
    .inner
}
