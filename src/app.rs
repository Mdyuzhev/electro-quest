use std::{sync::Arc, time::Duration};

use eframe::egui::{
    self, Align, Button, Color32, ComboBox, FontData, FontDefinitions, FontFamily, Frame, Layout,
    RichText, ScrollArea, Sense, Stroke, TextStyle, Vec2,
};
use serde::{Deserialize, Serialize};

use crate::board::{self, part_short_label};
use crate::levels::all_levels;
use crate::model::{FeedbackKind, LevelDef, LevelSession, TerminalRef, Tool, Wire};
use crate::simulation::{CircuitStatus, ElectricalReading, reading};

const BACKGROUND: Color32 = Color32::from_rgb(11, 19, 38);
const PANEL: Color32 = Color32::from_rgb(22, 34, 57);
const ACCENT: Color32 = Color32::from_rgb(255, 203, 74);
const CYAN: Color32 = Color32::from_rgb(83, 202, 231);
const GREEN: Color32 = Color32::from_rgb(80, 202, 137);
const MUTED: Color32 = Color32::from_rgb(157, 179, 203);

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
}

impl ElectroApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_appearance(&cc.egui_ctx);
        let mut progress: Progress = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        progress.unlocked = progress.unlocked.clamp(1, 5);
        Self {
            levels: all_levels(),
            progress,
            screen: Screen::Menu,
            current_level: 0,
            session: None,
            theory_return: Screen::Play,
            result_stars: 0,
            time: 0.0,
            confirm_reset: false,
        }
    }

    fn start_level(&mut self, index: usize) {
        self.current_level = index;
        self.session = Some(LevelSession::new(index, &self.levels[index]));
        self.theory_return = Screen::Play;
        self.screen = Screen::Theory;
    }

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(Frame::default().fill(BACKGROUND))
            .show(ui, |ui| {
                ui.add_space(38.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("УЧЕБНАЯ ЛАБОРАТОРИЯ")
                            .size(15.0)
                            .color(CYAN)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("ЭЛЕКТРОКВЕСТ")
                            .size(48.0)
                            .color(ACCENT)
                            .strong(),
                    );
                    ui.label(
                        RichText::new("От первой лампочки до защитного щитка")
                            .size(21.0)
                            .color(Color32::WHITE),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("Интерактивная игра по электротехнике · 7–9 классы")
                            .size(15.0)
                            .color(MUTED),
                    );
                });

                ui.add_space(30.0);
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 740.0).max(0.0) / 2.0);
                    for (index, def) in self.levels.iter().enumerate() {
                        let unlocked = index < self.progress.unlocked;
                        let color = if unlocked {
                            level_color(index)
                        } else {
                            Color32::from_rgb(62, 74, 94)
                        };
                        ui.group(|ui| {
                            ui.set_width(132.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(format!("{:02}", index + 1))
                                        .size(24.0)
                                        .color(color)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(def.title)
                                        .size(13.0)
                                        .color(if unlocked { Color32::WHITE } else { MUTED }),
                                );
                                ui.label(
                                    RichText::new(if index < self.progress.unlocked {
                                        if self.progress.stars[index] > 0 {
                                            "Пройдено"
                                        } else {
                                            "Доступно"
                                        }
                                    } else {
                                        "Закрыто"
                                    })
                                    .size(12.0)
                                    .color(color),
                                );
                            });
                        });
                    }
                });

                ui.add_space(32.0);
                ui.vertical_centered(|ui| {
                    let continue_level = (self.progress.unlocked - 1).min(4);
                    if ui
                        .add_sized(
                            [300.0, 48.0],
                            primary_button(
                                if self.progress.stars[0] == 0 {
                                    "Начать обучение"
                                } else {
                                    "Продолжить"
                                },
                                GREEN,
                            ),
                        )
                        .clicked()
                    {
                        self.start_level(continue_level);
                    }
                    ui.add_space(8.0);
                    if ui
                        .add_sized([300.0, 42.0], primary_button("Карта уровней", CYAN))
                        .clicked()
                    {
                        self.screen = Screen::Map;
                    }
                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            [300.0, 38.0],
                            Button::new(RichText::new("Как играть").size(16.0)),
                        )
                        .clicked()
                    {
                        self.screen = Screen::Help;
                    }
                });

                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    let total: u8 = self.progress.stars.iter().sum();
                    ui.label(
                        RichText::new(format!("Собрано звёзд: {total} из 15"))
                            .color(ACCENT)
                            .size(14.0),
                    );
                    if !self.confirm_reset {
                        if ui
                            .add(Button::new(
                                RichText::new("Сбросить прогресс").size(12.0).color(MUTED),
                            ))
                            .clicked()
                        {
                            self.confirm_reset = true;
                        }
                    } else {
                        ui.label("Удалить результаты всех уровней на этом компьютере?");
                        ui.horizontal(|ui| {
                            if ui.button("Да, сбросить").clicked() {
                                self.progress = Progress::default();
                                self.confirm_reset = false;
                                self.session = None;
                            }
                            if ui.button("Отмена").clicked() {
                                self.confirm_reset = false;
                            }
                        });
                    }
                });

                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(
                            "Учебная симуляция. Не используйте её как руководство для работы с реальной сетью 230 В.",
                        )
                        .size(12.0)
                        .color(MUTED),
                    );
                });
            });
    }

    fn show_map(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("map_top")
            .exact_size(70.0)
            .frame(Frame::default().fill(PANEL))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    if ui.button("< Главное меню").clicked() {
                        self.screen = Screen::Menu;
                    }
                    ui.separator();
                    ui.heading("Карта электролаборатории");
                });
            });
        egui::CentralPanel::default()
            .frame(Frame::default().fill(BACKGROUND))
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(18.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Пройди все пять испытаний")
                                .size(25.0)
                                .color(Color32::WHITE),
                        );
                        ui.label(
                            RichText::new(
                                "Следующий уровень открывается после успешной проверки схемы",
                            )
                            .color(MUTED),
                        );
                    });
                    ui.add_space(18.0);
                    for index in 0..self.levels.len() {
                        let def = self.levels[index].clone();
                        let unlocked = index < self.progress.unlocked;
                        let stars = self.progress.stars[index];
                        Frame::default()
                            .fill(if unlocked {
                                Color32::from_rgb(25, 42, 67)
                            } else {
                                Color32::from_rgb(20, 28, 44)
                            })
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(6.0);
                                    ui.label(
                                        RichText::new(format!("{}", index + 1))
                                            .size(36.0)
                                            .color(if unlocked {
                                                level_color(index)
                                            } else {
                                                Color32::from_rgb(74, 86, 105)
                                            })
                                            .strong(),
                                    );
                                    ui.add_space(14.0);
                                    ui.vertical(|ui| {
                                        ui.set_width(610.0);
                                        ui.label(
                                            RichText::new(def.title)
                                                .size(22.0)
                                                .color(if unlocked {
                                                    Color32::WHITE
                                                } else {
                                                    MUTED
                                                })
                                                .strong(),
                                        );
                                        ui.label(
                                            RichText::new(def.subtitle).size(14.0).color(MUTED),
                                        );
                                        if stars > 0 {
                                            ui.label(
                                                RichText::new(format!("Результат: {stars}/3"))
                                                    .size(16.0)
                                                    .color(ACCENT),
                                            );
                                        }
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let label = if !unlocked {
                                            "Закрыто"
                                        } else if stars > 0 {
                                            "Пройти ещё раз"
                                        } else {
                                            "Начать >"
                                        };
                                        if ui
                                            .add_sized(
                                                [150.0, 42.0],
                                                Button::new(label).fill(if unlocked {
                                                    level_color(index)
                                                } else {
                                                    Color32::from_rgb(55, 65, 80)
                                                }),
                                            )
                                            .clicked()
                                            && unlocked
                                        {
                                            self.start_level(index);
                                        }
                                    });
                                });
                            });
                        ui.add_space(10.0);
                    }
                });
            });
    }

    fn show_theory(&mut self, ui: &mut egui::Ui) {
        let def = self.levels[self.current_level].clone();
        egui::CentralPanel::default()
            .frame(Frame::default().fill(BACKGROUND))
            .show(ui, |ui| {
                ui.add_space(35.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("УРОВЕНЬ {} · ТЕОРИЯ", self.current_level + 1))
                            .size(14.0)
                            .color(level_color(self.current_level))
                            .strong(),
                    );
                    ui.label(RichText::new(def.title).size(36.0).color(Color32::WHITE).strong());
                    ui.label(RichText::new(def.subtitle).size(17.0).color(MUTED));
                });
                ui.add_space(25.0);
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 780.0).max(0.0) / 2.0;
                    ui.add_space(width);
                    ui.vertical(|ui| {
                        ui.set_width(780.0);
                        Frame::default()
                            .fill(PANEL)
                            .inner_margin(24.0)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Что нужно знать")
                                        .size(22.0)
                                        .color(ACCENT)
                                        .strong(),
                                );
                                ui.add_space(12.0);
                                for item in def.theory {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(RichText::new("-").color(CYAN));
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(*item)
                                                    .size(16.0)
                                                    .color(Color32::from_rgb(222, 232, 242)),
                                            )
                                            .wrap(),
                                        );
                                    });
                                    ui.add_space(12.0);
                                }
                                if self.current_level == 4 {
                                    ui.separator();
                                    ui.label(
                                        RichText::new(
                                            "Безопасность: в реальной жизни работы в щитке выполняет только квалифицированный специалист.",
                                        )
                                        .size(15.0)
                                        .color(Color32::from_rgb(255, 151, 115))
                                        .strong(),
                                    );
                                }
                            });
                        ui.add_space(22.0);
                        ui.vertical_centered(|ui| {
                            if ui
                                .add_sized(
                                    [300.0, 48.0],
                                    primary_button(
                                        if self.theory_return == Screen::Play {
                                            "К монтажному столу >"
                                        } else {
                                            "Вернуться"
                                        },
                                        GREEN,
                                    ),
                                )
                                .clicked()
                            {
                                self.screen = self.theory_return;
                            }
                            if self.theory_return == Screen::Play && ui.button("Вернуться на карту").clicked() {
                                self.screen = Screen::Map;
                            }
                        });
                    });
                });
            });
    }

    fn show_help(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(Frame::default().fill(BACKGROUND))
            .show(ui, |ui| {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.heading(RichText::new("Как играть").size(34.0).color(ACCENT));
                });
                ui.add_space(20.0);
                Frame::default()
                    .fill(PANEL)
                    .inner_margin(24.0)
                    .show(ui, |ui| {
                        ui.set_max_width(850.0);
                        for (title, text) in [
                            ("1. Установи детали", "Перетащи деталь из нижней панели в пустую рамку. Можно также выбрать деталь кликом, а затем нажать на рамку."),
                            ("2. Проведи провода", "В режиме «Провод» нажми последовательно на две клеммы. Активная клемма подсвечивается белым кольцом."),
                            ("3. Исправляй смело", "Ластик удаляет провод при клике на него. Правый клик по детали снимает её. Кнопка «Сброс» начинает уровень заново."),
                            ("4. Управляй и измеряй", "Обычный клик по выключателю меняет его состояние. На уровнях 3 и 4 выбери прибор, а затем нажми на измеряемый элемент."),
                            ("5. Проверь результат", "Выполни цели на панели справа и нажми зелёную кнопку проверки. Подсказки уменьшают итоговую награду, но не мешают пройти игру."),
                        ] {
                            ui.label(RichText::new(title).size(19.0).color(CYAN).strong());
                            ui.label(RichText::new(text).size(15.0));
                            ui.add_space(14.0);
                        }
                        ui.separator();
                        ui.label(
                            RichText::new(
                                "Все аварии виртуальные. Не повторяйте действия с реальными розетками, проводами или электрощитком.",
                            )
                            .color(Color32::from_rgb(255, 151, 115))
                            .strong(),
                        );
                    });
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    if ui.add_sized([220.0, 42.0], primary_button("Понятно", GREEN)).clicked() {
                        self.screen = Screen::Menu;
                    }
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

        egui::Panel::top("play_top")
            .exact_size(58.0)
            .frame(Frame::default().fill(PANEL))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("< Карта").clicked() {
                        self.screen = Screen::Map;
                    }
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!("{} · {}", index + 1, def.title))
                                .size(19.0)
                                .color(Color32::WHITE)
                                .strong(),
                        );
                        ui.label(RichText::new(def.subtitle).size(11.5).color(MUTED));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Сброс").clicked() {
                            session = LevelSession::new(index, &def);
                        }
                        if ui.button("Подсказка").clicked() {
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
                        if ui.button("Теория").clicked() {
                            self.theory_return = Screen::Play;
                            self.screen = Screen::Theory;
                        }
                    });
                });
            });

        egui::Panel::bottom("play_controls")
            .exact_size(142.0)
            .frame(Frame::default().fill(PANEL).inner_margin(10.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("ДЕТАЛИ").size(12.0).color(MUTED).strong());
                    for part in def.palette {
                        let selected = session.selected_part == Some(*part);
                        let response = ui.add(
                            Button::new(
                                RichText::new(part_short_label(*part))
                                    .size(13.0)
                                    .color(if selected { BACKGROUND } else { Color32::WHITE }),
                            )
                            .fill(if selected {
                                ACCENT
                            } else {
                                Color32::from_rgb(45, 65, 94)
                            })
                            .sense(Sense::click_and_drag()),
                        );
                        if response.clicked() || response.drag_started() {
                            session.selected_part = Some(*part);
                        }
                        if response.drag_stopped() {
                            dropped_part = Some(*part);
                        }
                    }
                    if !def.palette.is_empty() {
                        ui.label(
                            RichText::new("Перетащи деталь или выбери её и нажми на пустой слот")
                                .size(11.5)
                                .color(MUTED),
                        );
                    }
                });
                ui.add_space(4.0);
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
                        if ui.button(power_label).clicked() {
                            self.toggle_diagnostic_power(&mut session);
                        }
                    }
                    if index == 4 {
                        let power_label = if session.power_connected {
                            "Отключить питание"
                        } else {
                            "Подать питание"
                        };
                        if ui.button(power_label).clicked() {
                            self.toggle_panel_power(&def, &mut session);
                        }
                        if ui.button("Сбросить аварию").clicked() {
                            self.reset_panel_event(&mut session);
                        }
                        if ui.button("Нормальная нагрузка").clicked() {
                            self.panel_normal(&def, &mut session);
                        }
                        if ui.button("Перегрузка 25 А").clicked() {
                            self.panel_overload(&def, &mut session);
                        }
                        if ui.button("Утечка 30 мА").clicked() {
                            self.panel_leakage(&def, &mut session);
                        }
                    }
                    ui.separator();
                    let check_text = if index == 2 && session.circuit == 0 {
                        "Принять последовательную схему"
                    } else {
                        "Проверить задание"
                    };
                    if ui
                        .add_sized([250.0, 32.0], primary_button(check_text, GREEN))
                        .clicked()
                    {
                        self.check_level(&def, &mut session);
                    }
                });
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(match session.tool {
                            Tool::Wire => "Режим: клик по двум клеммам создаёт провод; клик по выключателю меняет контакт.",
                            Tool::Erase => "Режим: нажми на провод, который нужно удалить.",
                            Tool::Ammeter => "Амперметр: нажми на лампу, через которую нужно измерить ток.",
                            Tool::Voltmeter => "Вольтметр: нажми на элемент, на котором нужно измерить напряжение.",
                            Tool::Continuity => "Проверка целостности: источник должен быть отключён.",
                            Tool::Replace => "Замена: источник должен быть отключён, деталь сначала проверена.",
                        })
                        .size(11.5)
                        .color(MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "Ошибки: {} · Подсказки: {}",
                                session.mistakes, session.hints_used
                            ))
                            .size(11.5)
                            .color(MUTED),
                        );
                    });
                });
            });

        let current_reading = reading(&def, &session);
        egui::Panel::right("objectives")
            .exact_size(320.0)
            .frame(Frame::default().fill(Color32::from_rgb(18, 29, 49)).inner_margin(14.0))
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(RichText::new("Задания").color(ACCENT));
                        if index == 2 {
                            ui.label(
                                RichText::new(format!("Этап {}/2", session.circuit + 1))
                                    .color(CYAN)
                                    .strong(),
                            );
                        }
                    });
                    ui.separator();
                    show_feedback(ui, &session);
                    for (text, complete) in objectives(&def, &session, &current_reading) {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(if complete { "[x]" } else { "[ ]" })
                                    .size(14.0)
                                    .color(if complete { GREEN } else { MUTED })
                                    .strong(),
                            );
                            ui.label(
                                RichText::new(text)
                                    .size(13.5)
                                    .color(if complete { Color32::WHITE } else { MUTED }),
                            );
                        });
                        ui.add_space(5.0);
                    }
                    ui.add_space(8.0);
                    show_reading(ui, index, &session, &current_reading);

                    if index == 4
                        && let Some(selected) = session.selected_component
                    {
                            let component = &def.components[selected];
                            if !component.rating_options.is_empty() {
                                ui.separator();
                                ui.label(
                                    RichText::new(component.name)
                                        .size(15.0)
                                        .color(CYAN)
                                        .strong(),
                                );
                                ui.label("Выбери номинал в панели справа от схемы:");
                                let selected_text = session.components[selected]
                                    .rating
                                    .map_or("Не выбран".to_owned(), |rating| {
                                        format!("{rating} {}", component.rating_unit)
                                    });
                                ComboBox::from_id_salt(("rating", selected))
                                    .selected_text(selected_text)
                                    .show_ui(ui, |ui| {
                                        for option in component.rating_options {
                                            let label = format!("{option} {}", component.rating_unit);
                                            if ui
                                                .selectable_label(
                                                    session.components[selected].rating == Some(*option),
                                                    label,
                                                )
                                                .clicked()
                                            {
                                                if session.power_connected || session.panel_event.is_some() {
                                                    session.mistake(
                                                        "Сначала отключи питание и сбрось аварию. Номиналы под напряжением не меняют.",
                                                    );
                                                } else {
                                                    session.components[selected].rating = Some(*option);
                                                }
                                            }
                                        }
                                    });
                            }
                    }

                    if let Some(last) = &session.last_reading {
                        ui.add_space(8.0);
                        Frame::default()
                            .fill(Color32::from_rgb(28, 49, 74))
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("ПРИБОР / ИСПЫТАНИЕ").size(11.0).color(CYAN).strong());
                                ui.label(RichText::new(last).size(14.0));
                            });
                    }

                    ui.add_space(8.0);
                    ui.collapsing(format!("Провода: {}", session.wires.len()), |ui| {
                        let mut remove = None;
                        for (wire_index, wire) in session.wires.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(wire_name(&def, *wire)).size(11.5));
                                if ui.small_button("×").clicked() {
                                    remove = Some(wire_index);
                                }
                            });
                        }
                        if let Some(wire_index) = remove
                            && let Err(message) = session.remove_wire(wire_index)
                        {
                            session.set_feedback(FeedbackKind::Warning, message);
                        }
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::default().fill(BACKGROUND).inner_margin(8.0))
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
        egui::CentralPanel::default()
            .frame(Frame::default().fill(BACKGROUND))
            .show(ui, |ui| {
                ui.add_space(65.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("ЗАДАНИЕ ВЫПОЛНЕНО")
                            .size(16.0)
                            .color(GREEN)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(def.title)
                            .size(38.0)
                            .color(Color32::WHITE)
                            .strong(),
                    );
                    ui.add_space(16.0);
                    star_rating(ui, self.result_stars);
                    ui.label(
                        RichText::new(match self.result_stars {
                            3 => "Отлично: точная работа без подсказок!",
                            2 => "Хорошо: схема собрана и проверена!",
                            _ => "Готово: новый раздел электротехники открыт!",
                        })
                        .size(18.0)
                        .color(MUTED),
                    );
                    ui.add_space(22.0);
                    Frame::default()
                        .fill(PANEL)
                        .inner_margin(22.0)
                        .show(ui, |ui| {
                            ui.set_width(620.0);
                            ui.label(
                                RichText::new("Что ты узнал")
                                    .size(20.0)
                                    .color(CYAN)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(level_summary(self.current_level))
                                    .size(16.0)
                                    .color(Color32::from_rgb(222, 232, 242)),
                            );
                        });
                    ui.add_space(28.0);
                    if self.current_level < 4
                        && ui
                            .add_sized([310.0, 48.0], primary_button("Следующий уровень >", GREEN))
                            .clicked()
                    {
                        self.start_level(self.current_level + 1);
                    }
                    if self.current_level == 4 {
                        ui.label(
                            RichText::new(
                                "Все пять уровней пройдены. Электролаборатория завершена!",
                            )
                            .size(18.0)
                            .color(ACCENT)
                            .strong(),
                        );
                    }
                    ui.add_space(8.0);
                    if ui
                        .add_sized([310.0, 40.0], primary_button("Карта уровней", CYAN))
                        .clicked()
                    {
                        self.screen = Screen::Map;
                    }
                    if ui.button("Пройти этот уровень ещё раз").clicked() {
                        self.start_level(self.current_level);
                    }
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
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }
}

fn configure_appearance(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "noto_sans".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
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
        .push("noto_sans".to_owned());
    ctx.set_fonts(fonts);

    let mut style = (*ctx.global_style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = PANEL;
    style.visuals.selection.bg_fill = Color32::from_rgb(50, 128, 158);
    style.spacing.item_spacing = Vec2::new(9.0, 8.0);
    style
        .text_styles
        .insert(TextStyle::Body, egui::FontId::proportional(15.0));
    ctx.set_global_style(style);
}

fn primary_button(text: impl Into<String>, color: Color32) -> Button<'static> {
    Button::new(RichText::new(text).size(15.0).color(BACKGROUND).strong()).fill(color)
}

fn tool_button(ui: &mut egui::Ui, session: &mut LevelSession, tool: Tool, label: &str) {
    let selected = session.tool == tool;
    if ui
        .add(
            Button::new(RichText::new(label).color(if selected {
                BACKGROUND
            } else {
                Color32::WHITE
            }))
            .fill(if selected {
                CYAN
            } else {
                Color32::from_rgb(45, 65, 94)
            }),
        )
        .clicked()
    {
        session.tool = tool;
        session.selected_terminal = None;
    }
}

fn show_feedback(ui: &mut egui::Ui, session: &LevelSession) {
    if let Some(feedback) = &session.message {
        let color = match feedback.kind {
            FeedbackKind::Info => CYAN,
            FeedbackKind::Success => GREEN,
            FeedbackKind::Warning => ACCENT,
            FeedbackKind::Error => Color32::from_rgb(255, 113, 115),
        };
        Frame::default()
            .fill(Color32::from_rgb(29, 43, 66))
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(&feedback.text)
                        .size(14.0)
                        .color(color)
                        .strong(),
                );
            });
        ui.add_space(8.0);
    }
}

fn show_reading(
    ui: &mut egui::Ui,
    level: usize,
    session: &LevelSession,
    reading: &ElectricalReading,
) {
    let (title, color) = if level == 4 {
        match session.panel_event {
            Some(crate::model::PanelEvent::Overload) => ("АВТОМАТ ОТКЛЮЧИЛ ЦЕПЬ", ACCENT),
            Some(crate::model::PanelEvent::Leakage) if session.has_flag("leakage_ok") => {
                ("УЗО ОТКЛЮЧИЛО УТЕЧКУ", GREEN)
            }
            Some(crate::model::PanelEvent::Leakage) => ("ПРОВЕРЬ УСТАВКУ УЗО", ACCENT),
            None if session.power_connected && session.has_flag("normal_ok") => {
                ("НОРМАЛЬНЫЙ РЕЖИМ", GREEN)
            }
            None if session.power_connected => ("ПИТАНИЕ ВКЛЮЧЕНО", ACCENT),
            None => ("ЩИТОК ОБЕСТОЧЕН", MUTED),
        }
    } else {
        match reading.status {
            CircuitStatus::Incomplete => ("СХЕМА НЕ ЗАВЕРШЕНА", MUTED),
            CircuitStatus::Open => ("ТОКА НЕТ", ACCENT),
            CircuitStatus::Short => ("КОРОТКОЕ ЗАМЫКАНИЕ", Color32::from_rgb(255, 113, 115)),
            CircuitStatus::Running => ("ЦЕПЬ РАБОТАЕТ", GREEN),
            CircuitStatus::Overload => ("ПЕРЕГРУЗКА", Color32::from_rgb(255, 113, 115)),
            CircuitStatus::Dim => ("СЛАБЫЙ ТОК", ACCENT),
        }
    };
    ui.separator();
    ui.label(RichText::new(title).size(13.0).color(color).strong());
    if level != 4 {
        ui.label(RichText::new(&reading.detail).size(13.5));
        if let Some(total_current) = reading.total_current_a {
            ui.label(
                RichText::new(format!(
                    "Общий ток источника: {} А",
                    crate::simulation::decimal(total_current, 2)
                ))
                .size(12.5)
                .color(MUTED),
            );
        }
    }
    if level == 4 && session.power_connected {
        ui.label(
            RichText::new("Питание включено. Перед изменением схемы отключи его.")
                .size(12.5)
                .color(ACCENT),
        );
    }
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

fn level_color(index: usize) -> Color32 {
    match index {
        0 => Color32::from_rgb(255, 203, 74),
        1 => Color32::from_rgb(255, 133, 107),
        2 => Color32::from_rgb(83, 202, 231),
        3 => Color32::from_rgb(175, 130, 244),
        _ => Color32::from_rgb(80, 202, 137),
    }
}

fn star_rating(ui: &mut egui::Ui, stars: u8) {
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - 204.0) / 2.0).max(0.0));
        for index in 0..3 {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 64.0), Sense::hover());
            let center = rect.center();
            let points = (0..10)
                .map(|point| {
                    let angle =
                        -std::f32::consts::FRAC_PI_2 + point as f32 * std::f32::consts::PI / 5.0;
                    let radius = if point % 2 == 0 { 28.0 } else { 13.0 };
                    center + Vec2::new(angle.cos() * radius, angle.sin() * radius)
                })
                .collect();
            let earned = index < stars;
            ui.painter().add(egui::Shape::convex_polygon(
                points,
                if earned {
                    ACCENT
                } else {
                    Color32::from_rgb(38, 51, 72)
                },
                Stroke::new(2.0, if earned { ACCENT } else { MUTED }),
            ));
        }
    });
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
