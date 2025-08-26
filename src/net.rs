use async_std::stream::StreamExt;
use enum_map::Enum;
use once_cell::sync::Lazy;
use std::{
    collections::{HashSet, VecDeque},
    fmt::Display,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use crate::ratelimit::Ratelimit;
use crate::util::spawn;

pub static RATELIMIT: Ratelimit = Ratelimit::new(Duration::from_secs(1));

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
            PokemonType::Rock => Color::Orange,
            PokemonType::Ghost => Color::Purple,
            PokemonType::Dragon => Color::Orange,
            PokemonType::Dark => Color::Black,
            PokemonType::Steel => Color::White,
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ItemDefinition {
    pub name: String,
    pub level: usize,
    #[serde(rename = "type")]
    pub ty: PokemonType,
    pub description: String,
    pub kind: ItemKind,
}

#[derive(Enum, PartialEq, Eq, Hash, Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MapGen {
    SimpleRoomsAndCorridors,
    Caves,
    Hive,
    DenseRooms,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Character {
    pub name: String,
    pub backstory: String,
    pub starting_items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Recipe {
    pub item1: String,
    pub item2: String,
    pub output: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ActionsArgs {
    state: GameDefs,
    ask: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiAction {
    SetSettingDesc(String),
    AddArea(Area),
    AddMonsterDef(MonsterDefinition),
    AddItemDef(ItemDefinition),
    SetBoss(BossDefinition),
    AddCharacter(Character),
    SetRecipe(Recipe),
}

impl AiAction {
    fn get_loading_message(&self) -> String {
        match self {
            AiAction::SetSettingDesc(_) => "Describing the setting...".into(),
            AiAction::AddArea(area) => format!("Mapping the {}", area.name),
            AiAction::AddMonsterDef(monster_definition) => {
                format!("Raising a {}", monster_definition.name)
            }
            AiAction::AddItemDef(item_definition) => format!("Placing a {}", item_definition.name),
            AiAction::SetBoss(boss_definition) => format!("Creating the {}", boss_definition.name),
            AiAction::AddCharacter(character) => format!("Designing the {}", character.name),
            AiAction::SetRecipe(recipe) => format!("Writing a recipe for {}", recipe.output),
        }
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

static API_URL: Lazy<String> = Lazy::new(|| {
    #[cfg(target_family = "wasm")]
    {
        use web_sys::window;
        if window()
            .unwrap()
            .location()
            .href()
            .unwrap()
            .contains("localhost:5000")
        {
            return "http://localhost:5000".into();
        }
    }

    std::env::var("SERVER_URL").unwrap_or("https://7drl24.pfe.io".into())
});

static ACTIONS_URL: Lazy<String> = Lazy::new(|| {
    let api_url = &*API_URL;
    format!("{api_url}/v1/actions")
});

static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

#[derive(serde::Deserialize)]
struct ServerError {
    error: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GameDefs {
    pub theme: String,
    pub setting_desc: Option<String>,
    pub areas: Vec<Area>,
    pub monsters: Vec<MonsterDefinition>,
    pub items: Vec<ItemDefinition>,
    pub boss: Option<BossDefinition>,
    pub characters: Vec<Character>,
    pub recipes: Vec<Recipe>,
}

impl GameDefs {
    pub fn new(theme: String) -> Self {
        Self {
            theme,
            setting_desc: None,
            areas: vec![],
            monsters: vec![],
            items: vec![],
            boss: None,
            characters: vec![],
            recipes: vec![],
        }
    }

    fn apply_action(&mut self, action: AiAction) {
        match action {
            AiAction::SetSettingDesc(s) => self.setting_desc = Some(s),
            AiAction::AddArea(new_area) => {
                if let Some(area) = self.areas.iter_mut().find(|a| a.name == new_area.name) {
                    *area = new_area;
                } else {
                    self.areas.push(new_area)
                }
            }
            AiAction::AddMonsterDef(new_mon) => {
                if let Some(m) = self.monsters.iter_mut().find(|m| m.name == new_mon.name) {
                    *m = new_mon;
                } else {
                    self.monsters.push(new_mon)
                }
            }
            AiAction::AddItemDef(new_item) => {
                if let Some(item) = self.items.iter_mut().find(|i| i.name == new_item.name) {
                    *item = new_item;
                } else {
                    self.items.push(new_item);
                }
            }
            AiAction::SetBoss(boss_definition) => {
                self.boss = Some(boss_definition);
            }
            AiAction::AddCharacter(new_char) => {
                if let Some(c) = self.characters.iter_mut().find(|c| c.name == new_char.name) {
                    *c = new_char;
                } else {
                    self.characters.push(new_char);
                }
            }
            AiAction::SetRecipe(recipe) => {
                if let Some(r) = self
                    .recipes
                    .iter_mut()
                    .find(|r| (&r.item1, &r.item2) == (&recipe.item1, &recipe.item2))
                {
                    *r = recipe;
                } else {
                    self.recipes.push(recipe);
                }
            }
        }
    }
}

enum WorkItem {
    Craft(usize, usize),
}

#[derive(Debug, Clone)]
pub enum GenerationAbortReason {
    ServerError,
    Loop,
}

#[derive(Debug, Clone)]
pub enum InitialGenerationStatus {
    Generating {
        msg: String,
        error_count: u64,
    },
    ErroredOut {
        msg: String,
        reason: GenerationAbortReason,
    },
    Done,
}

struct IdeaGuyState {
    game_defs: GameDefs,
    init_gen_status: InitialGenerationStatus,
    work: VecDeque<WorkItem>,
}

fn stream_actions(ask: String, game_defs: GameDefs) -> mpsc::Receiver<Result<AiAction, String>> {
    let (tx, rx) = mpsc::channel();
    spawn(async move {
        let actions_args = ActionsArgs {
            state: game_defs.clone(),
            ask,
        };
        RATELIMIT.wait().await;
        match CLIENT.post(&*ACTIONS_URL).json(&actions_args).send().await {
            Ok(r) if r.status().as_u16() >= 400 => {
                // try decoding full server error
                let err_payload = r.text().await.unwrap_or_default();
                tx.send(Err(serde_json::from_str::<ServerError>(&err_payload)
                    .map(|e| e.error)
                    .unwrap_or(crate::util::trim(&err_payload, 100).into())))
                    .ok();
            }
            Ok(resp) => {
                let mut response_bytes = Vec::new();
                let mut stream = resp.bytes_stream();
                loop {
                    match stream.next().await {
                        Some(Ok(chunk)) => {
                            response_bytes.extend_from_slice(&chunk);
                            while let Some(i) = response_bytes.iter().position(|x| *x == b'\n') {
                                if let Ok(ServerError { error }) =
                                    serde_json::from_slice(&response_bytes[..i])
                                {
                                    tx.send(Err(error)).ok();
                                } else {
                                    match serde_json::from_slice(&response_bytes[..i]) {
                                        Ok(action) => {
                                            tx.send(Ok(action)).ok();
                                        }
                                        Err(err) => {
                                            macroquad::miniquad::error!(
                                                "Error deserializing AI action: {}: {}",
                                                err,
                                                String::from_utf8_lossy(&response_bytes[..i])
                                            );
                                        }
                                    }
                                }
                                response_bytes.drain(..=i);
                            }
                        }
                        None => break,
                        Some(Err(err)) => {
                            macroquad::miniquad::error!("Error streaming request: {err}");
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                let err = format_full_error_chain(err);
                macroquad::miniquad::error!("Request error: {err}");
                tx.send(Err(err)).ok();
            }
        }
    });
    rx
}

fn get_missing_requirements(defs: &GameDefs) -> String {
    let mut reqs = String::new();
    if defs.setting_desc.is_none() {
        reqs += "- a setting_desc string describing the style, mood, substance, and high-level ideas to inform the artistic direction and content for the game.\n";
    }
    if defs.areas.len() < 3 {
        reqs +=
            "- three areas i.e. levels for the player to explore on his way to the final boss.\n";
    }
    let mentioned_monsters = defs
        .areas
        .iter()
        .flat_map(|area| area.enemies.iter())
        .cloned()
        .collect::<HashSet<String>>();
    let defined_monsters = defs
        .monsters
        .iter()
        .map(|mon| mon.name.clone())
        .collect::<HashSet<String>>();
    for monster_name in mentioned_monsters.difference(&defined_monsters) {
        reqs += &format!("- a monster definition for \"{monster_name}\".\n");
    }
    let mentioned_items = defs
        .areas
        .iter()
        .flat_map(|area| {
            area.equipment
                .iter()
                .chain(area.melee_weapons.iter())
                .chain(area.ranged_weapons.iter())
                .chain(area.food.iter())
        })
        .chain(defs.characters.iter().flat_map(|c| c.starting_items.iter()))
        .cloned()
        .collect::<HashSet<String>>();
    let defined_items = defs
        .items
        .iter()
        .map(|i| i.name.clone())
        .collect::<HashSet<String>>();
    for item_name in mentioned_items.difference(&defined_items) {
        reqs += &format!("- an item definition for \"{item_name}\".\n");
    }
    if defs.boss.is_none() {
        reqs += "- a final boss.\n"
    }
    if defs.characters.len() < 3 {
        reqs += "- at least 3 characters or character classes available to the player.\n";
    }
    reqs
}

async fn gen_and_apply_actions(state: Arc<Mutex<IdeaGuyState>>, instructions: String) {
    let tmp_defs = state.lock().unwrap().game_defs.clone();
    let actions = stream_actions(instructions, tmp_defs);
    loop {
        match actions.try_recv() {
            Ok(result) => {
                let mut state = state.lock().unwrap();
                match result {
                    Ok(action) => {
                        state.init_gen_status = InitialGenerationStatus::Generating {
                            msg: action.get_loading_message(),
                            error_count: 0,
                        };
                        state.game_defs.apply_action(action);
                    }
                    Err(err) => {
                        if let InitialGenerationStatus::Generating {
                            msg: _,
                            error_count,
                        } = state.init_gen_status
                        {
                            let error_count = error_count + 1;
                            state.init_gen_status = InitialGenerationStatus::Generating {
                                msg: format!("Error: {err}. Retrying (x{})...", error_count),
                                error_count,
                            }
                        }
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                crate::util::sleep(Duration::from_millis(100)).await;
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
        };
    }
}

async fn generate_work(state: Arc<Mutex<IdeaGuyState>>) {
    // Initial generation loop.
    let mut num_noop_generations = 0;
    for initial_loop_idx in 0.. {
        let start_defs = state.lock().unwrap().game_defs.clone();
        let reqs = get_missing_requirements(&start_defs);
        if reqs.is_empty() {
            break;
        }
        let instructions = format!("Generate everything missing. Namely, we need:\n{reqs}");
        gen_and_apply_actions(state.clone(), instructions).await;

        let state = &mut state.lock().unwrap();

        if let InitialGenerationStatus::Generating { msg, error_count } =
            state.init_gen_status.clone()
        {
            if error_count >= 3 {
                state.init_gen_status = InitialGenerationStatus::ErroredOut {
                    msg,
                    reason: GenerationAbortReason::ServerError,
                };
                return;
            } else {
                if error_count == 0 && state.game_defs == start_defs {
                    num_noop_generations += 1;
                } else {
                    num_noop_generations = 0;
                }
                if num_noop_generations >= 2 || initial_loop_idx >= 10 {
                    state.init_gen_status = InitialGenerationStatus::ErroredOut {
                        msg: "Detected that the AI got in a loop".into(),
                        reason: GenerationAbortReason::Loop,
                    };
                    return;
                }
            }
        }
    }
    state.lock().unwrap().init_gen_status = InitialGenerationStatus::Done;

    // Work loop.
    loop {
        crate::util::sleep(Duration::from_millis(100)).await;
        let work = state.lock().unwrap().work.pop_front();
        if let Some(work) = work {
            let tmp_defs = state.lock().unwrap().game_defs.clone();
            match work {
                WorkItem::Craft(item1, item2) => {
                    let item1_name = tmp_defs.items[item1].name.clone();
                    let item2_name = tmp_defs.items[item2].name.clone();
                    while !state
                        .lock()
                        .unwrap()
                        .game_defs
                        .recipes
                        .iter()
                        .any(|r| r.item1 == item1_name && r.item2 == item2_name)
                    {
                        gen_and_apply_actions(
                            state.clone(),
                            format!("Create a recipe for combining \"{}\" and \"{}\" and any item definition for the output, if required.",
                            item1_name,
                            item2_name)
                        )
                        .await;
                    }
                }
            }
        }
    }
}

fn start_background_generation_worker(game_defs: GameDefs) -> Arc<Mutex<IdeaGuyState>> {
    let state = Arc::new(Mutex::new(IdeaGuyState {
        game_defs,
        init_gen_status: InitialGenerationStatus::Generating {
            msg: "".into(),
            error_count: 0,
        },
        work: [].into(),
    }));
    spawn(generate_work(state.clone()));
    state
}

/// Contains raw AI-generated content fetched from the server.
pub struct IdeaGuy {
    generation_state: Arc<Mutex<IdeaGuyState>>,
    pub game_defs: GameDefs,
}

impl IdeaGuy {
    pub fn new(game_defs: GameDefs) -> Self {
        Self {
            generation_state: start_background_generation_worker(game_defs.clone()),
            game_defs,
        }
    }

    pub fn craft(&mut self, item1: usize, item2: usize) {
        self.generation_state
            .lock()
            .unwrap()
            .work
            .push_back(WorkItem::Craft(item1, item2));
    }

    pub fn tick(&mut self) {
        self.game_defs = self.generation_state.lock().unwrap().game_defs.clone();
    }
    pub fn get_state(&self) -> InitialGenerationStatus {
        self.generation_state
            .lock()
            .unwrap()
            .init_gen_status
            .clone()
    }
}
