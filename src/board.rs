use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Id, PointerButton, Pos2, Rect, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};

use crate::model::{
    BOARD_HEIGHT, BOARD_WIDTH, ComponentDef, LevelDef, LevelSession, PartKind, TerminalKind,
    TerminalRef, Tool,
};
use crate::simulation::{CircuitStatus, ElectricalReading};

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

    painter.rect_filled(rect, 18.0, Color32::from_rgb(13, 22, 43));
    painter.rect_filled(board_rect, 14.0, Color32::from_rgb(20, 34, 60));
    draw_grid(painter, board_rect, &transform);

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
        let mut color = terminal_color(terminal_kind);
        if nearest_wire == Some(index) {
            color = Color32::from_rgb(255, 100, 115);
        }
        painter.line_segment([a, b], Stroke::new(8.0, Color32::from_rgb(8, 15, 29)));
        painter.line_segment([a, b], Stroke::new(4.0, color));
        if terminal_kind == TerminalKind::Protective {
            for step in 0..7 {
                let t = (step as f32 + 0.5) / 7.0;
                let point = a + (b - a) * t;
                painter.circle_filled(point, 2.2, Color32::from_rgb(255, 220, 60));
            }
        }
        if flowing && terminal_kind != TerminalKind::Protective {
            let speed = if reading.status == CircuitStatus::Dim {
                0.12
            } else {
                0.35
            };
            for dot in 0..3 {
                let t = (time * speed + dot as f32 / 3.0 + index as f32 * 0.07).fract();
                let point = a + (b - a) * t;
                let dot_color = if reading.status == CircuitStatus::Overload {
                    Color32::from_rgb(255, 100, 85)
                } else {
                    Color32::from_rgb(255, 244, 160)
                };
                painter.circle_filled(point, 3.3, dot_color);
            }
        }
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
            let color = terminal_color(terminal.kind);
            painter.circle_filled(center, radius + 3.0, Color32::from_rgb(8, 15, 29));
            painter.circle_filled(center, radius, color);
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(
                    if selected { 3.0 } else { 1.5 },
                    if selected || response.hovered() {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(190, 207, 225)
                    },
                ),
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

    if let Some(first) = session.selected_terminal
        && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
    {
        let start = terminal_position(def, &transform, first);
        painter.line_segment(
            [start, pointer],
            Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
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

fn draw_grid(painter: &egui::Painter, rect: Rect, transform: &Transform) {
    for x in 0..=20 {
        for y in 0..=11 {
            let point = transform.point((x as f32 * 50.0, y as f32 * 50.0));
            if rect.contains(point) {
                painter.circle_filled(point, 1.0, Color32::from_rgb(43, 61, 87));
            }
        }
    }
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
        TerminalKind::Generic => Color32::from_rgb(91, 213, 235),
        TerminalKind::Positive => Color32::from_rgb(255, 99, 108),
        TerminalKind::Negative => Color32::from_rgb(150, 165, 185),
        TerminalKind::Phase => Color32::from_rgb(224, 143, 72),
        TerminalKind::Neutral => Color32::from_rgb(74, 153, 255),
        TerminalKind::Protective => Color32::from_rgb(79, 195, 116),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_component(
    painter: &egui::Painter,
    session: &LevelSession,
    reading: &ElectricalReading,
    panel_lamp_on: bool,
    index: usize,
    component: &ComponentDef,
    rect: Rect,
    hovered: bool,
) {
    let state = &session.components[index];
    let Some(part) = state.placed else {
        painter.rect_filled(rect, 12.0, Color32::from_rgb(25, 43, 70));
        painter.rect_stroke(
            rect,
            12.0,
            Stroke::new(2.0, Color32::from_rgb(79, 111, 147)),
            StrokeKind::Outside,
        );
        painter.text(
            rect.center() - Vec2::new(0.0, 8.0),
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(28.0),
            Color32::from_rgb(106, 148, 187),
        );
        painter.text(
            rect.center() + Vec2::new(0.0, 20.0),
            Align2::CENTER_CENTER,
            component.name,
            FontId::proportional(13.0),
            Color32::from_rgb(154, 177, 202),
        );
        return;
    };

    let selected = session.selected_component == Some(index);
    let border = if selected {
        Color32::from_rgb(255, 220, 91)
    } else if hovered {
        Color32::from_rgb(132, 205, 239)
    } else {
        Color32::from_rgb(81, 108, 143)
    };
    let fill = match part {
        PartKind::DcSource | PartKind::AcSource => Color32::from_rgb(38, 74, 96),
        PartKind::BusN => Color32::from_rgb(28, 66, 111),
        PartKind::BusPe => Color32::from_rgb(31, 83, 61),
        _ => Color32::from_rgb(37, 55, 84),
    };
    painter.rect_filled(rect, 12.0, fill);
    painter.rect_stroke(
        rect,
        12.0,
        Stroke::new(if selected { 3.0 } else { 1.5 }, border),
        StrokeKind::Outside,
    );

    let center = rect.center();
    let unit = (rect.width() / 130.0).clamp(0.55, 1.25);
    match part {
        PartKind::DcSource => draw_battery(painter, rect, part == PartKind::AcSource),
        PartKind::AcSource => draw_ac_source(painter, rect),
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
            draw_breaker(painter, rect, state.closed, state.rating)
        }
        PartKind::Rcd => draw_rcd(painter, rect, state.closed, state.rating),
        PartKind::BusN | PartKind::BusPe => draw_bus(painter, rect, part),
        PartKind::Socket => draw_socket(painter, rect),
    }

    let name_y = rect.bottom() - 13.0;
    painter.text(
        Pos2::new(center.x, name_y),
        Align2::CENTER_CENTER,
        component.name,
        FontId::proportional(12.5),
        Color32::from_rgb(218, 230, 243),
    );
}

fn draw_battery(painter: &egui::Painter, rect: Rect, _ac: bool) {
    let center = rect.center() - Vec2::new(0.0, 8.0);
    painter.line_segment(
        [
            center + Vec2::new(-22.0, -22.0),
            center + Vec2::new(-22.0, 22.0),
        ],
        Stroke::new(4.0, Color32::from_rgb(240, 244, 249)),
    );
    painter.line_segment(
        [
            center + Vec2::new(5.0, -13.0),
            center + Vec2::new(5.0, 13.0),
        ],
        Stroke::new(7.0, Color32::from_rgb(240, 244, 249)),
    );
    painter.text(
        center + Vec2::new(-38.0, -30.0),
        Align2::CENTER_CENTER,
        "+",
        FontId::proportional(20.0),
        Color32::from_rgb(255, 116, 121),
    );
}

fn draw_ac_source(painter: &egui::Painter, rect: Rect) {
    let center = rect.center() - Vec2::new(0.0, 5.0);
    painter.circle_stroke(
        center,
        31.0,
        Stroke::new(2.0, Color32::from_rgb(215, 232, 243)),
    );
    let points = (0..=24)
        .map(|step| {
            let x = -25.0 + step as f32 * 50.0 / 24.0;
            let y = (step as f32 / 24.0 * std::f32::consts::TAU).sin() * 12.0;
            center + Vec2::new(x, y)
        })
        .collect::<Vec<_>>();
    painter.add(egui::Shape::line(
        points,
        Stroke::new(2.0, Color32::from_rgb(255, 205, 94)),
    ));
}

fn draw_switch(painter: &egui::Painter, center: Pos2, unit: f32, closed: bool) {
    let left = center + Vec2::new(-32.0, 0.0) * unit;
    let right = center + Vec2::new(32.0, 0.0) * unit;
    let end = if closed {
        right
    } else {
        center + Vec2::new(24.0, -28.0) * unit
    };
    painter.circle_filled(left, 4.0 * unit, Color32::WHITE);
    painter.circle_filled(right, 4.0 * unit, Color32::WHITE);
    painter.line_segment(
        [left, end],
        Stroke::new(4.0 * unit, Color32::from_rgb(255, 208, 91)),
    );
}

fn draw_lamp(painter: &egui::Painter, center: Pos2, unit: f32, brightness: f32, short: bool) {
    let lamp_center = center - Vec2::new(0.0, 8.0) * unit;
    if brightness > 0.0 {
        painter.circle_filled(
            lamp_center,
            39.0 * unit,
            Color32::from_rgba_unmultiplied(255, 214, 74, (70.0 * brightness) as u8),
        );
    }
    let glass = if brightness > 0.0 {
        Color32::from_rgb(255, 224, 102)
    } else {
        Color32::from_rgb(168, 190, 210)
    };
    painter.circle_stroke(lamp_center, 26.0 * unit, Stroke::new(2.5 * unit, glass));
    painter.add(egui::Shape::line(
        vec![
            lamp_center + Vec2::new(-12.0, 8.0) * unit,
            lamp_center + Vec2::new(-5.0, -7.0) * unit,
            lamp_center + Vec2::new(3.0, 8.0) * unit,
            lamp_center + Vec2::new(11.0, -7.0) * unit,
        ],
        Stroke::new(2.0 * unit, glass),
    ));
    if short {
        painter.line_segment(
            [
                lamp_center + Vec2::new(-20.0, -20.0) * unit,
                lamp_center + Vec2::new(20.0, 20.0) * unit,
            ],
            Stroke::new(3.0, Color32::from_rgb(255, 100, 110)),
        );
    }
}

fn draw_led(painter: &egui::Painter, center: Pos2, unit: f32, brightness: f32) {
    let center = center - Vec2::new(0.0, 8.0) * unit;
    let color = if brightness > 0.4 {
        Color32::from_rgb(255, 102, 120)
    } else {
        Color32::from_rgb(161, 102, 128)
    };
    if brightness > 0.0 {
        painter.circle_filled(
            center,
            36.0 * unit,
            Color32::from_rgba_unmultiplied(255, 73, 107, (60.0 * brightness) as u8),
        );
    }
    painter.add(egui::Shape::convex_polygon(
        vec![
            center + Vec2::new(-18.0, -20.0) * unit,
            center + Vec2::new(-18.0, 20.0) * unit,
            center + Vec2::new(15.0, 0.0) * unit,
        ],
        color,
        Stroke::new(1.5, Color32::WHITE),
    ));
    painter.line_segment(
        [
            center + Vec2::new(17.0, -22.0) * unit,
            center + Vec2::new(17.0, 22.0) * unit,
        ],
        Stroke::new(3.0 * unit, Color32::WHITE),
    );
}

fn draw_resistor(painter: &egui::Painter, center: Pos2, unit: f32, part: PartKind) {
    let body = Rect::from_center_size(
        center - Vec2::new(0.0, 6.0) * unit,
        Vec2::new(72.0, 30.0) * unit,
    );
    painter.rect_filled(body, 7.0 * unit, Color32::from_rgb(231, 188, 126));
    painter.rect_stroke(
        body,
        7.0 * unit,
        Stroke::new(1.5, Color32::from_rgb(255, 231, 190)),
        StrokeKind::Outside,
    );
    let bands = [Color32::from_rgb(125, 69, 42), Color32::BLACK, Color32::RED];
    for (index, color) in bands.into_iter().enumerate() {
        let x = body.left() + (20.0 + index as f32 * 16.0) * unit;
        painter.line_segment(
            [
                Pos2::new(x, body.top() + 3.0),
                Pos2::new(x, body.bottom() - 3.0),
            ],
            Stroke::new(4.0 * unit, color),
        );
    }
    let label = match part {
        PartKind::Resistor100 => "100 Ом",
        PartKind::Resistor1000 => "1 кОм",
        _ => "300 Ом",
    };
    painter.text(
        center - Vec2::new(0.0, 30.0) * unit,
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.0 * unit),
        Color32::from_rgb(255, 227, 156),
    );
}

fn draw_fuse(painter: &egui::Painter, center: Pos2, unit: f32, apparently_whole: bool) {
    let center = center - Vec2::new(0.0, 5.0) * unit;
    let body = Rect::from_center_size(center, Vec2::new(74.0, 28.0) * unit);
    painter.rect_filled(
        body,
        12.0 * unit,
        Color32::from_rgba_unmultiplied(160, 216, 232, 70),
    );
    painter.rect_stroke(
        body,
        12.0 * unit,
        Stroke::new(2.0, Color32::from_rgb(167, 218, 235)),
        StrokeKind::Outside,
    );
    if apparently_whole {
        painter.line_segment(
            [body.left_center(), body.right_center()],
            Stroke::new(2.0, Color32::from_rgb(255, 216, 105)),
        );
    } else {
        painter.line_segment(
            [body.left_center(), center - Vec2::new(8.0, 5.0) * unit],
            Stroke::new(2.0, Color32::from_rgb(255, 120, 110)),
        );
        painter.line_segment(
            [center + Vec2::new(8.0, 5.0) * unit, body.right_center()],
            Stroke::new(2.0, Color32::from_rgb(255, 120, 110)),
        );
    }
}

fn draw_breaker(painter: &egui::Painter, rect: Rect, closed: bool, rating: Option<u32>) {
    let center = rect.center() - Vec2::new(0.0, 8.0);
    let lever = Rect::from_center_size(center, Vec2::new(28.0, 48.0));
    painter.rect_filled(lever, 5.0, Color32::from_rgb(218, 228, 237));
    let handle_center = if closed {
        center - Vec2::new(0.0, 11.0)
    } else {
        center + Vec2::new(0.0, 11.0)
    };
    painter.rect_filled(
        Rect::from_center_size(handle_center, Vec2::new(20.0, 15.0)),
        3.0,
        if closed {
            Color32::from_rgb(86, 204, 139)
        } else {
            Color32::from_rgb(255, 119, 105)
        },
    );
    painter.text(
        Pos2::new(center.x, rect.top() + 22.0),
        Align2::CENTER_CENTER,
        rating.map_or("— А".to_owned(), |value| format!("{value} А")),
        FontId::proportional(15.0),
        Color32::from_rgb(255, 222, 126),
    );
}

fn draw_rcd(painter: &egui::Painter, rect: Rect, closed: bool, rating: Option<u32>) {
    let center = rect.center();
    painter.text(
        Pos2::new(center.x, rect.top() + 23.0),
        Align2::CENTER_CENTER,
        "УЗО · L + N",
        FontId::proportional(15.0),
        Color32::from_rgb(135, 210, 255),
    );
    painter.circle_stroke(
        center - Vec2::new(0.0, 3.0),
        20.0,
        Stroke::new(
            3.0,
            if closed {
                Color32::from_rgb(93, 211, 151)
            } else {
                Color32::from_rgb(255, 111, 105)
            },
        ),
    );
    painter.text(
        center + Vec2::new(0.0, 25.0),
        Align2::CENTER_CENTER,
        rating.map_or("Уставка —".to_owned(), |value| {
            format!("{value} мА")
        }),
        FontId::proportional(14.0),
        Color32::from_rgb(255, 222, 126),
    );
}

fn draw_bus(painter: &egui::Painter, rect: Rect, part: PartKind) {
    let color = if part == PartKind::BusN {
        Color32::from_rgb(86, 164, 255)
    } else {
        Color32::from_rgb(102, 214, 131)
    };
    let strip = Rect::from_center_size(rect.center(), Vec2::new(rect.width() * 0.72, 22.0));
    painter.rect_filled(strip, 7.0, color);
    for step in 0..4 {
        let x = strip.left() + 18.0 + step as f32 * (strip.width() - 36.0) / 3.0;
        painter.circle_filled(
            Pos2::new(x, strip.center().y),
            4.0,
            Color32::from_rgb(225, 237, 242),
        );
    }
}

fn draw_socket(painter: &egui::Painter, rect: Rect) {
    let center = rect.center() - Vec2::new(0.0, 8.0);
    painter.circle_stroke(
        center,
        36.0,
        Stroke::new(3.0, Color32::from_rgb(206, 221, 237)),
    );
    painter.circle_filled(
        center + Vec2::new(-13.0, 0.0),
        5.0,
        Color32::from_rgb(206, 221, 237),
    );
    painter.circle_filled(
        center + Vec2::new(13.0, 0.0),
        5.0,
        Color32::from_rgb(206, 221, 237),
    );
    painter.text(
        center + Vec2::new(0.0, -21.0),
        Align2::CENTER_CENTER,
        "PE",
        FontId::proportional(12.0),
        Color32::from_rgb(114, 220, 137),
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
