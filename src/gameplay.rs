use crate::app::{ElectroApp, Screen};
use crate::board::BoardAction;
use crate::model::{
    FeedbackKind, LevelDef, LevelSession, PartKind, TerminalKind, TerminalRef, Tool,
};
use crate::simulation::{CircuitStatus, connection_issue, decimal, reading};

impl ElectroApp {
    pub(crate) fn handle_board_action(
        &mut self,
        def: &LevelDef,
        session: &mut LevelSession,
        action: BoardAction,
    ) {
        match action {
            BoardAction::Component(index) => self.component_clicked(def, session, index),
            BoardAction::RemoveComponent(index) => match session.remove_part(def, index) {
                Ok(()) => {
                    session.set_feedback(FeedbackKind::Info, "Деталь снята, её провода удалены.")
                }
                Err(message) => session.set_feedback(FeedbackKind::Warning, message),
            },
            BoardAction::Terminal(terminal) => terminal_clicked(def, session, terminal),
            BoardAction::RemoveWire(index) => match session.remove_wire(index) {
                Ok(()) => session.set_feedback(FeedbackKind::Info, "Провод удалён."),
                Err(message) => session.set_feedback(FeedbackKind::Warning, message),
            },
        }
    }

    fn component_clicked(&mut self, def: &LevelDef, session: &mut LevelSession, index: usize) {
        if session.components[index].placed.is_none() {
            let Some(part) = session.selected_part else {
                session.set_feedback(
                    FeedbackKind::Info,
                    "Сначала выбери деталь на нижней панели.",
                );
                return;
            };
            match session.place_part(def, index, part) {
                Ok(()) => session.set_feedback(
                    FeedbackKind::Success,
                    format!("{} установлено. Теперь соедини клеммы.", part.label()),
                ),
                Err(message) => session.set_feedback(FeedbackKind::Warning, message),
            }
            return;
        }

        session.selected_component = Some(index);
        let part = session.components[index].placed.unwrap_or(PartKind::Lamp);
        match session.level {
            0 if part == PartKind::Switch && session.tool == Tool::Wire => {
                let closed = {
                    let closed = &mut session.components[index].closed;
                    *closed = !*closed;
                    *closed
                };
                session.set_feedback(
                    if closed {
                        FeedbackKind::Success
                    } else {
                        FeedbackKind::Info
                    },
                    if closed {
                        "Выключатель замкнут. Если контур собран верно, лампа загорится."
                    } else {
                        "Выключатель разомкнут: путь для тока разорван."
                    },
                );
            }
            2 if matches!(session.tool, Tool::Ammeter | Tool::Voltmeter) => {
                measure_lamp(def, session, index);
            }
            3 => diagnostic_component_clicked(session, index, part),
            4 => {
                let component = &def.components[index];
                if component.rating_options.is_empty() {
                    session.set_feedback(FeedbackKind::Info, component.name);
                } else {
                    session.set_feedback(
                        FeedbackKind::Info,
                        "Номинал выбирается в списке на правой панели. Питание должно быть отключено.",
                    );
                }
            }
            _ => {}
        }
    }

    pub(crate) fn toggle_diagnostic_power(&mut self, session: &mut LevelSession) {
        session.power_connected = !session.power_connected;
        if !session.power_connected {
            session.set_feedback(
                FeedbackKind::Success,
                "Источник отключён. Теперь можно измерять сопротивление и менять детали.",
            );
        } else if !session.components[1].healthy {
            session.set_feedback(
                FeedbackKind::Info,
                "Источник подключён, но предохранитель разомкнут. Проверь напряжение источника, затем отключи его для диагностики.",
            );
        } else if !session.components[3].healthy {
            session.mistake(
                "В цепи остаётся короткое замыкание лампы. Отключи источник и устрани причину.",
            );
        } else {
            session.set_feedback(
                FeedbackKind::Success,
                "Источник подключён. При замкнутом выключателе исправная лампа горит.",
            );
        }
    }

    pub(crate) fn toggle_panel_power(&mut self, def: &LevelDef, session: &mut LevelSession) {
        if session.power_connected {
            session.power_connected = false;
            session.set_feedback(
                FeedbackKind::Success,
                "Питание отключено. Теперь можно безопасно менять учебную схему.",
            );
            return;
        }
        if session.panel_event.is_some() {
            session
                .mistake("Авария ещё активна. Нажми «Сбросить аварию», затем снова подай питание.");
            return;
        }
        if let Some(message) = connection_issue(def, session) {
            session.mistake(message);
            return;
        }
        let missing_rating = def.components.iter().enumerate().any(|(index, component)| {
            !component.rating_options.is_empty() && session.components[index].rating.is_none()
        });
        if missing_rating {
            session.mistake("У всех автоматов и УЗО должен быть выбран номинал. Нажми на аппарат и используй список справа.");
            return;
        }
        if session
            .components
            .iter()
            .any(|state| state.placed.is_some_and(PartKind::is_protective) && !state.closed)
        {
            session.mistake(
                "Один из аппаратов остался отключён. Сбрось аварию, чтобы взвести защиту.",
            );
            return;
        }
        session.power_connected = true;
        session.set_feedback(
            FeedbackKind::Success,
            "Питание виртуального щитка включено. Сначала запусти нормальную нагрузку.",
        );
    }

    pub(crate) fn panel_normal(&mut self, def: &LevelDef, session: &mut LevelSession) {
        if let Err(message) = panel_precheck(def, session) {
            session.mistake(message);
            return;
        }
        let main = def.component_index("main").unwrap_or(0);
        let socket = def.component_index("socket_breaker").unwrap_or(0);
        if session.components[main].rating == Some(10) {
            session.components[main].closed = false;
            session.panel_event = Some(crate::model::PanelEvent::Overload);
            session.mistake("Общий ток около 13,3 А превышает номинал вводного автомата 10 А. Он отключил весь щиток.");
            session.last_reading =
                Some("Вводной автомат 10 А: отключение при нормальной нагрузке.".to_owned());
            return;
        }
        if session.components[socket].rating == Some(10) {
            session.components[socket].closed = false;
            session.panel_event = Some(crate::model::PanelEvent::Overload);
            session.mistake("Розетки потребляют 12 А, поэтому групповой автомат 10 А отключился даже при нормальной нагрузке.");
            session.last_reading =
                Some("Автомат розеток 10 А: отключение при токе 12 А.".to_owned());
            return;
        }
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

    pub(crate) fn panel_overload(&mut self, def: &LevelDef, session: &mut LevelSession) {
        if let Err(message) = panel_precheck(def, session) {
            session.mistake(message);
            return;
        }
        if !session.has_flag("normal_ok") {
            session.mistake("Сначала убедись, что схема выдерживает нормальную нагрузку.");
            return;
        }
        let socket = def.component_index("socket_breaker").unwrap_or(0);
        match session.components[socket].rating {
            Some(16) => {
                session.components[socket].closed = false;
                session.panel_event = Some(crate::model::PanelEvent::Overload);
                session.flag("overload_ok");
                session.last_reading = Some(
                    "Длительная перегрузка 25 А: автомат розеток 16 А отключил только свою группу."
                        .to_owned(),
                );
                session.set_feedback(
                    FeedbackKind::Success,
                    "Автомат розеток сработал и ограничил аварию одной группой. Сбрось аварию, затем проверь утечку.",
                );
            }
            Some(32) => {
                session.last_reading = Some(
                    "Перегрузка 25 А: автомат 32 А не защищает эту линию от длительного перегруза."
                        .to_owned(),
                );
                session.mistake("Номинал 32 А слишком велик для учебной линии розеток. Отключи питание, выбери 16 А и повтори испытания.");
            }
            _ => {
                session.mistake(
                    "Для испытания перегрузки нужен правильно подобранный автомат розеток 16 А.",
                );
            }
        }
    }

    pub(crate) fn panel_leakage(&mut self, def: &LevelDef, session: &mut LevelSession) {
        if let Err(message) = panel_precheck(def, session) {
            session.mistake(message);
            return;
        }
        if !session.has_flag("normal_ok") {
            session.mistake("Сначала проверь нормальную нагрузку.");
            return;
        }
        let rcd = def.component_index("rcd").unwrap_or(0);
        match session.components[rcd].rating {
            Some(30) => {
                session.components[rcd].closed = false;
                session.panel_event = Some(crate::model::PanelEvent::Leakage);
                session.flag("leakage_ok");
                session.last_reading = Some(
                    "Утечка 30 мА: УЗО обнаружило разницу токов L и N и отключило питание. Автоматы не рассчитаны на такую утечку."
                        .to_owned(),
                );
                session.set_feedback(
                    FeedbackKind::Success,
                    "УЗО сработало на утечку. Это защита другого типа, чем срабатывание автомата при перегрузке.",
                );
            }
            Some(10) => {
                session.components[rcd].closed = false;
                session.panel_event = Some(crate::model::PanelEvent::Leakage);
                session.mistake("УЗО 10 мА отключилось, но в учебном задании требуется уставка 30 мА. Сбрось аварию, отключи питание и исправь номинал.");
                session.last_reading =
                    Some("УЗО 10 мА: отключение; требуемая учебная уставка — 30 мА.".to_owned());
            }
            _ => {
                session.panel_event = Some(crate::model::PanelEvent::Leakage);
                session.mistake("Уставка 100 мА не реагирует на учебную утечку 30 мА. Опасное состояние сохраняется. Сбрось сценарий и выбери 30 мА.");
                session.last_reading =
                    Some("УЗО 100 мА: при утечке 30 мА отключения нет.".to_owned());
            }
        }
    }

    pub(crate) fn reset_panel_event(&mut self, session: &mut LevelSession) {
        if session.panel_event.is_none() {
            session.set_feedback(
                FeedbackKind::Info,
                "Активной аварии нет. Защитные аппараты в нормальном состоянии.",
            );
            return;
        }
        session.panel_event = None;
        session.power_connected = false;
        for state in &mut session.components {
            if state.placed.is_some_and(PartKind::is_protective) {
                state.closed = true;
            }
        }
        session.set_feedback(
            FeedbackKind::Info,
            "Виртуальная причина аварии устранена, аппараты взведены, питание отключено. Подай питание для следующего испытания.",
        );
    }

    pub(crate) fn check_level(&mut self, def: &LevelDef, session: &mut LevelSession) {
        if let Some(message) = connection_issue(def, session) {
            session.mistake(message);
            return;
        }
        let current_reading = reading(def, session);
        match session.level {
            0 => {
                if current_reading.status == CircuitStatus::Running {
                    self.complete_level(session);
                } else {
                    session.mistake("Соединения правильные, но цепь разомкнута. Нажми на выключатель.");
                }
            }
            1 => match current_reading.status {
                CircuitStatus::Running => self.complete_level(session),
                CircuitStatus::Overload => session.mistake(
                    "С резистором 100 Ом ток составляет 30 мА — это слишком много. Нужен резистор 300 Ом.",
                ),
                CircuitStatus::Dim => session.mistake(
                    "С резистором 1 кОм ток всего 3 мА — светодиод светит слишком слабо. Расчёт даёт 300 Ом.",
                ),
                _ => session.mistake("Цепь ещё не готова. Проверь детали и соединения."),
            },
            2 => {
                if !session.has_flag("current_measured") || !session.has_flag("voltage_measured") {
                    session.mistake("Схема собрана, но нужно снять показания и амперметром, и вольтметром. Выбери прибор и нажми на лампу.");
                    return;
                }
                if session.circuit == 0 {
                    session.circuit = 1;
                    session.wires.clear();
                    session.flags.remove("current_measured");
                    session.flags.remove("voltage_measured");
                    session.tool = Tool::Wire;
                    session.selected_terminal = None;
                    session.last_reading = Some(
                        "Сравнение: последовательно лампы получали по 6 В и были тусклее. Теперь собери параллельную схему."
                            .to_owned(),
                    );
                    session.set_feedback(
                        FeedbackKind::Success,
                        "Первый этап принят. Провода очищены: соедини обе лампы параллельно и повтори измерения.",
                    );
                } else {
                    self.complete_level(session);
                }
            }
            3 => {
                let required_flags = [
                    "source_voltage",
                    "fuse_tested",
                    "lamp_tested",
                    "lamp_replaced",
                    "fuse_replaced",
                ];
                if required_flags.iter().any(|flag| !session.has_flag(flag)) {
                    session.mistake("Диагностика не завершена: выполни все пункты измерений и замены деталей на панели справа.");
                    return;
                }
                if current_reading.status == CircuitStatus::Running {
                    self.complete_level(session);
                } else {
                    session.mistake("Детали заменены, но лампа ещё не горит. Подключи источник и замкни выключатель.");
                }
            }
            4 => {
                if !session.ratings_match(def) {
                    session.mistake("Проверь номиналы: вводной 20 А, УЗО 30 мА, свет 10 А, розетки 16 А.");
                    return;
                }
                if !session.has_flag("normal_ok")
                    || !session.has_flag("overload_ok")
                    || !session.has_flag("leakage_ok")
                {
                    session.mistake("Схема собрана, но испытания не завершены. Нужны нормальная нагрузка, правильное отключение перегрузки и утечки.");
                    return;
                }
                self.complete_level(session);
            }
            _ => {}
        }
    }

    fn complete_level(&mut self, session: &mut LevelSession) {
        if session.completed {
            return;
        }
        session.completed = true;
        let stars = if session.hints_used == 0 && session.mistakes == 0 {
            3
        } else if session.hints_used <= 1 && session.mistakes <= 3 {
            2
        } else {
            1
        };
        self.result_stars = stars;
        self.progress.stars[session.level] = self.progress.stars[session.level].max(stars);
        self.progress.unlocked = self.progress.unlocked.max((session.level + 2).min(5));
        session.set_feedback(FeedbackKind::Success, "Уровень пройден!");
        self.screen = Screen::Result;
    }
}

fn terminal_clicked(def: &LevelDef, session: &mut LevelSession, terminal: TerminalRef) {
    if let Some(message) = session.editing_blocked() {
        session.set_feedback(FeedbackKind::Warning, message);
        return;
    }
    let Some(first) = session.selected_terminal else {
        session.selected_terminal = Some(terminal);
        let component = &def.components[terminal.component];
        let terminal_def = &component.terminals[terminal.terminal];
        session.set_feedback(
            FeedbackKind::Info,
            format!(
                "Выбрана клемма {} · {}. Теперь нажми вторую клемму.",
                component.name, terminal_def.label
            ),
        );
        return;
    };
    session.selected_terminal = None;
    if first == terminal {
        session.set_feedback(FeedbackKind::Info, "Выбор клеммы отменён.");
        return;
    }
    match session.add_wire(def, first, terminal) {
        Ok(()) => {
            let a = def.terminal_def(first).kind;
            let b = def.terminal_def(terminal).kind;
            if matches!(
                (a, b),
                (TerminalKind::Neutral, TerminalKind::Protective)
                    | (TerminalKind::Protective, TerminalKind::Neutral)
            ) {
                session.mistake("Провод добавлен, но соединять N и PE после УЗО нельзя. Удали этот провод ластиком.");
            } else {
                session.set_feedback(FeedbackKind::Success, "Провод соединил клеммы.");
            }
        }
        Err(message) => session.set_feedback(FeedbackKind::Warning, message),
    }
}

fn measure_lamp(def: &LevelDef, session: &mut LevelSession, index: usize) {
    if session.components[index].placed != Some(PartKind::Lamp) {
        session.set_feedback(
            FeedbackKind::Info,
            "В этом задании измеряют параметры одной из ламп.",
        );
        return;
    }
    let current = reading(def, session);
    if current.status != CircuitStatus::Running {
        session.mistake(
            "Сначала собери схему текущего этапа: при правильных соединениях лампы горят.",
        );
        return;
    }
    let lamp_name = def.components[index].name;
    match session.tool {
        Tool::Ammeter => {
            let value = current.component_current_a.unwrap_or(0.0);
            session.flag("current_measured");
            session.last_reading = Some(format!(
                "Амперметр · {lamp_name}: {} А. Прибор включён последовательно с этой лампой.",
                decimal(value, 2)
            ));
            session.set_feedback(
                FeedbackKind::Success,
                "Сила тока измерена. Теперь используй вольтметр.",
            );
        }
        Tool::Voltmeter => {
            let value = current.lamp_voltage_v.unwrap_or(0.0);
            session.flag("voltage_measured");
            session.last_reading = Some(format!(
                "Вольтметр · {lamp_name}: {} В. Прибор подключён параллельно лампе.",
                decimal(value, 1)
            ));
            session.set_feedback(
                FeedbackKind::Success,
                "Напряжение измерено. Можно сравнить яркость и результаты этапов.",
            );
        }
        _ => {}
    }
}

fn diagnostic_component_clicked(session: &mut LevelSession, index: usize, part: PartKind) {
    match session.tool {
        Tool::Wire => {
            if part == PartKind::Switch {
                let closed = {
                    let closed = &mut session.components[index].closed;
                    *closed = !*closed;
                    *closed
                };
                session.set_feedback(
                    FeedbackKind::Info,
                    if closed {
                        "Выключатель замкнут."
                    } else {
                        "Выключатель разомкнут."
                    },
                );
            } else {
                session.set_feedback(
                    FeedbackKind::Info,
                    "Выбери вольтметр, проверку целостности или замену детали на нижней панели.",
                );
            }
        }
        Tool::Voltmeter => {
            if !session.power_connected {
                session.mistake("Напряжение источника измеряют при подключённом источнике. Подключи его для этого измерения.");
                return;
            }
            if part == PartKind::DcSource {
                session.flag("source_voltage");
                session.last_reading = Some(
                    "Вольтметр · источник: 12 В. Источник исправен. Теперь отключи его перед проверкой сопротивления."
                        .to_owned(),
                );
                session.set_feedback(FeedbackKind::Success, "Напряжение источника в норме: 12 В.");
            } else {
                session.set_feedback(
                    FeedbackKind::Info,
                    "Сначала измерь напряжение на самом источнике.",
                );
            }
        }
        Tool::Continuity => {
            if session.power_connected {
                session.mistake("Опасно: сопротивление и целостность проверяют только при отключённом источнике. Отключи его кнопкой внизу.");
                return;
            }
            match part {
                PartKind::Fuse => {
                    session.flag("fuse_tested");
                    let healthy = session.components[index].healthy;
                    session.last_reading = Some(if healthy {
                        "Предохранитель: 0,2 Ом. Проводимость есть, деталь исправна.".to_owned()
                    } else {
                        "Предохранитель: обрыв. Цепь внутри разомкнута — предохранитель перегорел."
                            .to_owned()
                    });
                    session.set_feedback(
                        if healthy {
                            FeedbackKind::Success
                        } else {
                            FeedbackKind::Warning
                        },
                        if healthy {
                            "Предохранитель исправен."
                        } else {
                            "Найден обрыв предохранителя. Проверь и лампу: нужно найти причину."
                        },
                    );
                }
                PartKind::Lamp => {
                    session.flag("lamp_tested");
                    let healthy = session.components[index].healthy;
                    session.last_reading = Some(if healthy {
                        "Лампа: 24 Ом. Сопротивление соответствует исправной лампе.".to_owned()
                    } else {
                        "Лампа: 0 Ом вместо 24 Ом. Внутреннее короткое замыкание — это причина сгоревшего предохранителя.".to_owned()
                    });
                    session.set_feedback(
                        if healthy {
                            FeedbackKind::Success
                        } else {
                            FeedbackKind::Warning
                        },
                        if healthy {
                            "Лампа исправна."
                        } else {
                            "Найдено короткое замыкание лампы. Замени её до замены предохранителя."
                        },
                    );
                }
                _ => session.set_feedback(FeedbackKind::Info, "Проверь предохранитель и лампу."),
            }
        }
        Tool::Replace => {
            if session.power_connected {
                session.mistake(
                    "Менять детали подключённого стенда нельзя. Сначала отключи источник.",
                );
                return;
            }
            match part {
                PartKind::Lamp => {
                    if !session.has_flag("lamp_tested") {
                        session.mistake(
                            "Сначала измерь сопротивление лампы, чтобы подтвердить неисправность.",
                        );
                        return;
                    }
                    if session.components[index].healthy {
                        session.set_feedback(FeedbackKind::Info, "Лампа уже исправна.");
                        return;
                    }
                    session.components[index].healthy = true;
                    session.flag("lamp_replaced");
                    session.last_reading = Some(
                        "Новая лампа: сопротивление 24 Ом, короткое замыкание устранено."
                            .to_owned(),
                    );
                    session.set_feedback(
                        FeedbackKind::Success,
                        "Лампа заменена. Теперь можно заменить перегоревший предохранитель.",
                    );
                }
                PartKind::Fuse => {
                    if !session.has_flag("fuse_tested") {
                        session.mistake("Сначала проверь предохранитель в режиме целостности.");
                        return;
                    }
                    if !session.components[3].healthy {
                        session.mistake("Причина короткого замыкания ещё остаётся. Сначала замени лампу, иначе новый предохранитель снова выйдет из строя.");
                        return;
                    }
                    if session.components[index].healthy {
                        session.set_feedback(FeedbackKind::Info, "Предохранитель уже исправен.");
                        return;
                    }
                    session.components[index].healthy = true;
                    session.flag("fuse_replaced");
                    session.last_reading =
                        Some("Новый предохранитель: 0,2 Ом. Цепь защиты восстановлена.".to_owned());
                    session.set_feedback(FeedbackKind::Success, "Предохранитель заменён после устранения причины. Подключи источник и проверь свет.");
                }
                _ => session.set_feedback(
                    FeedbackKind::Info,
                    "Заменить нужно неисправные лампу и предохранитель.",
                ),
            }
        }
        _ => session.set_feedback(FeedbackKind::Info, "Выбери нужный прибор или режим замены."),
    }
}

fn panel_precheck(def: &LevelDef, session: &LevelSession) -> Result<(), String> {
    if !session.power_connected {
        return Err("Сначала подай питание на виртуальный щиток.".to_owned());
    }
    if session.panel_event.is_some() {
        return Err("Авария уже активна. Сбрось её перед следующим испытанием.".to_owned());
    }
    if let Some(message) = connection_issue(def, session) {
        return Err(message);
    }
    if session
        .components
        .iter()
        .any(|state| state.placed.is_some_and(PartKind::is_protective) && !state.closed)
    {
        return Err(
            "Один из защитных аппаратов отключён. Сбрось аварию и взведи защиту.".to_owned(),
        );
    }
    Ok(())
}
