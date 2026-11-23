use std::collections::{HashSet, VecDeque};

pub const BOARD_WIDTH: f32 = 1000.0;
pub const BOARD_HEIGHT: f32 = 560.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    DcSource,
    AcSource,
    Switch,
    Lamp,
    Led,
    Resistor100,
    Resistor300,
    Resistor1000,
    Fuse,
    MainBreaker,
    Rcd,
    BranchBreaker,
    BusN,
    BusPe,
    Socket,
}

impl PartKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::DcSource => "Источник постоянного тока",
            Self::AcSource => "Ввод L / N / PE",
            Self::Switch => "Выключатель",
            Self::Lamp => "Лампа",
            Self::Led => "Светодиод",
            Self::Resistor100 => "Резистор 100 Ом",
            Self::Resistor300 => "Резистор 300 Ом",
            Self::Resistor1000 => "Резистор 1 кОм",
            Self::Fuse => "Предохранитель",
            Self::MainBreaker => "Вводной автомат",
            Self::Rcd => "УЗО",
            Self::BranchBreaker => "Групповой автомат",
            Self::BusN => "Шина N",
            Self::BusPe => "Шина PE",
            Self::Socket => "Розетка",
        }
    }

    pub fn resistance_ohms(self) -> Option<f32> {
        match self {
            Self::Resistor100 => Some(100.0),
            Self::Resistor300 => Some(300.0),
            Self::Resistor1000 => Some(1000.0),
            _ => None,
        }
    }

    pub fn is_source(self) -> bool {
        matches!(self, Self::DcSource | Self::AcSource)
    }

    pub fn is_protective(self) -> bool {
        matches!(self, Self::MainBreaker | Self::Rcd | Self::BranchBreaker)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalKind {
    Generic,
    Positive,
    Negative,
    Phase,
    Neutral,
    Protective,
}

#[derive(Clone, Debug)]
pub struct TerminalDef {
    pub label: &'static str,
    pub kind: TerminalKind,
    pub offset: (f32, f32),
}

#[derive(Clone, Debug)]
pub struct ComponentDef {
    pub id: &'static str,
    pub name: &'static str,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub terminals: Vec<TerminalDef>,
    pub accepts: &'static [PartKind],
    pub fixed: Option<PartKind>,
    pub rating_options: &'static [u32],
    pub correct_rating: Option<u32>,
    pub rating_unit: &'static str,
}

impl ComponentDef {
    pub fn accepts(&self, part: PartKind) -> bool {
        self.fixed == Some(part) || self.accepts.contains(&part)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WireDef {
    pub from_id: &'static str,
    pub from_terminal: usize,
    pub to_id: &'static str,
    pub to_terminal: usize,
}

pub const fn wire(
    from_id: &'static str,
    from_terminal: usize,
    to_id: &'static str,
    to_terminal: usize,
) -> WireDef {
    WireDef {
        from_id,
        from_terminal,
        to_id,
        to_terminal,
    }
}

#[derive(Clone, Debug)]
pub struct CircuitDef {
    pub wires: &'static [WireDef],
}

#[derive(Clone, Debug)]
pub struct LevelDef {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub theory: &'static [&'static str],
    pub hints: &'static [&'static str],
    pub palette: &'static [PartKind],
    pub components: Vec<ComponentDef>,
    pub required: &'static [(&'static str, PartKind)],
    pub circuits: &'static [CircuitDef],
}

impl LevelDef {
    pub fn component_index(&self, id: &str) -> Option<usize> {
        self.components
            .iter()
            .position(|component| component.id == id)
    }

    pub fn terminal_by_id(&self, id: &str, terminal: usize) -> Option<TerminalRef> {
        self.component_index(id).map(|component| TerminalRef {
            component,
            terminal,
        })
    }

    pub fn expected_wires(&self, circuit: usize) -> Vec<Wire> {
        self.circuits[circuit]
            .wires
            .iter()
            .filter_map(|wire| {
                Some(Wire::new(
                    self.terminal_by_id(wire.from_id, wire.from_terminal)?,
                    self.terminal_by_id(wire.to_id, wire.to_terminal)?,
                ))
            })
            .collect()
    }

    pub fn terminal_def(&self, terminal: TerminalRef) -> &TerminalDef {
        &self.components[terminal.component].terminals[terminal.terminal]
    }

    pub fn source_index(&self) -> Option<usize> {
        self.components
            .iter()
            .position(|component| component.fixed.is_some_and(PartKind::is_source))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalRef {
    pub component: usize,
    pub terminal: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wire {
    pub from: TerminalRef,
    pub to: TerminalRef,
}

impl Wire {
    pub fn new(a: TerminalRef, b: TerminalRef) -> Self {
        if a <= b {
            Self { from: a, to: b }
        } else {
            Self { from: b, to: a }
        }
    }

    pub fn touches_component(self, component: usize) -> bool {
        self.from.component == component || self.to.component == component
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Wire,
    Erase,
    Ammeter,
    Voltmeter,
    Continuity,
    Replace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedbackKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Feedback {
    pub kind: FeedbackKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelEvent {
    Overload,
    Leakage,
}

#[derive(Clone, Debug)]
pub struct ComponentState {
    pub placed: Option<PartKind>,
    pub closed: bool,
    pub healthy: bool,
    pub rating: Option<u32>,
}

impl Default for ComponentState {
    fn default() -> Self {
        Self {
            placed: None,
            closed: true,
            healthy: true,
            rating: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LevelSession {
    pub level: usize,
    pub circuit: usize,
    pub components: Vec<ComponentState>,
    pub wires: Vec<Wire>,
    pub selected_part: Option<PartKind>,
    pub selected_terminal: Option<TerminalRef>,
    pub selected_component: Option<usize>,
    pub tool: Tool,
    pub hints_used: u32,
    pub hint_cursor: usize,
    pub mistakes: u32,
    pub flags: HashSet<String>,
    pub message: Option<Feedback>,
    pub last_reading: Option<String>,
    pub power_connected: bool,
    pub panel_event: Option<PanelEvent>,
    pub completed: bool,
}

impl LevelSession {
    pub fn new(level: usize, def: &LevelDef) -> Self {
        let components: Vec<ComponentState> = def
            .components
            .iter()
            .map(|component| ComponentState {
                placed: component.fixed,
                ..ComponentState::default()
            })
            .collect();

        let mut session = Self {
            level,
            circuit: 0,
            components,
            wires: Vec::new(),
            selected_part: def.palette.first().copied(),
            selected_terminal: None,
            selected_component: None,
            tool: Tool::Wire,
            hints_used: 0,
            hint_cursor: 0,
            mistakes: 0,
            flags: HashSet::new(),
            message: None,
            last_reading: None,
            power_connected: level == 3,
            panel_event: None,
            completed: false,
        };

        if level == 3 {
            session.wires = def.expected_wires(0);
            if let Some(fuse) = def.component_index("fuse") {
                session.components[fuse].healthy = false;
            }
            if let Some(lamp) = def.component_index("lamp") {
                session.components[lamp].healthy = false;
            }
        }

        if level == 4 {
            session.power_connected = false;
        }

        session
    }

    pub fn set_feedback(&mut self, kind: FeedbackKind, text: impl Into<String>) {
        self.message = Some(Feedback {
            kind,
            text: text.into(),
        });
    }

    pub fn mistake(&mut self, text: impl Into<String>) {
        self.mistakes += 1;
        self.set_feedback(FeedbackKind::Error, text);
    }

    pub fn flag(&mut self, flag: &str) {
        self.flags.insert(flag.to_owned());
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub fn editing_blocked(&self) -> Option<&'static str> {
        if self.level == 3 {
            Some("Провода и детали диагностического стенда зафиксированы.")
        } else if self.level == 4 && self.power_connected {
            Some("Сначала отключите питание щитка. Менять схему под напряжением нельзя.")
        } else if self.panel_event.is_some() {
            Some("Сначала устраните аварию и сбросьте защиту.")
        } else {
            None
        }
    }

    pub fn place_part(
        &mut self,
        def: &LevelDef,
        component: usize,
        part: PartKind,
    ) -> Result<(), String> {
        if let Some(message) = self.editing_blocked() {
            return Err(message.to_owned());
        }
        let component_def = &def.components[component];
        if component_def.fixed.is_some() {
            return Err("Этот элемент уже закреплён на стенде.".to_owned());
        }
        if !component_def.accepts(part) {
            return Err(format!("«{}» не подходит для этого места.", part.label()));
        }
        self.wires.retain(|wire| !wire.touches_component(component));
        self.components[component] = ComponentState {
            placed: Some(part),
            closed: part != PartKind::Switch,
            healthy: true,
            rating: None,
        };
        self.selected_component = Some(component);
        Ok(())
    }

    pub fn remove_part(&mut self, def: &LevelDef, component: usize) -> Result<(), String> {
        if let Some(message) = self.editing_blocked() {
            return Err(message.to_owned());
        }
        if def.components[component].fixed.is_some() {
            return Err("Закреплённый элемент нельзя снять.".to_owned());
        }
        self.wires.retain(|wire| !wire.touches_component(component));
        self.components[component] = ComponentState::default();
        self.selected_component = None;
        Ok(())
    }

    pub fn add_wire(
        &mut self,
        def: &LevelDef,
        a: TerminalRef,
        b: TerminalRef,
    ) -> Result<(), String> {
        if let Some(message) = self.editing_blocked() {
            return Err(message.to_owned());
        }
        if a == b || a.component == b.component {
            return Err("Выберите клеммы двух разных элементов.".to_owned());
        }
        if self.components[a.component].placed.is_none()
            || self.components[b.component].placed.is_none()
        {
            return Err("Сначала установите оба элемента.".to_owned());
        }
        if a.terminal >= def.components[a.component].terminals.len()
            || b.terminal >= def.components[b.component].terminals.len()
        {
            return Err("Такой клеммы нет.".to_owned());
        }
        let wire = Wire::new(a, b);
        if self.wires.contains(&wire) {
            return Err("Эти клеммы уже соединены.".to_owned());
        }
        self.wires.push(wire);
        Ok(())
    }

    pub fn remove_wire(&mut self, index: usize) -> Result<(), String> {
        if let Some(message) = self.editing_blocked() {
            return Err(message.to_owned());
        }
        if index < self.wires.len() {
            self.wires.remove(index);
        }
        Ok(())
    }

    pub fn wiring_matches(&self, def: &LevelDef) -> bool {
        let mut actual = self.wires.clone();
        let mut expected = def.expected_wires(self.circuit);
        actual.sort();
        expected.sort();
        actual == expected
    }

    pub fn missing_parts(&self, def: &LevelDef) -> Vec<&'static str> {
        def.required
            .iter()
            .filter_map(|(id, required_part)| {
                let index = def.component_index(id)?;
                (self.components[index].placed != Some(*required_part))
                    .then_some(def.components[index].name)
            })
            .collect()
    }

    pub fn ratings_match(&self, def: &LevelDef) -> bool {
        def.components.iter().enumerate().all(|(index, component)| {
            component.correct_rating.is_none()
                || self.components[index].rating == component.correct_rating
        })
    }

    pub fn has_neutral_pe_bridge(&self, def: &LevelDef) -> bool {
        self.wires.iter().any(|wire| {
            let a = def.terminal_def(wire.from).kind;
            let b = def.terminal_def(wire.to).kind;
            matches!(
                (a, b),
                (TerminalKind::Neutral, TerminalKind::Protective)
                    | (TerminalKind::Protective, TerminalKind::Neutral)
            )
        })
    }

    pub fn has_direct_short(&self, def: &LevelDef) -> bool {
        let Some(source) = def.source_index() else {
            return false;
        };
        if def.components[source].terminals.len() < 2 {
            return false;
        }
        self.terminals_connected(
            def,
            TerminalRef {
                component: source,
                terminal: 0,
            },
            TerminalRef {
                component: source,
                terminal: 1,
            },
            false,
        )
    }

    fn terminals_connected(
        &self,
        def: &LevelDef,
        start: TerminalRef,
        goal: TerminalRef,
        include_loads: bool,
    ) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([start]);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if current == goal {
                return true;
            }
            let mut neighbours = Vec::new();
            for wire in &self.wires {
                if wire.from == current {
                    neighbours.push(wire.to);
                } else if wire.to == current {
                    neighbours.push(wire.from);
                }
            }
            for (component, state) in self.components.iter().enumerate() {
                let Some(part) = state.placed else {
                    continue;
                };
                let count = def.components[component].terminals.len();
                let mut connect = |a: usize, b: usize| {
                    if a >= count || b >= count {
                        return;
                    }
                    let a = TerminalRef {
                        component,
                        terminal: a,
                    };
                    let b = TerminalRef {
                        component,
                        terminal: b,
                    };
                    if current == a {
                        neighbours.push(b);
                    } else if current == b {
                        neighbours.push(a);
                    }
                };
                match part {
                    PartKind::Switch | PartKind::MainBreaker | PartKind::BranchBreaker
                        if state.closed =>
                    {
                        connect(0, 1)
                    }
                    PartKind::Fuse if state.healthy => connect(0, 1),
                    PartKind::Rcd if state.closed => {
                        connect(0, 2);
                        connect(1, 3);
                    }
                    PartKind::BusN | PartKind::BusPe => {
                        for a in 0..count {
                            for b in (a + 1)..count {
                                connect(a, b);
                            }
                        }
                    }
                    PartKind::Lamp | PartKind::Led if include_loads && state.healthy => {
                        connect(0, 1)
                    }
                    PartKind::Resistor100 | PartKind::Resistor300 | PartKind::Resistor1000
                        if include_loads =>
                    {
                        connect(0, 1)
                    }
                    _ => {}
                }
            }
            for neighbour in neighbours {
                if visited.insert(neighbour) {
                    queue.push_back(neighbour);
                }
            }
        }
        false
    }
}
