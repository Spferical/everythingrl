use std::{
    collections::{hash_map::Entry, HashMap},
    sync::mpsc::{self, Receiver},
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

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    Lightgray,
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

pub fn request(url: String) -> BootlegFuture<Result<String, String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        tx.send(
            reqwest::blocking::get(url)
                .map_err(|e| e.to_string())
                .and_then(|r| r.text().map_err(|e| e.to_string())),
        )
        .ok();
    });
    BootlegFuture { rx, state: None }
}

pub fn api_url() -> Option<String> {
    std::env::var("SERVER_URL").ok()
}

pub enum IdeaGuy {
    Starting(BootlegFuture<Result<String, String>>),
    GotMonsters(Vec<MonsterDefinition>),
}

impl IdeaGuy {
    pub fn new(theme: &str) -> Self {
        if let Some(url) = api_url() {
            let boot_fut = request(format!("{url}/monsters/{theme}/1"));
            Self::Starting(boot_fut)
        } else {
            Self::GotMonsters([
        MonsterDefinition{name: "grid bug".into(), char: "x".into(), color: Color::Purple, type1: PokemonType::Bug, type2: Some(PokemonType::Electric), attack_type: PokemonType::Electric, description: "These electronically based creatures are not native to this universe. They appear to come from a world whose laws of motion are radically different from ours.".into()},
        MonsterDefinition{name: "floating eye".into(), char: "e".into(), color: Color::Blue, type1: PokemonType::Psychic, type2: None, attack_type: PokemonType::Psychic, description: "Floating eyes, not surprisingly, are large, floating eyeballs which drift about the dungeon. Though not dangerous in and of themselves, their power to paralyse those who gaze at their large eye in combat is widely feared.".into()},
        MonsterDefinition{name: "yellow mold".into(), char: "m".into(), color: Color::Yellow, type1: PokemonType::Poison, type2: None, attack_type: PokemonType::Poison, description: "Mold, multicellular organism of the division Fungi, typified by plant bodies composed of a network of cottony filaments.".into()},

        ].into())
        }
    }

    pub fn get_monsters(&mut self) -> Option<Vec<MonsterDefinition>> {
        match self {
            Self::GotMonsters(monsters) => Some(monsters.clone()),
            Self::Starting(boot_fut) => match boot_fut.get() {
                None => None,
                Some(Ok(resp)) => match serde_json::from_str(&resp) {
                    Ok(monsters) => {
                        *self = Self::GotMonsters(monsters);
                        self.get_monsters()
                    }
                    Err(err) => {
                        println!("Error parsing monsters: {}", err);
                        todo!()
                    }
                },
                Some(Err(err_msg)) => {
                    println!("Error fetching monsters: {}", err_msg);
                    panic!()
                }
            },
        }
    }
}
