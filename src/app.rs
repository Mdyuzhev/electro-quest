use std::time::Duration;

use eframe::egui::{
    self, Align, Color32, ComboBox, CursorIcon, FontId, Label, Layout, RichText, ScrollArea, Sense,
    Stroke, pos2, vec2,
};
use serde::{Deserialize, Serialize};

use crate::board::{self, part_short_label};
use crate::levels::all_levels;
use crate::model::{FeedbackKind, LevelDef, LevelSession, PanelEvent, TerminalRef, Tool, Wire};
use crate::shot::Capture;
use crate::simulation::{CircuitStatus, ElectricalReading, reading};
use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Menu,
    Map,
    Theory,
    Play,
    Result,
    Help,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Progress {
    pub unlocked: usize,
    pub stars: [u8; 5],
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            unlocked: 1,
            stars: [0; 5],
        }
    }
}

pub struct ElectroApp {
    pub(crate) levels: Vec<LevelDef>,
    pub(crate) progress: Progress,
    pub(crate) screen: Screen,
    pub(crate) current_level: usize,
    pub(crate) session: Option<LevelSession>,
    pub(crate) theory_return: Screen,
    pub(crate) result_stars: u8,
    time: f32,
    confirm_reset: bool,
    capture: Option<Capture>,
}

impl ElectroApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut progress: Progress = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        progress.unlocked = progress.unlocked.clamp(1, 5);
        let mut app = Self {
            levels: all_levels(),
            progress,
            screen: Screen::Menu,
            current_level: 0,
            session: None,
            theory_return: Screen::Play,
            result_stars: 0,
            time: 0.0,
            confirm_reset: false,
            capture: None,
        };
        let capture = Capture::from_env();
        if let Some(capture) = &capture {
            let scene = capture.scene.clone();
            app.apply_scene(&scene);
        }
        app.capture = capture;
        app
    }

    fn apply_scene(&mut self, scene: &str) {
        match scene {
            "menu" => self.screen = Screen::Menu,
            "reset" => {
                self.screen = Screen::Menu;
                self.confirm_reset = true;
            }
            "map" => self.screen = Screen::Map,
            "help" => self.screen = Screen::Help,
            "theory" | "theory0" => {
                self.current_level = 0;
                self.theory_return = Screen::Map;
                self.screen = Screen::Theory;
            }
            "theory4" => {
                self.current_level = 4;
                self.theory_return = Screen::Map;
                self.screen = Screen::Theory;
            }
            "play0" => self.open_level(0, false, false),
            "play0_live" => self.open_level(0, true, false),
            "play2" => self.open_level(2, false, false),
            "play4" => self.open_level(4, false, false),
            "play4_live" => self.open_level(4, true, false),
            "error" => self.open_level(0, false, true),
            "result" => {
                self.current_level = 0;
                self.result_stars = 3;
                self.screen = Screen::Result;
            }
            "result_final" => {
                self.current_level = 4;
                self.result_stars = 2;
                self.screen = Screen::Result;
            }
            _ => self.screen = Screen::Menu,
        }
    }

    fn open_level(&mut self, index: usize, live: bool, error: bool) {
        self.current_level = index;
        let def = self.levels[index].clone();
        let mut session = LevelSession::new(index, &def);
        if live {
            for (id, part) in def.required {
                if let Some(component) = def.component_index(id) {
                    let _ = session.place_part(&def, component, *part);
                }
            }
            for (component_index, component) in def.components.iter().enumerate() {
                if let Some(rating) = component.correct_rating {
                    session.components[component_index].rating = Some(rating);
                }
            }
            session.wires = def.expected_wires(0);
            if index == 0
                && let Some(switch) = def.component_index("switch")
            {
                session.components[switch].closed = true;
            }
            if index == 4 {
                session.power_connected = true;
                session.flag("normal_ok");
                session.last_reading = Some(
                    "Свет: 300 Вт, около 1,3 А. Розетки: 12 А. Общий ток: около 13,3 А. Защита не срабатывает."
                        .to_owned(),
                );
                session.set_feedback(
                    FeedbackKind::Success,
                    "Нормальная нагрузка работает: лампа светит, розетки запитаны, аппараты включены. Теперь проверь перегрузку.",
                );
            }
            session.selected_component = None;
        }
        if error {
            session.mistake("Цепь ещё не готова. Проверь детали и соединения.");
        }
        self.session = Some(session);
        self.screen = Screen::Play;
    }

    fn start_level(&mut self, index: usize) {
        self.current_level = index;
        self.session = Some(LevelSession::new(index, &self.levels[index]));
        self.theory_return = Screen::Play;
        self.screen = Screen::Theory;
    }

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("safety")
            .exact_size(48.0)
            .frame(theme::chassis())
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(
                        Label::new(
                            RichText::new(theme::SAFETY_LINE)
                                .size(12.5)
                                .color(theme::MUTED),
                        )
                        .wrap(),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                theme::backdrop(ui.painter(), ui.max_rect());
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(18.0);
                        self.menu_hero(ui);
                        ui.add_space(16.0);
                        self.menu_levels(ui);
                        ui.add_space(16.0);
                        self.menu_actions(ui);
                        ui.add_space(6.0);
                        self.menu_progress(ui);
                        ui.add_space(12.0);
                    });
            });

        if self.confirm_reset {
            self.reset_dialog(ui.ctx());
        }
    }

    fn menu_hero(&self, ui: &mut egui::Ui) {
        theme::centered_column(ui, 980.0, |ui| {
            ui.horizontal_top(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(96.0, 96.0), Sense::hover());
                theme::emblem(ui.painter(), rect.center(), 40.0, self.time);
                let text_w = (ui.available_width() - 4.0).max(40.0);
                ui.allocate_ui_with_layout(
                    vec2(text_w, 0.0),
                    Layout::top_down(Align::LEFT),
                    |ui| {
                        ui.set_width(text_w);
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("УЧЕБНАЯ ЛАБОРАТОРИЯ")
                                .size(13.0)
                                .color(theme::CYAN)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("ЭЛЕКТРОКВЕСТ")
                                .size(42.0)
                                .color(theme::AMBER)
                                .strong(),
                        );
                        theme::wrapped(
                            ui,
                            RichText::new("От первой лампочки до защитного щитка")
                                .size(18.0)
                                .color(theme::TEXT),
                        );
                        theme::wrapped(
                            ui,
                            RichText::new("Интерактивная игра по электротехнике · 7–9 классы")
                                .size(14.0)
                                .color(theme::MUTED),
                        );
                    },
                );
            });
        });
    }

    fn menu_levels(&mut self, ui: &mut egui::Ui) {
        let outer = ui.available_width();
        let gap = 12.0;
        let cols = (((outer + gap) / (200.0 + gap)).floor() as usize).clamp(1, 5);
        let card_w = ((outer - gap * (cols as f32 - 1.0)) / cols as f32).clamp(168.0, 220.0);
        let mut index = 0;
        while index < self.levels.len() {
            let end = (index + cols).min(self.levels.len());
            let count = end - index;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let used = count as f32 * card_w + (count - 1) as f32 * gap;
                ui.add_space(((ui.available_width() - used) / 2.0).max(0.0));
                for card_index in index..end {
                    if card_index > index {
                        ui.add_space(gap);
                    }
                    let title = self.levels[card_index].title;
                    if self.level_card(ui, card_index, title, card_w) {
                        self.start_level(card_index);
                    }
                }
            });
            ui.add_space(gap);
            index = end;
        }
    }

    fn level_card(&self, ui: &mut egui::Ui, index: usize, title: &str, width: f32) -> bool {
        let unlocked = index < self.progress.unlocked;
        let stars = self.progress.stars[index];
        let color = if unlocked {
            theme::level_color(index)
        } else {
            theme::MUTED
        };
        let (rect, response) = ui.allocate_exact_size(
            vec2(width, 176.0),
            if unlocked {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        if unlocked {
            response.clone().on_hover_cursor(CursorIcon::PointingHand);
        }
        let painter = ui.painter();
        let fill = if unlocked {
            theme::PANEL
        } else {
            Color32::from_rgb(14, 18, 24)
        };
        if response.hovered() && unlocked {
            painter.rect_filled(
                rect.expand(3.0),
                16.0,
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 28),
            );
        }
        painter.rect_filled(rect, 14.0, fill);
        painter.rect_stroke(
            rect,
            14.0,
            Stroke::new(
                1.2,
                if response.hovered() && unlocked {
                    color
                } else {
                    theme::LINE
                },
            ),
            egui::StrokeKind::Inside,
        );
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min + vec2(14.0, 0.0), vec2(width - 28.0, 3.0)),
            0.0,
            color,
        );

        let inner = rect.shrink(12.0);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::top_down(Align::LEFT)),
        );
        child.set_clip_rect(inner);
        child.add_space(6.0);
        child.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{:02}", index + 1))
                    .size(22.0)
                    .color(color)
                    .strong(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (glyph, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
                theme::level_glyph(ui.painter(), glyph, index, color);
            });
        });
        child.add(
            Label::new(
                RichText::new(title)
                    .size(14.5)
                    .color(if unlocked { theme::TEXT } else { theme::MUTED })
                    .strong(),
            )
            .wrap(),
        );
        let status = if !unlocked {
            "Закрыто"
        } else if stars > 0 {
            "Пройдено"
        } else {
            "Доступно"
        };
        child.label(RichText::new(status).size(12.5).color(color));
        theme::stars(&mut child, if unlocked { stars } else { 0 }, 6.5);
        response.clicked() && unlocked
    }

    fn menu_actions(&mut self, ui: &mut egui::Ui) {
        let continue_level = (self.progress.unlocked - 1).min(4);
        let label = if self.progress.stars[0] == 0 {
            "Начать обучение"
        } else {
            "Продолжить"
        };
        if theme::fill_button(ui, label, theme::MINT, theme::INK, 48.0).clicked() {
            self.start_level(continue_level);
        }
        ui.add_space(8.0);
        if theme::fill_button(ui, "Карта уровней", theme::CYAN, theme::INK, 44.0).clicked()
        {
            self.screen = Screen::Map;
        }
        ui.add_space(8.0);
        if theme::fill_button(ui, "Как играть", theme::PANEL_2, theme::TEXT, 40.0).clicked()
        {
            self.screen = Screen::Help;
        }
    }

    fn menu_progress(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            let total: u8 = self.progress.stars.iter().sum();
            ui.label(
                RichText::new(format!("Собрано звёзд: {total} из 15"))
                    .color(theme::AMBER)
                    .size(14.0),
            );
            if ui
                .add(theme::lab_button(
                    "Сбросить прогресс",
                    theme::INSET,
                    theme::MUTED,
                ))
                .clicked()
            {
                self.confirm_reset = true;
            }
        });
    }

    fn reset_dialog(&mut self, ctx: &egui::Context) {
        let modal = egui::Modal::new(egui::Id::unique("reset_progress"))
            .backdrop_color(Color32::from_black_alpha(170))
            .frame(theme::card())
            .show(ctx, |ui| {
                ui.set_min_width(280.0);
                ui.set_max_width(440.0);
                ui.label(
                    RichText::new("Сбросить прогресс?")
                        .size(22.0)
                        .color(theme::TEXT)
                        .strong(),
                );
                ui.add_space(6.0);
                theme::wrapped(
                    ui,
                    RichText::new("Удалить результаты всех уровней на этом компьютере?")
                        .size(15.0)
                        .color(theme::TEXT_DIM),
                );
                ui.add_space(14.0);
                let (yes, no) = ui
                    .horizontal_wrapped(|ui| {
                        let yes = ui.add(
                            theme::lab_button("Да, сбросить", theme::CORAL, theme::INK)
                                .min_size(vec2(0.0, 38.0)),
                        );
                        let no = ui.add(
                            theme::lab_button("Отмена", theme::PANEL_2, theme::TEXT)
                                .min_size(vec2(0.0, 38.0)),
                        );
                        (yes.clicked(), no.clicked())
                    })
                    .inner;
                (yes, no)
            });
        if modal.inner.0 {
            self.progress = Progress::default();
            self.confirm_reset = false;
            self.session = None;
        } else if modal.inner.1 || modal.should_close() {
            self.confirm_reset = false;
        }
    }

    fn show_map(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("map_top")
            .exact_size(64.0)
            .frame(theme::chassis())
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui
                        .add(theme::lab_button("В меню", theme::PANEL_2, theme::TEXT))
                        .clicked()
                    {
                        self.screen = Screen::Menu;
                    }
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Карта электролаборатории")
                                .size(20.0)
                                .color(theme::TEXT)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Пять испытаний, от лампы до щитка")
                                .size(12.5)
                                .color(theme::MUTED),
                        );
                    });
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                theme::backdrop(ui.painter(), ui.max_rect());
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(18.0);
                        theme::centered_column(ui, 880.0, |ui| {
                            theme::wrapped(
                                ui,
                                RichText::new(
                                    "Следующий уровень открывается после успешной проверки схемы",
                                )
                                .size(14.0)
                                .color(theme::MUTED),
                            );
                            ui.add_space(12.0);
                            for index in 0..self.levels.len() {
                                self.map_card(ui, index);
                                ui.add_space(10.0);
                            }
                        });
                        ui.add_space(16.0);
                    });
            });
    }

    fn map_card(&mut self, ui: &mut egui::Ui, index: usize) {
        let def = self.levels[index].clone();
        let unlocked = index < self.progress.unlocked;
        let stars = self.progress.stars[index];
        let color = if unlocked {
            theme::level_color(index)
        } else {
            theme::MUTED
        };
        theme::card().show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let (badge, _) = ui.allocate_exact_size(vec2(52.0, 52.0), Sense::hover());
                let painter = ui.painter();
                painter.rect_filled(
                    badge,
                    10.0,
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 36),
                );
                painter.text(
                    badge.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:02}", index + 1),
                    FontId::proportional(20.0),
                    color,
                );
                let action_w = 176.0;
                let text_w = (ui.available_width() - action_w - 8.0).max(80.0);
                ui.allocate_ui_with_layout(
                    vec2(text_w, 0.0),
                    Layout::top_down(Align::LEFT),
                    |ui| {
                        ui.set_width(text_w);
                        ui.add(
                            Label::new(
                                RichText::new(def.title)
                                    .size(20.0)
                                    .color(if unlocked { theme::TEXT } else { theme::MUTED })
                                    .strong(),
                            )
                            .wrap(),
                        );
                        ui.add(
                            Label::new(RichText::new(def.subtitle).size(13.5).color(theme::MUTED))
                                .wrap(),
                        );
                        if stars > 0 {
                            ui.horizontal(|ui| {
                                theme::stars(ui, stars, 7.0);
                                ui.label(
                                    RichText::new(format!("Результат: {stars}/3"))
                                        .size(13.0)
                                        .color(theme::AMBER),
                                );
                            });
                        }
                    },
                );
                ui.allocate_ui_with_layout(
                    vec2(action_w, 0.0),
                    Layout::top_down(Align::RIGHT),
                    |ui| {
                        let caption = if !unlocked {
                            "Закрыто"
                        } else if stars > 0 {
                            "Пройти ещё раз"
                        } else {
                            "Начать >"
                        };
                        let button = theme::lab_button(
                            caption,
                            if unlocked { color } else { theme::PANEL_2 },
                            if unlocked { theme::INK } else { theme::MUTED },
                        )
                        .min_size(vec2(0.0, 40.0))
                        .wrap();
                        if ui.add_enabled(unlocked, button).clicked() {
                            self.start_level(index);
                        }
                    },
                );
            });
        });
    }

    fn show_theory(&mut self, ui: &mut egui::Ui) {
        let def = self.levels[self.current_level].clone();
        let level = self.current_level;
        let back_to_play = self.theory_return == Screen::Play;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                theme::backdrop(ui.painter(), ui.max_rect());
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(28.0);
                        theme::centered_column(ui, 760.0, |ui| {
                            ui.label(
                                RichText::new(format!("УРОВЕНЬ {} · ТЕОРИЯ", level + 1))
                                    .size(13.0)
                                    .color(theme::level_color(level))
                                    .strong(),
                            );
                            ui.add(
                                Label::new(
                                    RichText::new(def.title)
                                        .size(34.0)
                                        .color(theme::TEXT)
                                        .strong(),
                                )
                                .wrap(),
                            );
                            ui.add(
                                Label::new(RichText::new(def.subtitle).size(16.0).color(theme::MUTED))
                                    .wrap(),
                            );
                            ui.add_space(16.0);
                            theme::card().show(ui, |ui| {
                                ui.label(
                                    RichText::new("Что нужно знать")
                                        .size(20.0)
                                        .color(theme::AMBER)
                                        .strong(),
                                );
                                ui.add_space(8.0);
                                for (number, item) in def.theory.iter().enumerate() {
                                    theory_row(ui, number + 1, item, theme::level_color(level));
                                    ui.add_space(8.0);
                                }
                                if level == 4 {
                                    ui.add_space(4.0);
                                    theme::alert(theme::COPPER).show(ui, |ui| {
                                        theme::wrapped(
                                            ui,
                                            RichText::new(
                                                "Безопасность: в реальной жизни работы в щитке выполняет только квалифицированный специалист.",
                                            )
                                            .size(14.5)
                                            .color(theme::COPPER)
                                            .strong(),
                                        );
                                    });
                                }
                            });
                            ui.add_space(18.0);
                            let forward = if back_to_play {
                                "К монтажному столу >"
                            } else {
                                "Вернуться"
                            };
                            if theme::fill_button(ui, forward, theme::MINT, theme::INK, 48.0).clicked() {
                                self.screen = self.theory_return;
                            }
                            if back_to_play {
                                ui.add_space(6.0);
                                ui.vertical_centered(|ui| {
                                    if ui
                                        .add(theme::lab_button(
                                            "Вернуться на карту",
                                            theme::PANEL_2,
                                            theme::TEXT,
                                        ))
                                        .clicked()
                                    {
                                        self.screen = Screen::Map;
                                    }
                                });
                            }
                        });
                        ui.add_space(24.0);
                    });
            });
    }

    fn show_help(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                theme::backdrop(ui.painter(), ui.max_rect());
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(16.0);
                        theme::centered_column(ui, 780.0, |ui| {
                            ui.label(
                                RichText::new("Как играть")
                                    .size(32.0)
                                    .color(theme::AMBER)
                                    .strong(),
                            );
                            ui.add(
                                Label::new(
                                    RichText::new("Коротко о монтажном столе")
                                        .size(15.0)
                                        .color(theme::MUTED),
                                )
                                .wrap(),
                            );
                            ui.add_space(8.0);
                            theme::card().show(ui, |ui| {
                                for (title, text) in [
                                    ("1. Установи детали", "Перетащи деталь из нижней панели в пустую рамку. Можно также выбрать деталь кликом, а затем нажать на рамку."),
                                    ("2. Проведи провода", "В режиме «Провод» нажми последовательно на две клеммы. Активная клемма подсвечивается белым кольцом."),
                                    ("3. Исправляй смело", "Ластик удаляет провод при клике на него. Правый клик по детали снимает её. Кнопка «Сброс» начинает уровень заново."),
                                    ("4. Управляй и измеряй", "Обычный клик по выключателю меняет его состояние. На уровнях 3 и 4 выбери прибор, а затем нажми на измеряемый элемент."),
                                    ("5. Проверь результат", "Выполни цели на панели справа и нажми зелёную кнопку проверки. Подсказки уменьшают итоговую награду, но не мешают пройти игру."),
                                ] {
                                    ui.label(RichText::new(title).size(18.0).color(theme::CYAN).strong());
                                    ui.add(Label::new(RichText::new(text).size(15.0).color(theme::TEXT)).wrap());
                                    ui.add_space(8.0);
                                }
                                theme::alert(theme::COPPER).show(ui, |ui| {
                                    theme::wrapped(
                                        ui,
                                        RichText::new(
                                            "Все аварии виртуальные. Не повторяйте действия с реальными розетками, проводами или электрощитком.",
                                        )
                                        .size(14.5)
                                        .color(theme::COPPER)
                                        .strong(),
                                    );
                                });
                            });
                            ui.add_space(10.0);
                            if theme::fill_button(ui, "Понятно", theme::MINT, theme::INK, 44.0).clicked() {
                                self.screen = Screen::Menu;
                            }
                        });
                        ui.add_space(24.0);
                    });
            });
    }

    fn show_play(&mut self, ui: &mut egui::Ui) {
        let index = self.current_level;
        let def = self.levels[index].clone();
        let mut session = self
            .session
            .take()
            .unwrap_or_else(|| LevelSession::new(index, &def));
        let mut dropped_part = None;
        let window_w = ui.available_width();
        let side = side_width(window_w);
        let dock = dock_height(window_w, index);

        egui::Panel::top("play_top")
            .exact_size(68.0)
            .frame(theme::chassis())
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui
                        .add(theme::lab_button("К карте", theme::PANEL_2, theme::TEXT))
                        .clicked()
                    {
                        self.screen = Screen::Map;
                    }
                    let reserve = 330.0;
                    let text_w = (ui.available_width() - reserve).max(48.0);
                    ui.allocate_ui_with_layout(
                        vec2(text_w, 46.0),
                        Layout::top_down(Align::LEFT),
                        |ui| {
                            ui.set_width(text_w);
                            ui.add(
                                Label::new(
                                    RichText::new(format!("{:02}   {}", index + 1, def.title))
                                        .size(18.0)
                                        .color(theme::TEXT)
                                        .strong(),
                                )
                                .truncate(),
                            );
                            ui.add(
                                Label::new(
                                    RichText::new(def.subtitle).size(12.5).color(theme::MUTED),
                                )
                                .truncate(),
                            );
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(theme::lab_button("Сброс", theme::PANEL_2, theme::TEXT))
                            .clicked()
                        {
                            session = LevelSession::new(index, &def);
                        }
                        if ui
                            .add(theme::lab_button("Подсказка", theme::PANEL_2, theme::AMBER))
                            .clicked()
                        {
                            let hint_index = session.hint_cursor.min(def.hints.len() - 1);
                            if session.circuit == 1 && index == 2 {
                                session.hint_cursor = def.hints.len() - 1;
                            }
                            let hint_index = if session.circuit == 1 && index == 2 {
                                def.hints.len() - 1
                            } else {
                                hint_index
                            };
                            session.hint_cursor = (hint_index + 1).min(def.hints.len());
                            session.hints_used += 1;
                            session.set_feedback(FeedbackKind::Info, def.hints[hint_index]);
                        }
                        if ui
                            .add(theme::lab_button("Теория", theme::PANEL_2, theme::CYAN))
                            .clicked()
                        {
                            self.theory_return = Screen::Play;
                            self.screen = Screen::Theory;
                        }
                    });
                });
            });

        egui::Panel::bottom("play_controls")
            .exact_size(dock)
            .frame(theme::chassis())
            .show_separator_line(false)
            .show(ui, |ui| {
                let scroll_h = (ui.available_height() - 52.0).max(32.0);
                ScrollArea::vertical()
                    .id_salt("dock_scroll")
                    .max_height(scroll_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !def.palette.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new("ДЕТАЛИ")
                                        .size(11.0)
                                        .color(theme::MUTED)
                                        .strong(),
                                );
                                for part in def.palette {
                                    let selected = session.selected_part == Some(*part);
                                    let (fill, fg) = if selected {
                                        (theme::AMBER, theme::INK)
                                    } else {
                                        (theme::PANEL_2, theme::TEXT)
                                    };
                                    let response = ui.add(
                                        theme::lab_button(part_short_label(*part), fill, fg)
                                            .sense(Sense::click_and_drag()),
                                    );
                                    if response.clicked() || response.drag_started() {
                                        session.selected_part = Some(*part);
                                    }
                                    if response.drag_stopped() {
                                        dropped_part = Some(*part);
                                    }
                                }
                            });
                            ui.add(
                                Label::new(
                                    RichText::new(
                                        "Перетащи деталь или выбери её и нажми на пустой слот",
                                    )
                                    .size(12.0)
                                    .color(theme::MUTED),
                                )
                                .wrap(),
                            );
                            ui.add_space(4.0);
                        }
                        ui.horizontal_wrapped(|ui| {
                            if index != 3 {
                                tool_button(ui, &mut session, Tool::Wire, "Провод");
                                tool_button(ui, &mut session, Tool::Erase, "Ластик");
                            }
                            if index == 2 {
                                tool_button(ui, &mut session, Tool::Ammeter, "Амперметр");
                                tool_button(ui, &mut session, Tool::Voltmeter, "Вольтметр");
                            }
                            if index == 3 {
                                tool_button(ui, &mut session, Tool::Voltmeter, "Вольтметр");
                                tool_button(ui, &mut session, Tool::Continuity, "Целостность / Ом");
                                tool_button(ui, &mut session, Tool::Replace, "Заменить деталь");
                                let power_label = if session.power_connected {
                                    "Отключить источник"
                                } else {
                                    "Подключить источник"
                                };
                                if ui
                                    .add(theme::lab_button(
                                        power_label,
                                        theme::PANEL_2,
                                        theme::TEXT,
                                    ))
                                    .clicked()
                                {
                                    self.toggle_diagnostic_power(&mut session);
                                }
                            }
                            if index == 4 {
                                let power_label = if session.power_connected {
                                    "Отключить питание"
                                } else {
                                    "Подать питание"
                                };
                                if ui
                                    .add(theme::lab_button(
                                        power_label,
                                        theme::PANEL_2,
                                        theme::TEXT,
                                    ))
                                    .clicked()
                                {
                                    self.toggle_panel_power(&def, &mut session);
                                }
                                if ui
                                    .add(theme::lab_button(
                                        "Сбросить аварию",
                                        theme::PANEL_2,
                                        theme::TEXT,
                                    ))
                                    .clicked()
                                {
                                    self.reset_panel_event(&mut session);
                                }
                                if ui
                                    .add(theme::lab_button(
                                        "Нормальная нагрузка",
                                        theme::PANEL_2,
                                        theme::TEXT,
                                    ))
                                    .clicked()
                                {
                                    self.panel_normal(&def, &mut session);
                                }
                                if ui
                                    .add(theme::lab_button(
                                        "Перегрузка 25 А",
                                        theme::PANEL_2,
                                        theme::TEXT,
                                    ))
                                    .clicked()
                                {
                                    self.panel_overload(&def, &mut session);
                                }
                                if ui
                                    .add(theme::lab_button(
                                        "Утечка 30 мА",
                                        theme::PANEL_2,
                                        theme::TEXT,
                                    ))
                                    .clicked()
                                {
                                    self.panel_leakage(&def, &mut session);
                                }
                            }
                        });
                        ui.add_space(2.0);
                        ui.add(
                            Label::new(
                                RichText::new(tool_hint(session.tool))
                                    .size(12.0)
                                    .color(theme::MUTED),
                            )
                            .wrap(),
                        );
                        ui.add_space(6.0);
                    });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Ошибки: {}   ·   Подсказки: {}",
                            session.mistakes, session.hints_used
                        ))
                        .size(12.5)
                        .color(theme::TEXT_DIM),
                    );
                    let check_text = if index == 2 && session.circuit == 0 {
                        "Принять последовательную схему"
                    } else {
                        "Проверить задание"
                    };
                    if ui
                        .add(
                            theme::primary_button(check_text, theme::MINT)
                                .min_size(vec2(0.0, 40.0)),
                        )
                        .clicked()
                    {
                        self.check_level(&def, &mut session);
                    }
                });
            });

        let current_reading = reading(&def, &session);
        egui::Panel::right("objectives").resizable(false)
            .exact_size(side)
            .frame(theme::chassis())
            .show_separator_line(false)
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("tasks_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("Задания")
                                    .size(20.0)
                                    .color(theme::AMBER)
                                    .strong(),
                            );
                            if index == 2 {
                                ui.label(
                                    RichText::new(format!("Этап {}/2", session.circuit + 1))
                                        .color(theme::CYAN)
                                        .strong(),
                                );
                            }
                        });
                        ui.add_space(6.0);
                        show_feedback(ui, &session);
                        for (text, complete) in objectives(&def, &session, &current_reading) {
                            theme::task_row(ui, complete, &text);
                            ui.add_space(3.0);
                        }
                        ui.add_space(6.0);
                        show_reading(ui, index, &session, &current_reading);

                        if index == 4
                            && let Some(selected) = session.selected_component
                        {
                            let component = &def.components[selected];
                            if !component.rating_options.is_empty() {
                                ui.add_space(8.0);
                                theme::inset().show(ui, |ui| {
                                    ui.label(
                                        RichText::new(component.name)
                                            .size(15.0)
                                            .color(theme::CYAN)
                                            .strong(),
                                    );
                                    ui.add(
                                        Label::new(
                                            RichText::new(
                                                "Выбери номинал в панели справа от схемы:",
                                            )
                                            .size(13.0)
                                            .color(theme::TEXT_DIM),
                                        )
                                        .wrap(),
                                    );
                                    let selected_text = session.components[selected]
                                        .rating
                                        .map_or("Не выбран".to_owned(), |rating| {
                                            format!("{rating} {}", component.rating_unit)
                                        });
                                    ComboBox::from_id_salt(("rating", selected))
                                        .width(ui.available_width().min(240.0))
                                        .selected_text(selected_text)
                                        .show_ui(ui, |ui| {
                                            for option in component.rating_options {
                                                let label =
                                                    format!("{option} {}", component.rating_unit);
                                                if ui
                                                    .selectable_label(
                                                        session.components[selected].rating
                                                            == Some(*option),
                                                        label,
                                                    )
                                                    .clicked()
                                                {
                                                    if session.power_connected
                                                        || session.panel_event.is_some()
                                                    {
                                                        session.mistake(
                                                            "Сначала отключи питание и сбрось аварию. Номиналы под напряжением не меняют.",
                                                        );
                                                    } else {
                                                        session.components[selected].rating =
                                                            Some(*option);
                                                    }
                                                }
                                            }
                                        });
                                });
                            }
                        }

                        if let Some(last) = &session.last_reading {
                            ui.add_space(8.0);
                            theme::inset().show(ui, |ui| {
                                ui.label(
                                    RichText::new("ПРИБОР / ИСПЫТАНИЕ")
                                        .size(11.0)
                                        .color(theme::CYAN)
                                        .strong(),
                                );
                                ui.add(Label::new(RichText::new(last).size(14.0).color(theme::TEXT)).wrap());
                            });
                        }

                        if index == 4 {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("МАРКИРОВКА")
                                    .size(11.0)
                                    .color(theme::MUTED)
                                    .strong(),
                            );
                            ui.horizontal_wrapped(|ui| {
                                swatch(ui, theme::PHASE, "L фаза");
                                swatch(ui, theme::NEUTRAL, "N ноль");
                                swatch(ui, theme::PROTECTIVE, "PE защита");
                            });
                        }

                        ui.add_space(8.0);
                        egui::CollapsingHeader::new(
                            RichText::new(format!("Провода · {}", session.wires.len()))
                                .size(14.0)
                                .color(theme::CYAN)
                                .strong(),
                        )
                        .default_open((1..=4).contains(&session.wires.len()))
                        .show(ui, |ui| {
                            if session.wires.is_empty() {
                                ui.add(
                                    Label::new(
                                        RichText::new("Проводов пока нет")
                                            .size(12.5)
                                            .color(theme::MUTED),
                                    )
                                    .wrap(),
                                );
                            }
                            let mut remove = None;
                            for (wire_index, wire) in session.wires.iter().enumerate() {
                                let name = wire_name(&def, *wire);
                                ui.horizontal_top(|ui| {
                                    let width = ui.available_width();
                                    let text_w = (width - 40.0).max(36.0);
                                    ui.allocate_ui_with_layout(
                                        vec2(text_w, 0.0),
                                        Layout::top_down(Align::LEFT),
                                        |ui| {
                                            ui.set_width(text_w);
                                            ui.add(
                                                Label::new(
                                                    RichText::new(name)
                                                        .size(12.0)
                                                        .color(theme::TEXT_DIM),
                                                )
                                                .wrap(),
                                            );
                                        },
                                    );
                                    if ui
                                        .add(
                                            theme::lab_button("×", theme::PANEL_2, theme::TEXT)
                                                .min_size(vec2(28.0, 28.0)),
                                        )
                                        .on_hover_text("Удалить провод")
                                        .clicked()
                                    {
                                        remove = Some(wire_index);
                                    }
                                });
                                ui.add_space(2.0);
                            }
                            if let Some(wire_index) = remove
                                && let Err(message) = session.remove_wire(wire_index)
                            {
                                session.set_feedback(FeedbackKind::Warning, message);
                            }
                        });
                        ui.add_space(8.0);
                    });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(12, 16, 20))
                    .inner_margin(8.0),
            )
            .show(ui, |ui| {
                let output = board::show(ui, &def, &session, &current_reading, self.time);
                if let Some(action) = output.action {
                    self.handle_board_action(&def, &mut session, action);
                }
                if let Some(part) = dropped_part
                    && let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
                    && let Some((component, _)) =
                        output.slots.iter().find(|(_, rect)| rect.contains(pointer))
                {
                    match session.place_part(&def, *component, part) {
                        Ok(()) => session.set_feedback(
                            FeedbackKind::Success,
                            format!("{} установлено.", part.label()),
                        ),
                        Err(message) => session.set_feedback(FeedbackKind::Warning, message),
                    }
                }
            });

        self.session = Some(session);
        ui.ctx().request_repaint_after(Duration::from_millis(33));
    }

    fn show_result(&mut self, ui: &mut egui::Ui) {
        let def = self.levels[self.current_level].clone();
        let level = self.current_level;
        let stars = self.result_stars;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                theme::backdrop(ui.painter(), ui.max_rect());
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(36.0);
                        theme::centered_column(ui, 680.0, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("ЗАДАНИЕ ВЫПОЛНЕНО")
                                        .size(14.0)
                                        .color(theme::MINT)
                                        .strong(),
                                );
                            });
                            ui.add(
                                Label::new(
                                    RichText::new(def.title)
                                        .size(36.0)
                                        .color(theme::TEXT)
                                        .strong(),
                                )
                                .wrap(),
                            );
                            ui.add_space(8.0);
                            theme::stars_centered(ui, stars, 26.0);
                            ui.add_space(6.0);
                            ui.vertical_centered(|ui| {
                                ui.add(
                                    Label::new(
                                        RichText::new(match stars {
                                            3 => "Отлично: точная работа без подсказок!",
                                            2 => "Хорошо: схема собрана и проверена!",
                                            _ => "Готово: новый раздел электротехники открыт!",
                                        })
                                        .size(16.0)
                                        .color(theme::MUTED),
                                    )
                                    .wrap(),
                                );
                            });
                            ui.add_space(16.0);
                            theme::card().show(ui, |ui| {
                                ui.label(
                                    RichText::new("Что ты узнал")
                                        .size(18.0)
                                        .color(theme::CYAN)
                                        .strong(),
                                );
                                ui.add_space(6.0);
                                theme::wrapped(
                                    ui,
                                    RichText::new(level_summary(level))
                                        .size(15.5)
                                        .color(theme::TEXT),
                                );
                            });
                            ui.add_space(18.0);
                            if level < 4
                                && theme::fill_button(ui, "Следующий уровень >", theme::MINT, theme::INK, 48.0)
                                    .clicked()
                            {
                                self.start_level(level + 1);
                            }
                            if level == 4 {
                                ui.vertical_centered(|ui| {
                                    ui.add(
                                        Label::new(
                                            RichText::new(
                                                "Все пять уровней пройдены. Электролаборатория завершена!",
                                            )
                                            .size(17.0)
                                            .color(theme::AMBER)
                                            .strong(),
                                        )
                                        .wrap(),
                                    );
                                });
                            }
                            ui.add_space(8.0);
                            if theme::fill_button(ui, "Карта уровней", theme::CYAN, theme::INK, 42.0).clicked() {
                                self.screen = Screen::Map;
                            }
                            ui.add_space(6.0);
                            ui.vertical_centered(|ui| {
                                if ui
                                    .add(theme::lab_button(
                                        "Пройти этот уровень ещё раз",
                                        theme::PANEL_2,
                                        theme::TEXT,
                                    ))
                                    .clicked()
                                {
                                    self.start_level(level);
                                }
                            });
                        });
                        ui.add_space(28.0);
                    });
            });
    }
}

impl eframe::App for ElectroApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.progress);
    }

    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(2)
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.time = ui.input(|input| input.time) as f32;
        match self.screen {
            Screen::Menu => self.show_menu(ui),
            Screen::Map => self.show_map(ui),
            Screen::Theory => self.show_theory(ui),
            Screen::Play => self.show_play(ui),
            Screen::Result => self.show_result(ui),
            Screen::Help => self.show_help(ui),
        }
        if let Some(capture) = &mut self.capture {
            capture.tick(ui.ctx());
        } else {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }
    }
}

fn side_width(window_w: f32) -> f32 {
    if window_w < 1240.0 {
        292.0
    } else if window_w < 1600.0 {
        340.0
    } else {
        380.0
    }
}

fn dock_height(window_w: f32, level: usize) -> f32 {
    let crowded = matches!(level, 2..=4);
    if window_w < 1280.0 {
        if level == 4 {
            268.0
        } else if crowded {
            240.0
        } else {
            220.0
        }
    } else if window_w < 1500.0 {
        if crowded { 236.0 } else { 220.0 }
    } else if crowded {
        210.0
    } else {
        196.0
    }
}

fn theory_row(ui: &mut egui::Ui, number: usize, text: &str, color: Color32) {
    let width = ui.available_width();
    ui.horizontal_top(|ui| {
        let (badge, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
        ui.painter().circle_filled(badge.center(), 11.0, color);
        ui.painter().text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            number.to_string(),
            FontId::proportional(12.0),
            theme::INK,
        );
        let text_w = (width - 34.0).max(40.0);
        ui.allocate_ui_with_layout(vec2(text_w, 0.0), Layout::top_down(Align::LEFT), |ui| {
            ui.set_width(text_w);
            ui.add(Label::new(RichText::new(text).size(15.5).color(theme::TEXT)).wrap());
        });
    });
}

fn tool_button(ui: &mut egui::Ui, session: &mut LevelSession, tool: Tool, label: &str) {
    let selected = session.tool == tool;
    let (fill, fg) = if selected {
        (theme::CYAN, theme::INK)
    } else {
        (theme::PANEL_2, theme::TEXT)
    };
    if ui.add(theme::lab_button(label, fill, fg)).clicked() {
        session.tool = tool;
        session.selected_terminal = None;
    }
}

fn tool_hint(tool: Tool) -> &'static str {
    match tool {
        Tool::Wire => {
            "Режим: клик по двум клеммам создаёт провод; клик по выключателю меняет контакт."
        }
        Tool::Erase => "Режим: нажми на провод, который нужно удалить.",
        Tool::Ammeter => "Амперметр: нажми на лампу, через которую нужно измерить ток.",
        Tool::Voltmeter => "Вольтметр: нажми на элемент, на котором нужно измерить напряжение.",
        Tool::Continuity => "Проверка целостности: источник должен быть отключён.",
        Tool::Replace => "Замена: источник должен быть отключён, деталь сначала проверена.",
    }
}

fn show_feedback(ui: &mut egui::Ui, session: &LevelSession) {
    let Some(feedback) = &session.message else {
        return;
    };
    let color = match feedback.kind {
        FeedbackKind::Info => theme::CYAN,
        FeedbackKind::Success => theme::MINT,
        FeedbackKind::Warning => theme::AMBER,
        FeedbackKind::Error => theme::CORAL,
    };
    theme::alert(color).show(ui, |ui| {
        theme::wrapped(
            ui,
            RichText::new(&feedback.text)
                .size(13.5)
                .color(color)
                .strong(),
        );
    });
    ui.add_space(8.0);
}

fn show_reading(
    ui: &mut egui::Ui,
    level: usize,
    session: &LevelSession,
    reading: &ElectricalReading,
) {
    let (title, color) = if level == 4 {
        match session.panel_event {
            Some(PanelEvent::Overload) => ("АВТОМАТ ОТКЛЮЧИЛ ЦЕПЬ", theme::AMBER),
            Some(PanelEvent::Leakage) if session.has_flag("leakage_ok") => {
                ("УЗО ОТКЛЮЧИЛО УТЕЧКУ", theme::MINT)
            }
            Some(PanelEvent::Leakage) => ("ПРОВЕРЬ УСТАВКУ УЗО", theme::AMBER),
            None if session.power_connected && session.has_flag("normal_ok") => {
                ("НОРМАЛЬНЫЙ РЕЖИМ", theme::MINT)
            }
            None if session.power_connected => ("ПИТАНИЕ ВКЛЮЧЕНО", theme::AMBER),
            None => ("ЩИТОК ОБЕСТОЧЕН", theme::MUTED),
        }
    } else {
        match reading.status {
            CircuitStatus::Incomplete => ("СХЕМА НЕ ЗАВЕРШЕНА", theme::MUTED),
            CircuitStatus::Open => ("ТОКА НЕТ", theme::AMBER),
            CircuitStatus::Short => ("КОРОТКОЕ ЗАМЫКАНИЕ", theme::CORAL),
            CircuitStatus::Running => ("ЦЕПЬ РАБОТАЕТ", theme::MINT),
            CircuitStatus::Overload => ("ПЕРЕГРУЗКА", theme::CORAL),
            CircuitStatus::Dim => ("СЛАБЫЙ ТОК", theme::AMBER),
        }
    };
    theme::inset().show(ui, |ui| {
        ui.label(
            RichText::new("СТАТУС ЦЕПИ")
                .size(11.0)
                .color(theme::MUTED)
                .strong(),
        );
        ui.add(Label::new(RichText::new(title).size(14.0).color(color).strong()).wrap());
        if level != 4 {
            ui.add(Label::new(RichText::new(&reading.detail).size(13.5).color(theme::TEXT)).wrap());
            if let Some(total_current) = reading.total_current_a {
                ui.add(
                    Label::new(
                        RichText::new(format!(
                            "Общий ток источника: {} А",
                            crate::simulation::decimal(total_current, 2)
                        ))
                        .size(12.5)
                        .color(theme::MUTED),
                    )
                    .wrap(),
                );
            }
        }
        if level == 4 && session.power_connected {
            ui.add(
                Label::new(
                    RichText::new("Питание включено. Перед изменением схемы отключи его.")
                        .size(12.5)
                        .color(theme::AMBER),
                )
                .wrap(),
            );
        }
    });
}

fn swatch(ui: &mut egui::Ui, color: Color32, text: &str) {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_owned(), FontId::proportional(12.0), theme::TEXT_DIM);
    let (rect, _) = ui.allocate_exact_size(vec2(16.0 + galley.size().x, 18.0), Sense::hover());
    ui.painter()
        .circle_filled(rect.left_center() + vec2(5.0, 0.0), 4.5, color);
    ui.painter().galley(
        pos2(rect.left() + 14.0, rect.center().y - galley.size().y * 0.5),
        galley,
        theme::TEXT_DIM,
    );
}

fn objectives(
    def: &LevelDef,
    session: &LevelSession,
    reading: &ElectricalReading,
) -> Vec<(String, bool)> {
    match session.level {
        0 => vec![
            (
                "Установи выключатель и лампу".to_owned(),
                session.missing_parts(def).is_empty(),
            ),
            (
                "Соедини три участка цепи".to_owned(),
                session.wiring_matches(def),
            ),
            (
                "Замкни выключатель и зажги лампу".to_owned(),
                reading.status == CircuitStatus::Running,
            ),
        ],
        1 => vec![
            (
                "Рассчитай и установи резистор 300 Ом".to_owned(),
                session.missing_parts(def).is_empty(),
            ),
            (
                "Подключи светодиод с соблюдением полярности".to_owned(),
                session.wiring_matches(def),
            ),
            (
                "Получи ток около 10 мА".to_owned(),
                reading.status == CircuitStatus::Running,
            ),
        ],
        2 => {
            if session.circuit == 0 {
                vec![
                    (
                        "Собери последовательную цепь".to_owned(),
                        session.wiring_matches(def),
                    ),
                    (
                        "Измерь ток через лампу амперметром".to_owned(),
                        session.has_flag("current_measured"),
                    ),
                    (
                        "Измерь напряжение на лампе вольтметром".to_owned(),
                        session.has_flag("voltage_measured"),
                    ),
                ]
            } else {
                vec![
                    ("Последовательная схема исследована".to_owned(), true),
                    (
                        "Собери две параллельные ветви".to_owned(),
                        session.wiring_matches(def),
                    ),
                    (
                        "Сними показания обоими приборами".to_owned(),
                        session.has_flag("current_measured")
                            && session.has_flag("voltage_measured"),
                    ),
                ]
            }
        }
        3 => vec![
            (
                "Проверь напряжение источника".to_owned(),
                session.has_flag("source_voltage"),
            ),
            (
                "Отключи источник и проверь предохранитель".to_owned(),
                session.has_flag("fuse_tested"),
            ),
            (
                "Проверь сопротивление лампы".to_owned(),
                session.has_flag("lamp_tested"),
            ),
            (
                "Замени короткозамкнутую лампу".to_owned(),
                session.has_flag("lamp_replaced"),
            ),
            (
                "Замени перегоревший предохранитель".to_owned(),
                session.has_flag("fuse_replaced"),
            ),
            (
                "Включи исправную цепь".to_owned(),
                reading.status == CircuitStatus::Running,
            ),
        ],
        4 => vec![
            (
                "Установи все аппараты и нагрузки".to_owned(),
                session.missing_parts(def).is_empty(),
            ),
            (
                "Собери L, N и отдельный PE без ошибок".to_owned(),
                session.wiring_matches(def) && !session.has_neutral_pe_bridge(def),
            ),
            (
                "Выбери номиналы: 20 А · 30 мА · 10 А · 16 А".to_owned(),
                session.ratings_match(def),
            ),
            (
                "Проверь нормальную нагрузку".to_owned(),
                session.has_flag("normal_ok"),
            ),
            (
                "Перегрузку должен отключить автомат розеток".to_owned(),
                session.has_flag("overload_ok"),
            ),
            (
                "Утечку 30 мА должно отключить УЗО".to_owned(),
                session.has_flag("leakage_ok"),
            ),
        ],
        _ => Vec::new(),
    }
}

fn wire_name(def: &LevelDef, wire: Wire) -> String {
    fn endpoint(def: &LevelDef, terminal: TerminalRef) -> String {
        let component = &def.components[terminal.component];
        let terminal = &component.terminals[terminal.terminal];
        format!("{} ({})", component.name, terminal.label)
    }
    format!("{} > {}", endpoint(def, wire.from), endpoint(def, wire.to))
}

fn level_summary(level: usize) -> &'static str {
    match level {
        0 => {
            "Ток существует в замкнутой цепи. Выключатель разрывает путь тока, а лампа превращает электрическую энергию в свет и тепло."
        }
        1 => {
            "Закон Ома связывает напряжение, ток и сопротивление. Резистор ограничивает ток через светодиод: R = (5 − 2) / 0,01 = 300 Ом."
        }
        2 => {
            "При последовательном соединении ток одинаков, а напряжение делится. При параллельном напряжение одинаково, а токи ветвей складываются. Поэтому во втором случае лампы ярче."
        }
        3 => {
            "Предохранитель разрывает цепь при опасном токе. Перед измерением сопротивления источник отключают. Сначала устраняют причину замыкания и только потом меняют предохранитель."
        }
        _ => {
            "Автомат и УЗО выполняют разные задачи: автомат реагирует на сверхток, УЗО — на утечку. Защитный PE не объединяют с рабочим N после УЗО."
        }
    }
}
