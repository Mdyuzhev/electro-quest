use std::sync::Arc;

use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Galley, Id, Painter, PointerButton, Pos2,
    Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

use crate::model::{
    BOARD_HEIGHT, BOARD_WIDTH, ComponentDef, LevelDef, LevelSession, PartKind, TerminalKind,
    TerminalRef, Tool,
};
use crate::simulation::{CircuitStatus, ElectricalReading};
use crate::theme;

use eframe::egui::text::LayoutJob;

#[derive(Clone, Copy, Debug)]
pub enum BoardAction {
    Component(usize),
    RemoveComponent(usize),
    Terminal(TerminalRef),
    RemoveWire(usize),
}

pub struct BoardOutput {
    pub action: Option<BoardAction>,
    pub slots: Vec<(usize, Rect)>,
}

struct Transform {
    origin: Pos2,
    scale: f32,
}

impl Transform {
    fn point(&self, point: (f32, f32)) -> Pos2 {
        self.origin + Vec2::new(point.0 * self.scale, point.1 * self.scale)
    }

    fn length(&self, value: f32) -> f32 {
        value * self.scale
    }
}

pub fn show(
    ui: &mut Ui,
    def: &LevelDef,
    session: &LevelSession,
    reading: &ElectricalReading,
    time: f32,
) -> BoardOutput {
    let available = ui.available_size();
    let (rect, board_response) = ui.allocate_exact_size(available, Sense::click());
    let scale = (rect.width() / BOARD_WIDTH)
        .min(rect.height() / BOARD_HEIGHT)
        .max(0.1);
    let content_size = Vec2::new(BOARD_WIDTH * scale, BOARD_HEIGHT * scale);
    let origin = rect.center() - content_size / 2.0;
    let transform = Transform { origin, scale };
    let board_rect = Rect::from_min_size(origin, content_size);
    let painter = ui.painter();

    painter.rect_filled(rect, 16.0, Color32::from_rgb(12, 16, 20));
    painter.rect_filled(board_rect, 12.0, theme::PCB);
    painter.rect_stroke(
        board_rect,
        12.0,
        Stroke::new(1.5, Color32::from_rgb(46, 96, 82)),
        StrokeKind::Inside,
    );
    painter.rect_stroke(
        board_rect.shrink(5.0),
        8.0,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(196, 140, 72, 55)),
        StrokeKind::Inside,
    );
    draw_grid(painter, board_rect, &transform);
    draw_screws(painter, rect, board_rect);

    let panel_lamp_on = session.level == 4
        && session.power_connected
        && session.has_flag("normal_ok")
        && ["main", "rcd", "light_breaker"].iter().all(|id| {
            def.component_index(id)
                .is_some_and(|index| session.components[index].closed)
        });

    let flowing = matches!(
        reading.status,
        CircuitStatus::Running | CircuitStatus::Overload | CircuitStatus::Dim
    ) || (session.level == 4
        && session.power_connected
        && session.has_flag("normal_ok")
        && session.panel_event.is_none());

    let mut nearest_wire = None;
    if session.tool == Tool::Erase
        && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
    {
        nearest_wire = session
            .wires
            .iter()
            .enumerate()
            .filter_map(|(index, wire)| {
                let a = terminal_position(def, &transform, wire.from);
                let b = terminal_position(def, &transform, wire.to);
                let distance = distance_to_segment(pointer, a, b);
                (distance < 13.0).then_some((index, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index);
    }

    for (index, wire) in session.wires.iter().enumerate() {
        let a = terminal_position(def, &transform, wire.from);
        let b = terminal_position(def, &transform, wire.to);
        let terminal_kind = def.terminal_def(wire.from).kind;
        let color = if nearest_wire == Some(index) {
            theme::CORAL
        } else {
            terminal_color(terminal_kind)
        };
        draw_wire(
            painter,
            a,
            b,
            color,
            terminal_kind,
            flowing,
            reading.status,
            time,
            index,
        );
    }

    let mut slots = Vec::new();
    let mut component_action = None;
    for (index, component) in def.components.iter().enumerate() {
        let center = transform.point(component.position);
        let size = Vec2::new(
            transform.length(component.size.0),
            transform.length(component.size.1),
        );
        let component_rect = Rect::from_center_size(center, size);
        slots.push((index, component_rect));
        let response = ui.interact(
            component_rect,
            Id::unique(("component", session.level, index)),
            Sense::click(),
        );
        let hovered = response.hovered() || ui.rect_contains_pointer(component_rect);
        draw_component(
            painter,
            session,
            reading,
            panel_lamp_on,
            index,
            component,
            component_rect,
            hovered,
        );
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if response.clicked() {
            component_action = Some(BoardAction::Component(index));
        } else if response.clicked_by(PointerButton::Secondary) {
            component_action = Some(BoardAction::RemoveComponent(index));
        }
    }

    let mut terminal_action = None;
    for (index, component) in def.components.iter().enumerate() {
        if session.components[index].placed.is_none() {
            continue;
        }
        for (terminal_index, terminal) in component.terminals.iter().enumerate() {
            let terminal_ref = TerminalRef {
                component: index,
                terminal: terminal_index,
            };
            let center = terminal_position(def, &transform, terminal_ref);
            let radius = transform.length(10.0).clamp(6.0, 11.0);
            let response = ui.interact(
                Rect::from_center_size(center, Vec2::splat(radius * 3.0)),
                Id::unique(("terminal", session.level, index, terminal_index)),
                Sense::click(),
            );
            let selected = session.selected_terminal == Some(terminal_ref);
            draw_terminal(
                painter,
                center,
                radius,
                terminal_color(terminal.kind),
                selected || response.hovered(),
                time,
            );
            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                let text = format!("{} · {}", component.name, terminal.label);
                response.clone().on_hover_text(text);
            }
            if response.clicked() {
                terminal_action = Some(BoardAction::Terminal(terminal_ref));
            }
        }
    }

    for component in def.components.iter() {
        let center = transform.point(component.position);
        let size = Vec2::new(
            transform.length(component.size.0),
            transform.length(component.size.1),
        );
        let component_rect = Rect::from_center_size(center, size);
        paint_caption(painter, component_rect, component.name);
    }

    if let Some(first) = session.selected_terminal
        && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
    {
        let start = terminal_position(def, &transform, first);
        painter.line_segment(
            [start, pointer],
            Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 170)),
        );
    }

    let wire_action =
        if terminal_action.is_none() && component_action.is_none() && board_response.clicked() {
            nearest_wire.map(BoardAction::RemoveWire)
        } else {
            None
        };

    BoardOutput {
        action: terminal_action.or(component_action).or(wire_action),
        slots,
    }
}

fn draw_screws(painter: &Painter, rect: Rect, board: Rect) {
    let points = [
        rect.left_top() + vec2(12.0, 12.0),
        rect.right_top() + vec2(-12.0, 12.0),
        rect.left_bottom() + vec2(12.0, -12.0),
        rect.right_bottom() + vec2(-12.0, -12.0),
    ];
    for point in points {
        if board.expand(6.0).contains(point) {
            continue;
        }
        painter.circle_filled(point, 4.5, Color32::from_rgb(36, 46, 58));
        painter.circle_stroke(
            point,
            4.5,
            Stroke::new(1.0, Color32::from_rgb(96, 112, 128)),
        );
        painter.line_segment(
            [point + vec2(-2.2, 0.0), point + vec2(2.2, 0.0)],
            Stroke::new(1.0, Color32::from_rgb(160, 176, 190)),
        );
    }
}

fn draw_grid(painter: &Painter, rect: Rect, transform: &Transform) {
    for x in 0..=20 {
        for y in 0..=11 {
            let point = transform.point((x as f32 * 50.0, y as f32 * 50.0));
            if rect.shrink(8.0).contains(point) {
                let major = x % 2 == 0 && y % 2 == 0;
                painter.circle_filled(
                    point,
                    if major { 1.7 } else { 1.05 },
                    if major {
                        theme::PCB_DOT_MAJOR
                    } else {
                        theme::PCB_DOT
                    },
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_wire(
    painter: &Painter,
    a: Pos2,
    b: Pos2,
    color: Color32,
    kind: TerminalKind,
    flowing: bool,
    status: CircuitStatus,
    time: f32,
    index: usize,
) {
    painter.line_segment([a, b], Stroke::new(8.0, Color32::from_black_alpha(150)));
    painter.line_segment([a, b], Stroke::new(5.5, color.gamma_multiply(0.28)));
    painter.line_segment([a, b], Stroke::new(2.4, color));
    if kind == TerminalKind::Protective {
        let length = a.distance(b);
        if length > 8.0 {
            let dir = (b - a) / length;
            let mut travel = 5.0;
            while travel < length - 4.0 {
                let end = (travel + 6.0).min(length);
                painter.line_segment(
                    [a + dir * travel, a + dir * end],
                    Stroke::new(2.0, Color32::from_rgb(255, 220, 80)),
                );
                travel += 14.0;
            }
        }
    }
    if flowing && kind != TerminalKind::Protective {
        let speed = if status == CircuitStatus::Dim {
            0.12
        } else {
            0.35
        };
        let dot_color = if status == CircuitStatus::Overload {
            theme::CORAL
        } else {
            Color32::from_rgb(255, 244, 190)
        };
        for dot in 0..3 {
            let t = (time * speed + dot as f32 / 3.0 + index as f32 * 0.07).fract();
            let point = a + (b - a) * t;
            painter.circle_filled(point, 6.5, dot_color.gamma_multiply(0.28));
            painter.circle_filled(point, 2.5, dot_color);
        }
    }
}

fn draw_terminal(
    painter: &Painter,
    center: Pos2,
    radius: f32,
    color: Color32,
    hot: bool,
    time: f32,
) {
    if hot {
        let pulse = 0.65 + 0.35 * (time * 5.0).sin();
        painter.circle_filled(
            center,
            radius + 5.0,
            Color32::from_white_alpha((70.0 * pulse) as u8),
        );
    }
    painter.circle_filled(center, radius + 2.4, Color32::from_rgb(6, 10, 14));
    painter.circle_filled(center, radius, color);
    painter.circle_filled(
        center - vec2(radius * 0.28, radius * 0.28),
        radius * 0.28,
        Color32::from_white_alpha(110),
    );
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            if hot { 2.0 } else { 1.2 },
            Color32::from_rgb(232, 238, 244),
        ),
    );
}

fn terminal_position(def: &LevelDef, transform: &Transform, terminal: TerminalRef) -> Pos2 {
    let component = &def.components[terminal.component];
    let terminal = &component.terminals[terminal.terminal];
    transform.point((
        component.position.0 + terminal.offset.0,
        component.position.1 + terminal.offset.1,
    ))
}

fn terminal_color(kind: TerminalKind) -> Color32 {
    match kind {
        TerminalKind::Generic => theme::CYAN,
        TerminalKind::Positive => theme::CORAL,
        TerminalKind::Negative => Color32::from_rgb(168, 180, 196),
        TerminalKind::Phase => theme::PHASE,
        TerminalKind::Neutral => theme::NEUTRAL,
        TerminalKind::Protective => theme::PROTECTIVE,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_component(
    painter: &Painter,
    session: &LevelSession,
    reading: &ElectricalReading,
    panel_lamp_on: bool,
    index: usize,
    _component: &ComponentDef,
    rect: Rect,
    hovered: bool,
) {
    let state = &session.components[index];
    let selected = session.selected_component == Some(index);
    if selected {
        painter.rect_stroke(
            rect.expand(3.0),
            14.0,
            Stroke::new(5.0, Color32::from_rgba_unmultiplied(245, 186, 66, 70)),
            StrokeKind::Outside,
        );
    }

    let Some(part) = state.placed else {
        painter.rect_filled(rect, 12.0, Color32::from_rgba_unmultiplied(18, 40, 44, 230));
        painter.rect_stroke(
            rect,
            12.0,
            Stroke::new(1.5, Color32::from_rgb(64, 120, 128)),
            StrokeKind::Inside,
        );
        let plus = rect.center() - vec2(0.0, rect.height() * 0.12);
        painter.text(
            plus,
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional((rect.height() * 0.28).clamp(16.0, 30.0)),
            Color32::from_rgb(120, 196, 196),
        );
        return;
    };

    let border = if selected {
        theme::AMBER
    } else if hovered {
        theme::CYAN
    } else {
        Color32::from_rgb(70, 96, 118)
    };
    let fill = match part {
        PartKind::DcSource | PartKind::AcSource => Color32::from_rgb(24, 52, 68),
        PartKind::BusN => Color32::from_rgb(18, 42, 78),
        PartKind::BusPe => Color32::from_rgb(18, 58, 42),
        _ => Color32::from_rgb(24, 36, 52),
    };
    painter.rect_filled(rect, 12.0, fill);
    let sheen = rect.shrink(1.0);
    let clipped = painter.with_clip_rect(sheen);
    clipped.add(egui::Shape::gradient_rect(
        sheen,
        egui::Direction::TopDown,
        [Color32::from_white_alpha(16), Color32::TRANSPARENT],
    ));
    painter.rect_stroke(
        rect,
        12.0,
        Stroke::new(if selected { 2.4 } else { 1.3 }, border),
        StrokeKind::Inside,
    );

    let caption_h = (rect.height() * 0.28).clamp(16.0, 34.0);
    let symbol =
        Rect::from_min_max(rect.min, pos2(rect.max.x, rect.bottom() - caption_h)).shrink(4.0);
    let unit = (symbol.width().min(symbol.height()) / 86.0).clamp(0.38, 1.25);
    let center = symbol.center();
    match part {
        PartKind::DcSource => draw_battery(painter, center, unit),
        PartKind::AcSource => draw_ac_source(painter, center, unit),
        PartKind::Switch => draw_switch(painter, center, unit, state.closed),
        PartKind::Lamp => draw_lamp(
            painter,
            center,
            unit,
            if panel_lamp_on {
                1.0
            } else {
                reading.brightness()
            },
            session.level == 3 && !state.healthy && session.has_flag("lamp_tested"),
        ),
        PartKind::Led => draw_led(painter, center, unit, reading.brightness()),
        PartKind::Resistor100 | PartKind::Resistor300 | PartKind::Resistor1000 => {
            draw_resistor(painter, center, unit, part)
        }
        PartKind::Fuse => draw_fuse(
            painter,
            center,
            unit,
            state.healthy || !session.has_flag("fuse_tested"),
        ),
        PartKind::MainBreaker | PartKind::BranchBreaker => {
            draw_breaker(painter, symbol, unit, state.closed, state.rating)
        }
        PartKind::Rcd => draw_rcd(painter, symbol, unit, state.closed, state.rating),
        PartKind::BusN | PartKind::BusPe => draw_bus(painter, symbol, part),
        PartKind::Socket => draw_socket(painter, center, unit),
    }
}

fn paint_caption(painter: &Painter, rect: Rect, text: &str) {
    let max_w = (rect.width() - 8.0).max(8.0);
    let max_h = (rect.height() * 0.4).clamp(16.0, 40.0);
    let max_size = (rect.width() / 9.2).clamp(9.0, 13.5);
    let galley = fitted_caption(painter, text, max_w, max_h, max_size);
    // Center-aligned galleys use x = 0 as the horizontal middle of the text.
    let y = (rect.bottom() - 3.0 - galley.rect.height()).max(rect.top() + 2.0);
    painter
        .with_clip_rect(rect.shrink(2.0))
        .galley(pos2(rect.center().x, y), galley, theme::TEXT);
}

fn fitted_caption(
    painter: &Painter,
    text: &str,
    max_w: f32,
    max_h: f32,
    max_size: f32,
) -> Arc<Galley> {
    let mut size = max_size;
    loop {
        let mut job = LayoutJob::simple(
            text.to_owned(),
            FontId::proportional(size),
            theme::TEXT,
            max_w,
        );
        job.halign = Align::Center;
        let galley = painter.layout_job(job);
        if galley.size().y <= max_h + 0.5 || size <= 8.0 {
            return galley;
        }
        size -= 0.5;
    }
}

fn center_line(painter: &Painter, center: Pos2, text: &str, size: f32, color: Color32, max_w: f32) {
    let mut font = size;
    let galley = loop {
        let galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(font), color);
        if galley.size().x <= max_w || font <= 8.0 {
            break galley;
        }
        font -= 0.5;
    };
    painter.galley(center - galley.size() * 0.5, galley, color);
}

fn draw_battery(painter: &Painter, center: Pos2, unit: f32) {
    painter.line_segment(
        [
            center + vec2(-16.0, -16.0) * unit,
            center + vec2(-16.0, 16.0) * unit,
        ],
        Stroke::new(3.2 * unit, theme::TEXT),
    );
    painter.line_segment(
        [
            center + vec2(6.0, -10.0) * unit,
            center + vec2(6.0, 10.0) * unit,
        ],
        Stroke::new(6.0 * unit, theme::TEXT),
    );
    painter.text(
        center + vec2(-28.0, -18.0) * unit,
        Align2::CENTER_CENTER,
        "+",
        FontId::proportional(16.0 * unit),
        theme::CORAL,
    );
}

fn draw_ac_source(painter: &Painter, center: Pos2, unit: f32) {
    painter.circle_stroke(center, 24.0 * unit, Stroke::new(1.8 * unit, theme::TEXT));
    let points = (0..=24)
        .map(|step| {
            let x = -18.0 + step as f32 * 36.0 / 24.0;
            let y = (step as f32 / 24.0 * std::f32::consts::TAU).sin() * 9.0;
            center + vec2(x, y) * unit
        })
        .collect::<Vec<_>>();
    painter.add(egui::Shape::line(
        points,
        Stroke::new(2.0 * unit, theme::AMBER),
    ));
}

fn draw_switch(painter: &Painter, center: Pos2, unit: f32, closed: bool) {
    let left = center + vec2(-26.0, 0.0) * unit;
    let right = center + vec2(26.0, 0.0) * unit;
    let end = if closed {
        right
    } else {
        center + vec2(18.0, -22.0) * unit
    };
    painter.circle_filled(left, 3.5 * unit, theme::TEXT);
    painter.circle_filled(right, 3.5 * unit, theme::TEXT);
    painter.line_segment([left, end], Stroke::new(3.2 * unit, theme::AMBER));
}

fn draw_lamp(painter: &Painter, center: Pos2, unit: f32, brightness: f32, short: bool) {
    if brightness > 0.0 {
        painter.circle_filled(
            center,
            34.0 * unit,
            Color32::from_rgba_unmultiplied(255, 210, 80, (90.0 * brightness) as u8),
        );
        painter.circle_filled(
            center,
            18.0 * unit,
            Color32::from_rgba_unmultiplied(255, 236, 170, (120.0 * brightness) as u8),
        );
    }
    let glass = if brightness > 0.0 {
        Color32::from_rgb(255, 224, 120)
    } else {
        Color32::from_rgb(176, 196, 214)
    };
    painter.circle_stroke(center, 20.0 * unit, Stroke::new(2.2 * unit, glass));
    painter.add(egui::Shape::line(
        vec![
            center + vec2(-9.0, 6.0) * unit,
            center + vec2(-3.0, -6.0) * unit,
            center + vec2(2.0, 6.0) * unit,
            center + vec2(8.0, -5.0) * unit,
        ],
        Stroke::new(1.7 * unit, glass),
    ));
    if short {
        painter.line_segment(
            [
                center + vec2(-14.0, -14.0) * unit,
                center + vec2(14.0, 14.0) * unit,
            ],
            Stroke::new(2.4 * unit, theme::CORAL),
        );
    }
}

fn draw_led(painter: &Painter, center: Pos2, unit: f32, brightness: f32) {
    let color = if brightness > 0.4 {
        Color32::from_rgb(255, 96, 118)
    } else {
        Color32::from_rgb(150, 90, 112)
    };
    if brightness > 0.0 {
        painter.circle_filled(
            center,
            28.0 * unit,
            Color32::from_rgba_unmultiplied(255, 80, 110, (80.0 * brightness) as u8),
        );
    }
    painter.add(egui::Shape::convex_polygon(
        vec![
            center + vec2(-14.0, -16.0) * unit,
            center + vec2(-14.0, 16.0) * unit,
            center + vec2(12.0, 0.0) * unit,
        ],
        color,
        Stroke::new(1.4 * unit, theme::TEXT),
    ));
    painter.line_segment(
        [
            center + vec2(14.0, -16.0) * unit,
            center + vec2(14.0, 16.0) * unit,
        ],
        Stroke::new(2.4 * unit, theme::TEXT),
    );
}

fn draw_resistor(painter: &Painter, center: Pos2, unit: f32, part: PartKind) {
    let body = Rect::from_center_size(center + vec2(0.0, 6.0) * unit, vec2(58.0, 22.0) * unit);
    painter.rect_filled(body, 6.0 * unit, Color32::from_rgb(214, 168, 104));
    painter.rect_stroke(
        body,
        6.0 * unit,
        Stroke::new(1.2, Color32::from_rgb(255, 226, 180)),
        StrokeKind::Outside,
    );
    let bands = [
        Color32::from_rgb(120, 64, 36),
        Color32::BLACK,
        Color32::from_rgb(190, 40, 40),
    ];
    for (index, color) in bands.into_iter().enumerate() {
        let x = body.left() + (14.0 + index as f32 * 12.0) * unit;
        painter.line_segment(
            [pos2(x, body.top() + 2.0), pos2(x, body.bottom() - 2.0)],
            Stroke::new(3.0 * unit, color),
        );
    }
    let label = match part {
        PartKind::Resistor100 => "100 Ом",
        PartKind::Resistor1000 => "1 кОм",
        _ => "300 Ом",
    };
    center_line(
        painter,
        center + vec2(0.0, -16.0) * unit,
        label,
        12.0 * unit,
        theme::AMBER,
        body.width() + 8.0,
    );
}

fn draw_fuse(painter: &Painter, center: Pos2, unit: f32, apparently_whole: bool) {
    let body = Rect::from_center_size(center, vec2(58.0, 20.0) * unit);
    painter.rect_filled(
        body,
        9.0 * unit,
        Color32::from_rgba_unmultiplied(160, 216, 232, 70),
    );
    painter.rect_stroke(
        body,
        9.0 * unit,
        Stroke::new(1.6 * unit, Color32::from_rgb(176, 220, 236)),
        StrokeKind::Outside,
    );
    if apparently_whole {
        painter.line_segment(
            [body.left_center(), body.right_center()],
            Stroke::new(1.8 * unit, theme::AMBER),
        );
    } else {
        painter.line_segment(
            [body.left_center(), center - vec2(6.0, 4.0) * unit],
            Stroke::new(1.8 * unit, theme::CORAL),
        );
        painter.line_segment(
            [center + vec2(6.0, 4.0) * unit, body.right_center()],
            Stroke::new(1.8 * unit, theme::CORAL),
        );
    }
}

fn draw_breaker(painter: &Painter, symbol: Rect, unit: f32, closed: bool, rating: Option<u32>) {
    let center = symbol.center();
    let lever = Rect::from_center_size(center + vec2(0.0, 4.0) * unit, vec2(22.0, 36.0) * unit);
    painter.rect_filled(lever, 4.0 * unit, Color32::from_rgb(226, 232, 238));
    let handle = if closed {
        center + vec2(0.0, -6.0) * unit
    } else {
        center + vec2(0.0, 12.0) * unit
    };
    painter.rect_filled(
        Rect::from_center_size(handle, vec2(16.0, 11.0) * unit),
        3.0,
        if closed { theme::MINT } else { theme::CORAL },
    );
    let label = rating.map_or("— А".to_owned(), |value| format!("{value} А"));
    center_line(
        painter,
        pos2(center.x, symbol.top() + 11.0 * unit.max(0.7)),
        &label,
        13.0 * unit,
        theme::AMBER,
        symbol.width() - 8.0,
    );
}

fn draw_rcd(painter: &Painter, symbol: Rect, unit: f32, closed: bool, rating: Option<u32>) {
    let center = symbol.center();
    center_line(
        painter,
        pos2(center.x, symbol.top() + 12.0),
        "УЗО · L + N",
        12.5 * unit.clamp(0.7, 1.1),
        theme::CYAN,
        symbol.width() - 8.0,
    );
    painter.circle_stroke(
        center + vec2(0.0, 2.0),
        15.0 * unit,
        Stroke::new(2.6 * unit, if closed { theme::MINT } else { theme::CORAL }),
    );
    let label = rating.map_or("уставка —".to_owned(), |value| {
        format!("{value} мА")
    });
    center_line(
        painter,
        center + vec2(0.0, 22.0) * unit,
        &label,
        12.5 * unit,
        theme::AMBER,
        symbol.width() - 6.0,
    );
}

fn draw_bus(painter: &Painter, symbol: Rect, part: PartKind) {
    let color = if part == PartKind::BusN {
        theme::NEUTRAL
    } else {
        theme::PROTECTIVE
    };
    let strip = Rect::from_center_size(
        symbol.center(),
        vec2(
            symbol.width() * 0.72,
            (symbol.height() * 0.42).clamp(12.0, 22.0),
        ),
    );
    painter.rect_filled(strip, 6.0, color);
    let holes = 4;
    for step in 0..holes {
        let x = strip.left() + 14.0 + step as f32 * (strip.width() - 28.0) / (holes as f32 - 1.0);
        painter.circle_filled(
            pos2(x, strip.center().y),
            3.4,
            Color32::from_rgb(232, 240, 244),
        );
    }
}

fn draw_socket(painter: &Painter, center: Pos2, unit: f32) {
    painter.circle_stroke(
        center,
        22.0 * unit,
        Stroke::new(2.2 * unit, theme::TEXT_DIM),
    );
    painter.circle_filled(center + vec2(-8.0, 2.0) * unit, 3.3 * unit, theme::TEXT);
    painter.circle_filled(center + vec2(8.0, 2.0) * unit, 3.3 * unit, theme::TEXT);
    center_line(
        painter,
        center + vec2(0.0, -10.0) * unit,
        "PE",
        11.0 * unit,
        theme::PROTECTIVE,
        40.0 * unit,
    );
}

fn distance_to_segment(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let length_squared = ab.length_sq();
    if length_squared == 0.0 {
        return point.distance(a);
    }
    let t = ((point - a).dot(ab) / length_squared).clamp(0.0, 1.0);
    point.distance(a + ab * t)
}

pub fn part_short_label(part: PartKind) -> &'static str {
    match part {
        PartKind::Resistor100 => "100 Ом",
        PartKind::Resistor300 => "300 Ом",
        PartKind::Resistor1000 => "1 кОм",
        PartKind::Rcd => "УЗО",
        PartKind::MainBreaker => "Вводной",
        PartKind::BranchBreaker => "Автомат",
        PartKind::DcSource => "Батарея",
        PartKind::AcSource => "Ввод",
        PartKind::BusN => "N",
        PartKind::BusPe => "PE",
        _ => part.label(),
    }
}
