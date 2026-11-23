use crate::model::{LevelDef, LevelSession, PartKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CircuitStatus {
    Incomplete,
    Open,
    Short,
    Running,
    Overload,
    Dim,
}

#[derive(Clone, Debug)]
pub struct ElectricalReading {
    pub status: CircuitStatus,
    pub total_current_a: Option<f32>,
    pub component_current_a: Option<f32>,
    pub lamp_voltage_v: Option<f32>,
    pub lamp_power_w: Option<f32>,
    pub detail: String,
}

impl ElectricalReading {
    fn idle(status: CircuitStatus, detail: impl Into<String>) -> Self {
        Self {
            status,
            total_current_a: None,
            component_current_a: None,
            lamp_voltage_v: None,
            lamp_power_w: None,
            detail: detail.into(),
        }
    }

    pub fn brightness(&self) -> f32 {
        match self.status {
            CircuitStatus::Running => {
                if self.lamp_power_w.is_some_and(|power| power < 2.0) {
                    0.4
                } else {
                    1.0
                }
            }
            CircuitStatus::Dim => 0.22,
            CircuitStatus::Overload => 0.8,
            _ => 0.0,
        }
    }
}

pub fn decimal(value: f32, digits: usize) -> String {
    format!("{value:.digits$}").replace('.', ",")
}

pub fn connection_issue(def: &LevelDef, session: &LevelSession) -> Option<String> {
    if session.has_direct_short(def) {
        return Some(
            "Короткое замыкание: источник замкнут проводниками без нагрузки. Удали ошибочный провод."
                .to_owned(),
        );
    }
    if session.level == 4 && session.has_neutral_pe_bridge(def) {
        return Some("N и PE соединены вместе. После УЗО эти проводники разделяют.".to_owned());
    }

    if session.level == 1 {
        let missing = def
            .required
            .iter()
            .filter_map(|(id, _)| {
                let index = def.component_index(id)?;
                (session.components[index].placed.is_none()).then_some(def.components[index].name)
            })
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Some(format!("Установи детали: {}.", missing.join(", ")));
        }
    } else {
        let missing = session.missing_parts(def);
        if !missing.is_empty() {
            return Some(format!(
                "Проверь установку деталей: {}.",
                missing.join(", ")
            ));
        }
    }

    if !session.wiring_matches(def) {
        let expected = def.expected_wires(session.circuit);
        let missing = expected
            .iter()
            .filter(|wire| !session.wires.contains(wire))
            .count();
        let extra = session
            .wires
            .iter()
            .filter(|wire| !expected.contains(wire))
            .count();
        return Some(format!(
            "Соединения пока не совпадают со схемой: не хватает {missing}, лишних {extra}. Провод можно стереть или удалить в списке."
        ));
    }
    None
}

pub fn reading(def: &LevelDef, session: &LevelSession) -> ElectricalReading {
    if session.has_direct_short(def) {
        return ElectricalReading::idle(
            CircuitStatus::Short,
            "Короткое замыкание — ток не ограничен нагрузкой.",
        );
    }

    match session.level {
        0 => level1_reading(def, session),
        1 => level2_reading(def, session),
        2 => level3_reading(def, session),
        3 => level4_reading(session),
        _ => ElectricalReading::idle(
            CircuitStatus::Incomplete,
            "Испытания щитка запускаются отдельными сценариями.",
        ),
    }
}

fn level1_reading(def: &LevelDef, session: &LevelSession) -> ElectricalReading {
    if !session.missing_parts(def).is_empty() || !session.wiring_matches(def) {
        return ElectricalReading::idle(
            CircuitStatus::Incomplete,
            "Собери замкнутый контур из трёх проводов.",
        );
    }
    let Some(switch) = def.component_index("switch") else {
        return ElectricalReading::idle(CircuitStatus::Incomplete, "Нет выключателя.");
    };
    if !session.components[switch].closed {
        return ElectricalReading::idle(
            CircuitStatus::Open,
            "Цепь разомкнута выключателем. Нажми на него.",
        );
    }
    ElectricalReading {
        status: CircuitStatus::Running,
        total_current_a: Some(0.5),
        component_current_a: Some(0.5),
        lamp_voltage_v: Some(6.0),
        lamp_power_w: Some(3.0),
        detail: "Цепь замкнута: I = 6 В / 12 Ом = 0,5 А.".to_owned(),
    }
}

fn level2_reading(def: &LevelDef, session: &LevelSession) -> ElectricalReading {
    let Some(resistor) = def.component_index("resistor") else {
        return ElectricalReading::idle(CircuitStatus::Incomplete, "Нет резистора.");
    };
    let Some(led) = def.component_index("led") else {
        return ElectricalReading::idle(CircuitStatus::Incomplete, "Нет светодиода.");
    };
    if session.components[resistor].placed.is_none()
        || session.components[led].placed.is_none()
        || !session.wiring_matches(def)
    {
        return ElectricalReading::idle(
            CircuitStatus::Incomplete,
            "Установи резистор и светодиод, затем замкни цепь.",
        );
    }
    let resistance = session.components[resistor]
        .placed
        .and_then(PartKind::resistance_ohms)
        .unwrap_or(300.0);
    let current_a = 3.0 / resistance;
    let current_ma = current_a * 1000.0;
    let status = if current_ma > 20.0 {
        CircuitStatus::Overload
    } else if current_ma < 5.0 {
        CircuitStatus::Dim
    } else {
        CircuitStatus::Running
    };
    let detail = format!(
        "I = (5 В − 2 В) / {} Ом = {} мА.",
        resistance as u32,
        decimal(current_ma, 1)
    );
    ElectricalReading {
        status,
        total_current_a: Some(current_a),
        component_current_a: Some(current_a),
        lamp_voltage_v: Some(2.0),
        lamp_power_w: Some(2.0 * current_a),
        detail,
    }
}

fn level3_reading(def: &LevelDef, session: &LevelSession) -> ElectricalReading {
    if !session.missing_parts(def).is_empty() || !session.wiring_matches(def) {
        return ElectricalReading::idle(
            CircuitStatus::Incomplete,
            if session.circuit == 0 {
                "Собери один последовательный путь через обе лампы."
            } else {
                "Создай две параллельные ветви между клеммами источника."
            },
        );
    }
    let (total_current_a, component_current_a, lamp_voltage_v, lamp_power_w) =
        if session.circuit == 0 {
            (0.25, 0.25, 6.0, 1.5)
        } else {
            (1.0, 0.5, 12.0, 6.0)
        };
    let detail = if session.circuit == 0 {
        "Последовательно: R = 24 + 24 = 48 Ом; I = 12 / 48 = 0,25 А; на каждой лампе 6 В."
            .to_owned()
    } else {
        "Параллельно: на каждой лампе 12 В; ток ветви 0,5 А; общий ток 1 А.".to_owned()
    };
    ElectricalReading {
        status: CircuitStatus::Running,
        total_current_a: Some(total_current_a),
        component_current_a: Some(component_current_a),
        lamp_voltage_v: Some(lamp_voltage_v),
        lamp_power_w: Some(lamp_power_w),
        detail,
    }
}

fn level4_reading(session: &LevelSession) -> ElectricalReading {
    if !session.power_connected {
        return ElectricalReading::idle(
            CircuitStatus::Open,
            "Источник отключён: можно безопасно проверять сопротивление и менять детали.",
        );
    }
    if !session.components[1].healthy {
        return ElectricalReading::idle(
            CircuitStatus::Open,
            "Тока нет: предохранитель разомкнут. Источник пока следует проверить вольтметром.",
        );
    }
    if !session.components[3].healthy {
        return ElectricalReading::idle(
            CircuitStatus::Short,
            "Лампа замыкает цепь накоротко. Причину нужно устранить до замены предохранителя.",
        );
    }
    if !session.components[2].closed {
        return ElectricalReading::idle(
            CircuitStatus::Open,
            "Исправная цепь разомкнута выключателем.",
        );
    }
    ElectricalReading {
        status: CircuitStatus::Running,
        total_current_a: Some(0.5),
        component_current_a: Some(0.5),
        lamp_voltage_v: Some(12.0),
        lamp_power_w: Some(6.0),
        detail: "Обе неисправности устранены: ток 0,5 А, лампа горит.".to_owned(),
    }
}
