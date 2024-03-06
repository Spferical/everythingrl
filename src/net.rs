use std::{
    fmt::Display,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, serde::Deserialize)]
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
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ItemDefinition {
    pub name: String,
    pub level: usize,
    pub color: Color,
    #[serde(rename = "type")]
    pub ty: PokemonType,
    pub description: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Area {
    pub name: String,
    pub blurb: String,
    pub enemies: Vec<String>,
    pub equipment: Vec<String>,
    pub melee_weapons: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonstersArgs {
    theme: String,
    setting: String,
    areas: Vec<Area>,
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

pub fn request<Input, Output>(url: String, input: Input) -> BootlegFuture<Result<Output, String>>
where
    Output: serde::de::DeserializeOwned + Send + 'static,
    Input: serde::Serialize + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
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
                .and_then(|r| r.text().map_err(|e| e.to_string()))
                .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string())),
        )
        .ok();
    });
    BootlegFuture { rx, state: None }
}

pub fn api_url() -> String {
    // TODO: change default url
    std::env::var("SERVER_URL").unwrap_or("http://localhost:5000".into())
}

enum IdeaGuyState {
    GetSetting(BootlegFuture<Result<String, String>>),
    GetAreas(BootlegFuture<Result<Vec<Area>, String>>),
    GetMonsters(BootlegFuture<Result<Vec<MonsterDefinition>, String>>),
    GetItems(BootlegFuture<Result<Vec<ItemDefinition>, String>>),
    Done,
}

/// Contains raw AI-generated content fetched from the server.
pub struct IdeaGuy {
    pub theme: String,
    pub api_url: String,
    pub setting: Option<String>,
    pub areas: Option<Vec<Area>>,
    pub monsters: Option<Vec<MonsterDefinition>>,
    pub items: Option<Vec<ItemDefinition>>,
    state: IdeaGuyState,
}

impl IdeaGuy {
    pub fn new(theme: &str) -> Self {
        let api_url = api_url();
        let boot_fut = request(format!("{api_url}/setting/{theme}"), "");
        Self {
            theme: theme.into(),
            api_url,
            setting: None,
            areas: None,
            monsters: None,
            items: None,
            state: IdeaGuyState::GetSetting(boot_fut),
        }
    }

    pub fn tick(&mut self) {
        match self.state {
            IdeaGuyState::GetSetting(ref mut fut) => match fut.get() {
                Some(Ok(resp)) => {
                    self.setting = Some(resp.clone());
                    let api_url = &self.api_url;
                    self.state = IdeaGuyState::GetAreas(request(
                        format!("{api_url}/areas"),
                        AreasArgs {
                            theme: self.theme.clone(),
                            setting: self.setting.clone().unwrap(),
                        },
                    ))
                }
                Some(Err(e)) => panic!("{}", e),
                None => {}
            },
            IdeaGuyState::GetAreas(ref mut fut) => match fut.get() {
                Some(Ok(resp)) => {
                    self.areas = Some(resp.clone());
                    let api_url = &self.api_url;
                    self.state = IdeaGuyState::GetMonsters(request(
                        format!("{api_url}/monsters"),
                        MonstersArgs {
                            theme: self.theme.clone(),
                            setting: self.setting.clone().unwrap().clone(),
                            areas: self.areas.clone().unwrap().clone(),
                        },
                    ))
                }
                Some(Err(e)) => panic!("{}", e),
                None => {}
            },
            IdeaGuyState::GetMonsters(ref mut fut) => match fut.get() {
                Some(Ok(resp)) => {
                    self.monsters = Some(resp.clone());
                    let api_url = &self.api_url;
                    self.state = IdeaGuyState::GetItems(request(
                        format!("{api_url}/items"),
                        MonstersArgs {
                            theme: self.theme.clone(),
                            setting: self.setting.clone().unwrap().clone(),
                            areas: self.areas.clone().unwrap().clone(),
                        },
                    ))
                }
                Some(Err(e)) => panic!("{}", e),
                None => {}
            },
            IdeaGuyState::GetItems(ref mut fut) => match fut.get() {
                Some(Ok(resp)) => {
                    self.items = Some(resp.clone());
                    self.state = IdeaGuyState::Done;
                }
                Some(Err(e)) => panic!("{}", e),
                None => {}
            },
            IdeaGuyState::Done => {}
        }
    }
}
