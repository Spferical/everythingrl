use enum_map::Enum;
use std::{
    collections::HashMap,
    fmt::Display,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crate::ratelimit::Ratelimiter;
use crate::util::spawn;

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
#[derive(Debug, Clone, Copy)]
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

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
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

impl From<Color> for egui::Color32 {
    fn from(value: Color) -> Self {
        let color = macroquad::color::Color::from(value);
        let [r, g, b, _a] = color.into();
        egui::Color32::from_rgb(r, g, b)
    }
}

impl From<&Color> for egui::Color32 {
    fn from(value: &Color) -> Self {
        egui::Color32::from(*value)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BossDefinition {
    pub name: String,
    pub char: String,
    pub color: Color,
    pub attack_type: PokemonType,
    pub type1: PokemonType,
    pub type2: Option<PokemonType>,
    pub description: String,
    pub intro_message: String,
    pub attack_messages: Vec<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
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
pub struct Character {
    pub name: String,
    pub backstory: String,
    pub starting_items: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonstersArgs {
    theme: String,
    setting: String,
    // names we are requesting
    names: Vec<String>,
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

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u32,
    pub data: String,
}

/// API client that ratelimits requests.
pub struct ApiClient {
    ratelimit: Ratelimiter,
}
impl ApiClient {
    pub fn new() -> Self {
        Self {
            ratelimit: Ratelimiter::new(Duration::from_secs(1)),
        }
    }
    pub fn request<Input>(
        &self,
        url: String,
        input: Input,
    ) -> BootlegFuture<Result<Response, String>>
    where
        Input: serde::Serialize + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let _rl = self.ratelimit.clone();
        spawn(async move {
            _rl.wait().await;
            // NOTE: wasm reqwest doesn't support timeouts.
            let resp = reqwest::Client::new().post(url).json(&input).send().await;
            tx.send(match resp {
                Ok(r) => Ok(Response {
                    status: r.status().as_u16().into(),
                    data: r.text().await.unwrap_or_default(),
                }),
                Err(err) => Err(format_full_error_chain(err)),
            })
            .ok();
        });

        BootlegFuture { rx, state: None }
    }
}

fn format_full_error_chain(err: impl std::error::Error) -> String {
    let mut err_string = err.to_string();
    let mut err: &dyn std::error::Error = &err as _;
    while let Some(source) = err.source() {
        err_string += "\n";
        err_string += &source.to_string();
        err = source;
    }
    err_string
}

pub fn api_url() -> String {
    std::env::var("SERVER_URL").unwrap_or("https://7drl24.pfe.io".into())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AnythingRequest {
    state: GameDefs,
    ask: String,
}

#[derive(Debug, Clone)]
pub enum Request {
    Craft {
        item1: usize,
        item2: usize,
        craft_id: CraftId,
    },
    Anything(AnythingRequest),
}

impl Request {}

pub struct PendingRequest {
    req: Request,
    fut: BootlegFuture<Result<Response, String>>,
}

enum RequestResult {
    Craft(ItemDefinition),
    Anything(GameDefs),
    Pending,
    Error(String),
}

#[derive(serde::Deserialize)]
struct ServerError {
    error: String,
}

impl PendingRequest {
    fn get(&mut self) -> RequestResult {
        use RequestResult::*;
        let resp = self.fut.get();
        match resp {
            None => Pending,
            Some(Err(s)) => Error(s.clone()),
            Some(Ok(resp)) => {
                macroquad::miniquad::info!("{:?}", resp);
                if resp.status >= 400 {
                    // try decoding full server error
                    return Error(
                        serde_json::from_str::<ServerError>(&resp.data)
                            .map(|e| e.error)
                            .unwrap_or(crate::util::trim(&resp.data, 100).into()),
                    );
                }
                match self.req {
                    Request::Anything { .. } => serde_json::from_str(&resp.data)
                        .map(Anything)
                        .unwrap_or_else(|e| Error(e.to_string())),
                    Request::Craft { .. } => serde_json::from_str(&resp.data)
                        .map(Craft)
                        .unwrap_or_else(|e| Error(e.to_string())),
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CraftId(usize);

pub enum IgState {
    Generating(&'static str),
    Idle,
    Error { msg: String, count: usize },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameDefs {
    pub theme: String,
    pub setting_desc: Option<String>,
    pub areas: Vec<Area>,
    pub monsters: Vec<MonsterDefinition>,
    pub items: Vec<ItemDefinition>,
    pub boss: Option<BossDefinition>,
    pub characters: Vec<Character>,
}

/// Contains raw AI-generated content fetched from the server.
pub struct IdeaGuy {
    pub game_defs: GameDefs,
    // keys/vals are indices into items.
    pub recipes: HashMap<(usize, usize), usize>,
    outgoing: Vec<PendingRequest>,
    pub next_craft_id: CraftId,
    pub error: Option<String>,
    pub error_count: usize,
    pub api: ApiClient,
}

impl IdeaGuy {
    pub fn new(theme: &str) -> Self {
        let mut slf = Self {
            game_defs: GameDefs {
                theme: theme.into(),
                setting_desc: None,
                areas: vec![],
                monsters: vec![],
                items: vec![],
                boss: None,
                characters: vec![],
            },
            outgoing: vec![],
            recipes: HashMap::new(),
            next_craft_id: CraftId(0),
            error: None,
            error_count: 0,
            api: ApiClient::new(),
        };
        slf.request(Request::Anything(AnythingRequest {
            ask: "Generate everything".into(),
            state: slf.game_defs.clone(),
        }));
        slf
    }

    pub fn from_saved(game_defs: GameDefs) -> Self {
        let next_craft_id = game_defs
            .items
            .iter()
            .flat_map(|item| item.craft_id)
            .max_by_key(|id| id.0)
            .map(|CraftId(n)| CraftId(n + 1))
            .unwrap_or(CraftId(0));
        Self {
            game_defs,
            outgoing: vec![],
            recipes: HashMap::new(),
            next_craft_id,
            error: None,
            error_count: 0,
            api: ApiClient::new(),
        }
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
        macroquad::miniquad::info!("Requesting {:?}", req);
        let api_url = api_url();
        let fut = match req {
            Request::Anything(ref req) => self
                .api
                .request(format!("{api_url}/v1/anything"), req.clone()),
            Request::Craft { item1, item2, .. } => self.api.request(
                format!("{api_url}/v1/craft"),
                CraftArgs {
                    theme: self.game_defs.theme.clone(),
                    setting: self.game_defs.setting_desc.clone().unwrap(),
                    items: self.game_defs.items.clone(),
                    item1: self.game_defs.items[item1].clone(),
                    item2: self.game_defs.items[item2].clone(),
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
            let result = req.get();
            if !matches!(result, RequestResult::Error(_) | RequestResult::Pending) {
                self.error = None;
                self.error_count = 0;
            }
            match result {
                RequestResult::Error(e) => {
                    macroquad::miniquad::error!("{}", e);
                    self.error = Some(e);
                    self.error_count += 1;
                    // Retry
                    self.request(req.req);
                }
                RequestResult::Pending => {
                    self.outgoing.push(req);
                }
                RequestResult::Anything(new_defs) => {
                    self.game_defs = new_defs;
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
                            .insert((item1, item2), self.game_defs.items.len());
                    }
                    self.game_defs.items.push(item);
                }
            }
        }
    }
    pub fn get_state(&self) -> IgState {
        if let Some(err) = self.error.as_ref() {
            IgState::Error {
                msg: err.clone(),
                count: self.error_count,
            }
        } else if self.game_defs.setting_desc.is_none() {
            IgState::Generating("everything")
        } else if !self.outgoing.is_empty() {
            IgState::Generating("crafted item")
        } else {
            IgState::Idle
        }
    }

    pub fn initial_generation_done(&self) -> bool {
        self.game_defs.boss.is_some()
    }
}
