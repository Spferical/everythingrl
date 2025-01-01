from enum import Enum
from typing import Annotated

import pydantic


class Color(str, Enum):
    lightgray = "lightgray"
    yellow = "yellow"
    gold = "gold"
    orange = "orange"
    pink = "pink"
    red = "red"
    maroon = "maroon"
    green = "green"
    lime = "lime"
    skyblue = "skyblue"
    blue = "blue"
    purple = "purple"
    violet = "violet"
    beige = "beige"
    brown = "brown"
    white = "white"
    magenta = "magenta"
    silver = "silver"
    gray = "gray"
    grey = "grey"
    black = "black"


def fix_color(v, handler, _info):
    try:
        Color(v)
    except ValueError:
        v = "lightgray"
    return handler(v)


Color = Annotated[Color, pydantic.WrapValidator(fix_color)]  # type: ignore


class PokemonType(str, Enum):
    normal = "normal"
    fire = "fire"
    water = "water"
    electric = "electric"
    grass = "grass"
    ice = "ice"
    fighting = "fighting"
    poison = "poison"
    ground = "ground"
    flying = "flying"
    psychic = "psychic"
    bug = "bug"
    rock = "rock"
    ghost = "ghost"
    dragon = "dragon"
    dark = "dark"
    steel = "steel"
    fairy = "fairy"


def fix_type(v, handler, _info):
    try:
        PokemonType(v)
    except ValueError:
        v = "normal"
    return handler(v)


PokemonType = Annotated[PokemonType, pydantic.WrapValidator(fix_type)]  # type: ignore


class MapGen(str, Enum):
    simple_rooms_and_corridors = "simple_rooms_and_corridors"
    caves = "caves"
    hive = "hive"
    dense_rooms = "dense_rooms"


class Monster(pydantic.BaseModel):
    name: str
    char: str
    level: int
    color: Color
    type1: PokemonType
    type2: PokemonType | None = None
    attack_type: PokemonType
    description: str
    seen: str
    attack: str
    death: str
    ranged: bool
    speed: int


class Boss(pydantic.BaseModel):
    name: str
    char: str
    color: Color
    type1: PokemonType
    type2: PokemonType | None = None
    attack_type: PokemonType
    description: str
    intro_message: str
    attack_messages: list[str]
    periodic_messages: list[str]
    game_victory_paragraph: str


class Area(pydantic.BaseModel):
    name: str
    blurb: str
    mapgen: MapGen
    enemies: list[str]
    equipment: list[str]
    melee_weapons: list[str]
    ranged_weapons: list[str]
    food: list[str]


class ItemKind(str, Enum):
    armor = "armor"
    melee_weapon = "melee_weapon"
    ranged_weapon = "ranged_weapon"
    food = "food"


class Item(pydantic.BaseModel):
    name: str
    level: int
    type: PokemonType
    description: str
    kind: ItemKind


class Character(pydantic.BaseModel):
    name: str
    backstory: str
    starting_items: list[str]


class SettingDesc(pydantic.BaseModel):
    setting_desc: str


class AiAction(pydantic.BaseModel):
    set_setting_desc: str | None = pydantic.Field(description='Several player-facing paragraphs to introduce the setting and tone of the game.', default=None)
    add_area: Area | None = None
    add_monster_def: Monster | None = None
    add_item_def: Item | None = None
    set_boss: Boss | None = None
    add_character: Character | None = None


class GameState(pydantic.BaseModel):
    theme: str | None = None
    setting_desc: str | None = None
    areas: list[Area] = []
    monsters: list[Monster] = []
    items: list[Item] = []
    boss: Boss | None = None
    characters: list[Character] = []

    def apply_action(self, action: AiAction):
        if action.set_setting_desc:
            self.setting_desc = action.set_setting_desc
        if action.add_area is not None:
            for (i, area) in enumerate(self.areas):
                if area.name == action.add_area.name:
                    self.areas[i] = action.add_area
                    break
            else:
                self.areas.append(action.add_area)
        if action.add_monster_def is not None:
            for (i, monster) in enumerate(self.monsters):
                if monster.name == action.add_monster_def.name:
                    self.monsters[i] = action.add_monster_def
            else:
                self.monsters.append(action.add_monster_def)
        if action.add_item_def is not None:
            for (i, item_def) in enumerate(self.items):
                if item_def.name == action.add_item_def.name:
                    self.items[i] = action.add_item_def
            else:
                self.items.append(action.add_item_def)
        if action.set_boss is not None:
            self.boss = action.set_boss
        if action.add_character is not None:
            self.characters.append(action.add_character)
