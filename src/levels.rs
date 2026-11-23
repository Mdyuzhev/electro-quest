use crate::model::{CircuitDef, ComponentDef, LevelDef, PartKind, TerminalDef, TerminalKind, wire};

const NO_PARTS: &[PartKind] = &[];
const NO_RATINGS: &[u32] = &[];
const SWITCH_LAMP: &[PartKind] = &[PartKind::Switch, PartKind::Lamp];
const RESISTORS_LED: &[PartKind] = &[
    PartKind::Resistor100,
    PartKind::Resistor300,
    PartKind::Resistor1000,
    PartKind::Led,
];
const LAMPS: &[PartKind] = &[PartKind::Lamp];
const PANEL_PARTS: &[PartKind] = &[
    PartKind::MainBreaker,
    PartKind::Rcd,
    PartKind::BranchBreaker,
    PartKind::Lamp,
    PartKind::Socket,
];
const MAIN_RATINGS: &[u32] = &[10, 20, 40];
const RCD_RATINGS: &[u32] = &[10, 30, 100];
const LIGHT_RATINGS: &[u32] = &[6, 10, 25];
const SOCKET_RATINGS: &[u32] = &[10, 16, 32];

fn component(
    id: &'static str,
    name: &'static str,
    position: (f32, f32),
    size: (f32, f32),
    terminals: Vec<TerminalDef>,
    accepts: &'static [PartKind],
    fixed: Option<PartKind>,
) -> ComponentDef {
    ComponentDef {
        id,
        name,
        position,
        size,
        terminals,
        accepts,
        fixed,
        rating_options: NO_RATINGS,
        correct_rating: None,
        rating_unit: "",
    }
}

fn two_terminal(
    id: &'static str,
    name: &'static str,
    position: (f32, f32),
    size: (f32, f32),
    accepts: &'static [PartKind],
    fixed: Option<PartKind>,
) -> ComponentDef {
    let (width, _) = size;
    component(
        id,
        name,
        position,
        size,
        vec![
            TerminalDef {
                label: "1",
                kind: TerminalKind::Generic,
                offset: (-width / 2.0, 0.0),
            },
            TerminalDef {
                label: "2",
                kind: TerminalKind::Generic,
                offset: (width / 2.0, 0.0),
            },
        ],
        accepts,
        fixed,
    )
}

fn phase_two_terminal(
    id: &'static str,
    name: &'static str,
    position: (f32, f32),
    size: (f32, f32),
    accepts: &'static [PartKind],
    fixed: Option<PartKind>,
) -> ComponentDef {
    let mut component = two_terminal(id, name, position, size, accepts, fixed);
    for (index, terminal) in component.terminals.iter_mut().enumerate() {
        terminal.kind = TerminalKind::Phase;
        terminal.label = if index == 0 { "L in" } else { "L out" };
    }
    component
}

fn dc_source(id: &'static str, name: &'static str, position: (f32, f32)) -> ComponentDef {
    component(
        id,
        name,
        position,
        (130.0, 120.0),
        vec![
            TerminalDef {
                label: "+",
                kind: TerminalKind::Positive,
                offset: (65.0, -28.0),
            },
            TerminalDef {
                label: "−",
                kind: TerminalKind::Negative,
                offset: (65.0, 28.0),
            },
        ],
        NO_PARTS,
        Some(PartKind::DcSource),
    )
}

fn rated(
    mut component: ComponentDef,
    options: &'static [u32],
    correct: u32,
    unit: &'static str,
) -> ComponentDef {
    component.rating_options = options;
    component.correct_rating = Some(correct);
    component.rating_unit = unit;
    component
}

const LEVEL1_CIRCUITS: &[CircuitDef] = &[CircuitDef {
    wires: &[
        wire("source", 0, "switch", 0),
        wire("switch", 1, "lamp", 0),
        wire("lamp", 1, "source", 1),
    ],
}];

const LEVEL2_CIRCUITS: &[CircuitDef] = &[CircuitDef {
    wires: &[
        wire("source", 0, "resistor", 0),
        wire("resistor", 1, "led", 0),
        wire("led", 1, "source", 1),
    ],
}];

const LEVEL3_CIRCUITS: &[CircuitDef] = &[
    CircuitDef {
        wires: &[
            wire("source", 0, "lamp1", 0),
            wire("lamp1", 1, "lamp2", 0),
            wire("lamp2", 1, "source", 1),
        ],
    },
    CircuitDef {
        wires: &[
            wire("source", 0, "lamp1", 0),
            wire("source", 0, "lamp2", 0),
            wire("lamp1", 1, "source", 1),
            wire("lamp2", 1, "source", 1),
        ],
    },
];

const LEVEL4_CIRCUITS: &[CircuitDef] = &[CircuitDef {
    wires: &[
        wire("source", 0, "fuse", 0),
        wire("fuse", 1, "switch", 0),
        wire("switch", 1, "lamp", 0),
        wire("lamp", 1, "source", 1),
    ],
}];

const LEVEL5_CIRCUITS: &[CircuitDef] = &[CircuitDef {
    wires: &[
        wire("source", 0, "main", 0),
        wire("main", 1, "rcd", 0),
        wire("source", 1, "rcd", 1),
        wire("rcd", 2, "light_breaker", 0),
        wire("rcd", 2, "socket_breaker", 0),
        wire("rcd", 3, "bus_n", 0),
        wire("light_breaker", 1, "lamp", 0),
        wire("lamp", 1, "bus_n", 1),
        wire("socket_breaker", 1, "socket", 0),
        wire("socket", 1, "bus_n", 2),
        wire("source", 2, "bus_pe", 0),
        wire("socket", 2, "bus_pe", 1),
    ],
}];

fn level1() -> LevelDef {
    LevelDef {
        title: "Первый свет",
        subtitle: "Собери замкнутую цепь и зажги лампу",
        theory: &[
            "Электрический ток идёт по замкнутому пути от источника через нагрузку и обратно.",
            "Разомкнутый выключатель разрывает цепь. Провод должен соединять именно клеммы деталей.",
            "В этой модели источник имеет напряжение 6 В, а лампа — сопротивление 12 Ом.",
        ],
        hints: &[
            "Установи выключатель и лампу в пустые рамки на монтажном столе.",
            "Нужны три провода: источник > выключатель > лампа > источник.",
            "После соединения нажми на выключатель, чтобы замкнуть его контакты.",
        ],
        palette: SWITCH_LAMP,
        components: vec![
            dc_source("source", "Батарея 6 В", (130.0, 280.0)),
            two_terminal(
                "switch",
                "Выключатель",
                (480.0, 120.0),
                (150.0, 90.0),
                &[PartKind::Switch],
                None,
            ),
            two_terminal(
                "lamp",
                "Лампа 12 Ом",
                (800.0, 280.0),
                (125.0, 120.0),
                &[PartKind::Lamp],
                None,
            ),
        ],
        required: &[("switch", PartKind::Switch), ("lamp", PartKind::Lamp)],
        circuits: LEVEL1_CIRCUITS,
    }
}

fn level2() -> LevelDef {
    LevelDef {
        title: "Подбери сопротивление",
        subtitle: "Защити светодиод с помощью закона Ома",
        theory: &[
            "Закон Ома: I = U / R. Сила тока измеряется в амперах, напряжение — в вольтах, сопротивление — в омах.",
            "На светодиоде остаётся 2 В, поэтому на резисторе: 5 В − 2 В = 3 В.",
            "Для тока 10 мА = 0,01 А нужно R = 3 В / 0,01 А = 300 Ом. Анод светодиода подключается к плюсу через резистор.",
        ],
        hints: &[
            "Сначала переведи 10 мА в амперы: 10 мА = 0,01 А.",
            "На резисторе будет 3 В. Раздели 3 В на 0,01 А.",
            "Выбери резистор 300 Ом и соедини цепь в том же порядке, что в первом уровне.",
        ],
        palette: RESISTORS_LED,
        components: vec![
            dc_source("source", "Источник 5 В", (130.0, 280.0)),
            two_terminal(
                "resistor",
                "Место резистора",
                (480.0, 120.0),
                (160.0, 90.0),
                &[
                    PartKind::Resistor100,
                    PartKind::Resistor300,
                    PartKind::Resistor1000,
                ],
                None,
            ),
            two_terminal(
                "led",
                "Светодиод 2 В",
                (800.0, 280.0),
                (130.0, 115.0),
                &[PartKind::Led],
                None,
            ),
        ],
        required: &[("resistor", PartKind::Resistor300), ("led", PartKind::Led)],
        circuits: LEVEL2_CIRCUITS,
    }
}

fn level3() -> LevelDef {
    LevelDef {
        title: "Две лампы",
        subtitle: "Сравни последовательное и параллельное соединение",
        theory: &[
            "При последовательном соединении через все элементы идёт один и тот же ток, а напряжение делится между ними.",
            "При параллельном соединении напряжение на каждой ветви одинаково, а токи ветвей складываются.",
            "Амперметр включают последовательно с нагрузкой, вольтметр — параллельно ей. В игре приборы подключаются к выбранной лампе автоматически.",
            "Обе лампы имеют сопротивление 24 Ом, источник — 12 В.",
        ],
        hints: &[
            "Последовательно: источник > первая лампа > вторая лампа > источник. Один путь для тока.",
            "После сборки выбери амперметр и нажми на лампу, затем вольтметр и снова нажми на лампу.",
            "Параллельно: плюс источника соединяется с входом каждой лампы, а выход каждой лампы — с минусом.",
        ],
        palette: LAMPS,
        components: vec![
            dc_source("source", "Источник 12 В", (110.0, 280.0)),
            two_terminal(
                "lamp1",
                "Лампа 1 · 24 Ом",
                (480.0, 140.0),
                (135.0, 115.0),
                &[PartKind::Lamp],
                None,
            ),
            two_terminal(
                "lamp2",
                "Лампа 2 · 24 Ом",
                (480.0, 420.0),
                (135.0, 115.0),
                &[PartKind::Lamp],
                None,
            ),
        ],
        required: &[("lamp1", PartKind::Lamp), ("lamp2", PartKind::Lamp)],
        circuits: LEVEL3_CIRCUITS,
    }
}

fn level4() -> LevelDef {
    LevelDef {
        title: "Найди неисправность",
        subtitle: "Диагностируй стенд и устрани причину отказа",
        theory: &[
            "Лампа не горит: предохранитель разомкнут, а лампа имеет внутреннее короткое замыкание. Нужно найти обе неисправности.",
            "Напряжение измеряют на включённом источнике. Сопротивление и целостность цепи — только после отключения источника.",
            "Нельзя просто заменить предохранитель: сначала устраняют причину короткого замыкания. Исправная лампа имеет сопротивление 24 Ом.",
        ],
        hints: &[
            "Выбери вольтметр и проверь источник. Затем отключи источник кнопкой под схемой.",
            "В режиме проверки целостности нажми на предохранитель и лампу. Результат «Обрыв» означает разрыв цепи, а 0 Ом у лампы — короткое замыкание.",
            "Отключив источник, выбери замену детали: сначала лампу, затем предохранитель. После этого подключи источник.",
        ],
        palette: NO_PARTS,
        components: vec![
            dc_source("source", "Источник 12 В", (100.0, 290.0)),
            two_terminal(
                "fuse",
                "Предохранитель",
                (350.0, 120.0),
                (140.0, 85.0),
                NO_PARTS,
                Some(PartKind::Fuse),
            ),
            two_terminal(
                "switch",
                "Выключатель",
                (610.0, 120.0),
                (140.0, 85.0),
                NO_PARTS,
                Some(PartKind::Switch),
            ),
            two_terminal(
                "lamp",
                "Лампа 24 Ом",
                (850.0, 290.0),
                (125.0, 115.0),
                NO_PARTS,
                Some(PartKind::Lamp),
            ),
        ],
        required: &[],
        circuits: LEVEL4_CIRCUITS,
    }
}

fn level5() -> LevelDef {
    let source = component(
        "source",
        "Однофазный ввод",
        (75.0, 260.0),
        (110.0, 160.0),
        vec![
            TerminalDef {
                label: "L",
                kind: TerminalKind::Phase,
                offset: (55.0, -55.0),
            },
            TerminalDef {
                label: "N",
                kind: TerminalKind::Neutral,
                offset: (55.0, 0.0),
            },
            TerminalDef {
                label: "PE",
                kind: TerminalKind::Protective,
                offset: (55.0, 55.0),
            },
        ],
        NO_PARTS,
        Some(PartKind::AcSource),
    );
    let rcd = rated(
        component(
            "rcd",
            "УЗО 25 А",
            (410.0, 165.0),
            (135.0, 165.0),
            vec![
                TerminalDef {
                    label: "L in",
                    kind: TerminalKind::Phase,
                    offset: (-67.5, -42.0),
                },
                TerminalDef {
                    label: "N in",
                    kind: TerminalKind::Neutral,
                    offset: (-67.5, 42.0),
                },
                TerminalDef {
                    label: "L out",
                    kind: TerminalKind::Phase,
                    offset: (67.5, -42.0),
                },
                TerminalDef {
                    label: "N out",
                    kind: TerminalKind::Neutral,
                    offset: (67.5, 42.0),
                },
            ],
            &[PartKind::Rcd],
            None,
        ),
        RCD_RATINGS,
        30,
        "мА",
    );
    let lamp = component(
        "lamp",
        "Свет · 300 Вт",
        (850.0, 75.0),
        (125.0, 110.0),
        vec![
            TerminalDef {
                label: "L",
                kind: TerminalKind::Phase,
                offset: (-62.5, -25.0),
            },
            TerminalDef {
                label: "N",
                kind: TerminalKind::Neutral,
                offset: (-62.5, 25.0),
            },
        ],
        &[PartKind::Lamp],
        None,
    );
    let socket = component(
        "socket",
        "Розетки · 12 А",
        (850.0, 285.0),
        (130.0, 140.0),
        vec![
            TerminalDef {
                label: "L",
                kind: TerminalKind::Phase,
                offset: (-65.0, -45.0),
            },
            TerminalDef {
                label: "N",
                kind: TerminalKind::Neutral,
                offset: (-65.0, 0.0),
            },
            TerminalDef {
                label: "PE",
                kind: TerminalKind::Protective,
                offset: (-65.0, 45.0),
            },
        ],
        &[PartKind::Socket],
        None,
    );
    let bus_n = component(
        "bus_n",
        "Нулевая шина N",
        (450.0, 390.0),
        (160.0, 70.0),
        vec![
            TerminalDef {
                label: "N",
                kind: TerminalKind::Neutral,
                offset: (-50.0, -35.0),
            },
            TerminalDef {
                label: "N",
                kind: TerminalKind::Neutral,
                offset: (0.0, -35.0),
            },
            TerminalDef {
                label: "N",
                kind: TerminalKind::Neutral,
                offset: (50.0, -35.0),
            },
        ],
        NO_PARTS,
        Some(PartKind::BusN),
    );
    let bus_pe = component(
        "bus_pe",
        "Шина защитного PE",
        (680.0, 490.0),
        (170.0, 60.0),
        vec![
            TerminalDef {
                label: "PE",
                kind: TerminalKind::Protective,
                offset: (-50.0, -30.0),
            },
            TerminalDef {
                label: "PE",
                kind: TerminalKind::Protective,
                offset: (50.0, -30.0),
            },
        ],
        NO_PARTS,
        Some(PartKind::BusPe),
    );

    LevelDef {
        title: "Электрощиток",
        subtitle: "Собери однолинейную схему и проверь защиту",
        theory: &[
            "Это упрощённая однолинейная учебная схема, а не инструкция для реального монтажа. Все изменения выполняются только при отключённом питании.",
            "Автомат защищает проводку от сверхтока. УЗО сравнивает токи L и N и отключает цепь при утечке; оно не заменяет автомат.",
            "Для учебной нагрузки выбраны: вводной автомат 20 А, УЗО 25 А с порогом 30 мА, автомат света 10 А и автомат розеток 16 А.",
            "N — рабочий нулевой проводник, PE — защитный. После УЗО объединять N и PE нельзя. Розетка обязательно подключается к PE.",
        ],
        hints: &[
            "Собери цепь L: ввод > автомат 20 А > УЗО > оба групповых автомата > нагрузки.",
            "N проходит через УЗО на синюю шину. PE идёт отдельно от ввода к жёлто-зелёной шине и розетке; N и PE не соединяются.",
            "Выбери номиналы в панели справа, подай питание и проверь нормальную нагрузку. Затем запусти перегрузку и утечку, после каждой аварии нажимая сброс.",
        ],
        palette: PANEL_PARTS,
        components: vec![
            source,
            rated(
                phase_two_terminal(
                    "main",
                    "Вводной автомат L",
                    (230.0, 150.0),
                    (115.0, 125.0),
                    &[PartKind::MainBreaker],
                    None,
                ),
                MAIN_RATINGS,
                20,
                "А",
            ),
            rcd,
            rated(
                phase_two_terminal(
                    "light_breaker",
                    "Автомат света",
                    (635.0, 75.0),
                    (115.0, 115.0),
                    &[PartKind::BranchBreaker],
                    None,
                ),
                LIGHT_RATINGS,
                10,
                "А",
            ),
            rated(
                phase_two_terminal(
                    "socket_breaker",
                    "Автомат розеток",
                    (635.0, 285.0),
                    (115.0, 115.0),
                    &[PartKind::BranchBreaker],
                    None,
                ),
                SOCKET_RATINGS,
                16,
                "А",
            ),
            lamp,
            socket,
            bus_n,
            bus_pe,
        ],
        required: &[
            ("main", PartKind::MainBreaker),
            ("rcd", PartKind::Rcd),
            ("light_breaker", PartKind::BranchBreaker),
            ("socket_breaker", PartKind::BranchBreaker),
            ("lamp", PartKind::Lamp),
            ("socket", PartKind::Socket),
        ],
        circuits: LEVEL5_CIRCUITS,
    }
}

pub fn all_levels() -> Vec<LevelDef> {
    vec![level1(), level2(), level3(), level4(), level5()]
}
