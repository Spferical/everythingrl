use enum_map::Enum;
use std::{
    collections::HashMap,
    fmt::Display,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

#[derive(Enum, PartialEq, Eq, Hash, Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    MeleeWeapon,
    RangedWeapon,
    Armor,
    Food,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PokemonType {
    Normal,
    Fire,
    Water,
    Electric,
    Grass,
    Ice,
    Fighting,
    Poison,
    Ground,
    Flying,
    Psychic,
    Bug,
    Rock,
    Ghost,
    Dragon,
    Dark,
    Steel,
    Fairy,
}

impl PokemonType {
    pub fn get_color(&self) -> Color {
        match self {
            PokemonType::Normal => Color::Lightgray,
            PokemonType::Fire => Color::Red,
            PokemonType::Water => Color::Blue,
            PokemonType::Electric => Color::Yellow,
            PokemonType::Grass => Color::Green,
            PokemonType::Ice => Color::Skyblue,
            PokemonType::Fighting => Color::Maroon,
            PokemonType::Poison => Color::Violet,
            PokemonType::Ground => Color::Brown,
            PokemonType::Flying => Color::Skyblue,
            PokemonType::Psychic => Color::Magenta,
            PokemonType::Bug => Color::Lime,
            PokemonType::Rock => Color::Brown,
            PokemonType::Ghost => Color::Purple,
            PokemonType::Dragon => Color::Orange,
            PokemonType::Dark => Color::Black,
            PokemonType::Steel => Color::Lightgray,
            PokemonType::Fairy => Color::Pink,
        }
    }

    pub fn get_effectiveness(self, defense: PokemonType) -> AttackEffectiveness {
        use AttackEffectiveness::*;
        use PokemonType::*;
        let attack = self;
        match (attack, defense) {
            (Normal, Rock | Steel) => Half,
            (Normal, Ghost) => Zero,

            (Fire, Fire | Water | Rock | Dragon) => Half,
            (Fire, Grass | Ice | Bug | Steel) => Two,

            (Water, Water | Grass | Dragon) => Half,
            (Water, Fire | Ground | Rock) => Two,

            (Electric, Water | Flying) => Two,
            (Electric, Electric | Grass) => Half,
            (Electric, Ground) => Zero,

            (Grass, Water | Ground | Rock) => Two,
            (Grass, Fire | Grass | Poison | Flying | Bug | Dragon | Steel) => Half,

            (Ice, Grass | Ground | Flying | Dragon) => Two,
            (Ice, Fire | Water | Ice | Steel) => Half,

            (Fighting, Ice | Rock | Normal | Dark | Steel) => Two,
            (Fighting, Flying | Poison | Bug | Psychic | Fairy) => Half,
            (Fighting, Ghost) => Zero,

            (Poison, Grass | Fairy) => Two,
            (Poison, Poison | Ground | Rock | Ghost) => Half,
            (Poison, Steel) => Zero,

            (Ground, Fire | Electric | Poison | Rock | Steel) => Two,
            (Ground, Grass | Bug) => Half,
            (Ground, Flying) => Zero,

            (Flying, Grass | Fighting | Bug) => Two,
            (Flying, Electric | Rock | Steel) => Half,

            (Psychic, Fighting | Poison) => Two,
            (Psychic, Psychic | Steel) => Half,
            (Psychic, Dark) => Zero,

            (Bug, Grass | Psychic | Dark) => Two,
            (Bug, Fire | Fighting | Poison | Flying | Ghost | Steel | Fairy) => Half,

            (Rock, Fire | Ice | Flying | Bug) => Two,
            (Rock, Fighting | Ground | Steel) => Half,

            (Ghost, Psychic | Ghost) => Two,
            (Ghost, Dark) => Half,
            (Ghost, Normal) => Zero,

            (Dragon, Dragon) => Two,
            (Dragon, Steel) => Half,
            (Dragon, Fairy) => Zero,

            (Dark, Psychic | Ghost) => Two,
            (Dark, Fighting | Dark | Fairy) => Half,

            (Steel, Ice | Rock | Fairy) => Two,
            (Steel, Fire | Water | Electric | Steel) => Half,

            (Fairy, Fighting | Dragon | Dark) => Two,
            (Fairy, Fire | Poison | Steel) => Half,

            _ => One,
        }
    }
    pub fn get_effectiveness2(
        self: PokemonType,
        defense1: PokemonType,
        defense2: Option<PokemonType>,
    ) -> AttackEffectiveness {
        use AttackEffectiveness::*;
        let attack = self;
        let eff1 = attack.get_effectiveness(defense1);
        let eff2 = defense2.map(|defense2| attack.get_effectiveness(defense2));
        multiply_effectiveness(eff1, eff2.unwrap_or(One))
    }
}
pub enum AttackEffectiveness {
    Zero,
    Quarter,
    Half,
    One,
    Two,
    Four,
}

impl AttackEffectiveness {
    pub fn get_scale(&self) -> usize {
        match self {
            AttackEffectiveness::Zero => 0,
            AttackEffectiveness::Quarter => 1,
            AttackEffectiveness::Half => 2,
            AttackEffectiveness::One => 4,
            AttackEffectiveness::Two => 8,
            AttackEffectiveness::Four => 16,
        }
    }
}

fn multiply_effectiveness(
    eff1: AttackEffectiveness,
    eff2: AttackEffectiveness,
) -> AttackEffectiveness {
    use AttackEffectiveness::*;
    match (eff1, eff2) {
        (Zero, _) | (_, Zero) => Zero,
        (Half, Half) => Quarter,
        (Half, Two) | (Two, Half) => One,
        (Two, Two) => Four,
        (eff1, One) => eff1,
        (One, eff2) => eff2,
        _ => One,
    }
}

impl Display for PokemonType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PokemonType::Normal => write!(f, "Normal"),
            PokemonType::Fire => write!(f, "Fire"),
            PokemonType::Water => write!(f, "Water"),
            PokemonType::Electric => write!(f, "Electric"),
            PokemonType::Grass => write!(f, "Grass"),
            PokemonType::Ice => write!(f, "Ice"),
            PokemonType::Fighting => write!(f, "Fighting"),
            PokemonType::Poison => write!(f, "Poison"),
            PokemonType::Ground => write!(f, "Ground"),
            PokemonType::Flying => write!(f, "Flying"),
            PokemonType::Psychic => write!(f, "Psychic"),
            PokemonType::Bug => write!(f, "Bug"),
            PokemonType::Rock => write!(f, "Rock"),
            PokemonType::Ghost => write!(f, "Ghost"),
            PokemonType::Dragon => write!(f, "Dragon"),
            PokemonType::Dark => write!(f, "Dark"),
            PokemonType::Steel => write!(f, "Steel"),
            PokemonType::Fairy => write!(f, "Fairy"),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    Lightgray,
    Gray,
    Grey,
    Silver,
    Black,
    Yellow,
    Gold,
    Orange,
    Pink,
    Red,
    Maroon,
    Green,
    Lime,
    Skyblue,
    Blue,
    Purple,
    Violet,
    Beige,
    Brown,
    White,
    Magenta,
}

impl From<Color> for macroquad::color::Color {
    fn from(value: Color) -> Self {
        match value {
            Color::Lightgray => macroquad::color::LIGHTGRAY,
            Color::Yellow => macroquad::color::YELLOW,
            Color::Gold => macroquad::color::GOLD,
            Color::Orange => macroquad::color::ORANGE,
            Color::Pink => macroquad::color::PINK,
            Color::Red => macroquad::color::RED,
            Color::Maroon => macroquad::color::MAROON,
            Color::Green => macroquad::color::GREEN,
            Color::Lime => macroquad::color::LIME,
            Color::Skyblue => macroquad::color::SKYBLUE,
            Color::Blue => macroquad::color::BLUE,
            Color::Purple => macroquad::color::PURPLE,
            Color::Violet => macroquad::color::VIOLET,
            Color::Beige => macroquad::color::BEIGE,
            Color::Brown => macroquad::color::BROWN,
            Color::White => macroquad::color::WHITE,
            Color::Magenta => macroquad::color::MAGENTA,
            // These are bad, but the AI sometimes generates them.
            Color::Gray => macroquad::color::LIGHTGRAY,
            Color::Grey => macroquad::color::LIGHTGRAY,
            Color::Silver => macroquad::color::LIGHTGRAY,
            Color::Black => macroquad::color::LIGHTGRAY,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MonsterDefinition {
    pub name: String,
    pub char: String,
    pub color: Color,
    pub attack_type: PokemonType,
    pub type1: PokemonType,
    pub type2: Option<PokemonType>,
    pub description: String,
    pub level: usize,
    pub seen: String,
    pub attack: String,
    pub death: String,
    pub ranged: bool,
    pub speed: u8,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BossDefinition {
    pub name: String,
    pub char: String,
    pub color: Color,
    pub attack_type: PokemonType,
    pub type1: PokemonType,
    pub type2: Option<PokemonType>,
    pub description: String,
    pub intro_message: String,
    pub attack_message: String,
    pub periodic_messages: Vec<String>,
    pub game_victory_paragraph: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ItemDefinition {
    pub name: String,
    pub level: usize,
    #[serde(rename = "type")]
    pub ty: PokemonType,
    pub description: String,
    pub kind: ItemKind,
    pub craft_id: Option<CraftId>,
}

#[derive(Enum, PartialEq, Eq, Hash, Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MapGen {
    SimpleRoomsAndCorridors,
    Caves,
    Hive,
    DenseRooms,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Area {
    pub name: String,
    pub blurb: String,
    pub mapgen: MapGen,
    pub enemies: Vec<String>,
    pub equipment: Vec<String>,
    pub melee_weapons: Vec<String>,
    pub ranged_weapons: Vec<String>,
    pub food: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonstersArgs {
    theme: String,
    setting: String,
    areas: Vec<Area>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CraftArgs {
    theme: String,
    setting: String,
    items: Vec<ItemDefinition>,
    item1: ItemDefinition,
    item2: ItemDefinition,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AreasArgs {
    theme: String,
    setting: String,
}

pub struct BootlegFuture<T> {
    rx: Receiver<T>,
    state: Option<T>,
}

impl<T> BootlegFuture<T> {
    fn get(&mut self) -> &Option<T> {
        if self.state.is_none() {
            if let Ok(result) = self.rx.try_recv() {
                self.state = Some(result);
            }
        }
        &self.state
    }
}

pub fn request<Input>(url: String, input: Input) -> BootlegFuture<Result<String, String>>
where
    Input: serde::Serialize + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    #[cfg(target_family = "wasm")]
    crate::wasm::post(url, serde_json::to_string(&input).unwrap(), tx);

    #[cfg(not(target_family = "wasm"))]
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        tx.send(
            client
                .post(url)
                .json(&input)
                .timeout(Duration::from_secs(60 * 3))
                .send()
                .map_err(|e| e.to_string())
                .and_then(|r| r.error_for_status().map_err(|e| e.to_string()))
                .and_then(|r| r.text().map_err(|e| e.to_string())),
        )
        .ok();
    });
    BootlegFuture { rx, state: None }
}

pub fn api_url() -> String {
    std::env::var("SERVER_URL").unwrap_or("http://localhost:5000".into())
}

#[derive(Debug, Clone)]
pub enum Request {
    Setting,
    Areas,
    Monsters,
    Items,
    Craft {
        item1: usize,
        item2: usize,
        craft_id: CraftId,
    },
    Boss,
}

impl Request {}

pub struct PendingRequest {
    req: Request,
    fut: BootlegFuture<Result<String, String>>,
}

enum RequestResult {
    Setting(String),
    Areas(Vec<Area>),
    Monsters(Vec<MonsterDefinition>),
    Items(Vec<ItemDefinition>),
    Craft(ItemDefinition),
    Boss(BossDefinition),
    Pending,
    Error(String),
}

impl PendingRequest {
    fn get(&mut self) -> RequestResult {
        use RequestResult::*;
        let resp = self.fut.get();
        match resp {
            None => Pending,
            Some(Err(s)) => Error(s.clone()),
            Some(Ok(s)) => match self.req {
                Request::Setting { .. } => serde_json::from_str(s)
                    .map(Setting)
                    .unwrap_or_else(|e| Error(e.to_string())),
                Request::Areas { .. } => serde_json::from_str(s)
                    .map(Areas)
                    .unwrap_or_else(|e| Error(e.to_string())),
                Request::Monsters { .. } => serde_json::from_str(s)
                    .map(Monsters)
                    .unwrap_or_else(|e| Error(e.to_string())),
                Request::Items { .. } => serde_json::from_str(s)
                    .map(Items)
                    .unwrap_or_else(|e| Error(e.to_string())),
                Request::Craft { .. } => serde_json::from_str(s)
                    .map(Craft)
                    .unwrap_or_else(|e| Error(e.to_string())),
                Request::Boss { .. } => serde_json::from_str(s)
                    .map(Boss)
                    .unwrap_or_else(|e| Error(e.to_string())),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CraftId(usize);

/// Contains raw AI-generated content fetched from the server.
pub struct IdeaGuy {
    pub theme: String,
    pub api_url: String,
    pub setting: Option<String>,
    pub areas: Option<Vec<Area>>,
    pub monsters: Option<Vec<MonsterDefinition>>,
    pub items: Option<Vec<ItemDefinition>>,
    pub boss: Option<BossDefinition>,
    // keys/vals are indices into items.
    pub recipes: HashMap<(usize, usize), usize>,
    outgoing: Vec<PendingRequest>,
    pub next_craft_id: CraftId,
}

impl IdeaGuy {
    pub fn new(theme: &str) -> Self {
        let api_url = api_url();
        let mut slf = Self {
            theme: theme.into(),
            api_url,
            setting: None,
            areas: None,
            monsters: None,
            items: None,
            boss: None,
            outgoing: vec![],
            recipes: HashMap::new(),
            next_craft_id: CraftId(0),
        };
        slf.request(Request::Setting);
        slf
    }

    pub fn craft(&mut self, item1: usize, item2: usize) {
        let craft_id = self.next_craft_id;
        self.next_craft_id = CraftId(self.next_craft_id.0 + 1);
        self.request(Request::Craft {
            item1,
            item2,
            craft_id,
        });
    }

    fn request_inner(&mut self, req: Request) -> PendingRequest {
        let api_url = api_url();
        let fut = match req {
            Request::Setting => {
                let theme = &self.theme;
                request(format!("{api_url}/setting/{theme}"), "")
            }
            Request::Areas => request(
                format!("{api_url}/areas"),
                AreasArgs {
                    theme: self.theme.clone(),
                    setting: self.setting.clone().unwrap(),
                },
            ),
            Request::Boss => request(
                format!("{api_url}/boss"),
                AreasArgs {
                    theme: self.theme.clone(),
                    setting: self.setting.clone().unwrap(),
                },
            ),
            Request::Monsters => request(
                format!("{api_url}/monsters"),
                MonstersArgs {
                    theme: self.theme.clone(),
                    setting: self.setting.clone().unwrap(),
                    areas: self.areas.clone().unwrap(),
                },
            ),
            Request::Items => request(
                format!("{api_url}/items"),
                MonstersArgs {
                    theme: self.theme.clone(),
                    setting: self.setting.clone().unwrap(),
                    areas: self.areas.clone().unwrap(),
                },
            ),
            Request::Craft { item1, item2, .. } => request(
                format!("{api_url}/craft"),
                CraftArgs {
                    theme: self.theme.clone(),
                    setting: self.setting.clone().unwrap(),
                    items: self.items.clone().unwrap(),
                    item1: self.items.as_ref().unwrap()[item1].clone(),
                    item2: self.items.as_ref().unwrap()[item2].clone(),
                },
            ),
        };
        PendingRequest { req, fut }
    }

    pub fn request(&mut self, req: Request) {
        let pending_req = self.request_inner(req);
        self.outgoing.push(pending_req);
    }

    pub fn tick(&mut self) {
        let mut queue = self.outgoing.drain(..).rev().collect::<Vec<_>>();
        while let Some(mut req) = queue.pop() {
            match req.get() {
                RequestResult::Error(e) => {
                    macroquad::miniquad::error!("{}", e);
                    // Retry
                    self.request(req.req);
                }
                RequestResult::Pending => {
                    self.outgoing.push(req);
                }
                RequestResult::Setting(s) => {
                    self.setting = Some(s);
                    self.request(Request::Areas);
                }
                RequestResult::Areas(areas) => {
                    self.areas = Some(areas);
                    self.request(Request::Monsters);
                }
                RequestResult::Monsters(monsters) => {
                    self.monsters = Some(monsters);
                    self.request(Request::Items);
                }
                RequestResult::Items(items) => {
                    self.items = Some(items);
                    self.request(Request::Boss);
                }
                RequestResult::Boss(boss) => {
                    self.boss = Some(boss);
                }
                RequestResult::Craft(mut item) => {
                    if let Request::Craft {
                        item1,
                        item2,
                        craft_id,
                    } = req.req
                    {
                        item.craft_id = Some(craft_id);
                        self.recipes
                            .insert((item1, item2), self.items.as_ref().unwrap().len());
                    }
                    if let Some(items) = self.items.as_mut() {
                        items.push(item)
                    }
                }
            }
        }
    }
}
