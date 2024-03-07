from enum import Enum
import json
import logging
import os

import pydantic
import requests
from requests.adapters import Retry, HTTPAdapter

MISTRAL_API_URL = "https://api.mistral.ai"
MISTRAL_API_KEY = os.getenv("MISTRAL_API_KEY")
AISTUDIO_API_KEY = os.getenv("AISTUDIO_API_KEY")

DIR_PATH = os.path.dirname(os.path.realpath(__file__))

with open(os.path.join(DIR_PATH, "data", "hk.txt")) as f:
    HK_SETTING_DESC = f.read()
with open(os.path.join(DIR_PATH, "data", "hk_areas.json")) as f:
    HK_AREAS = json.load(f)
with open(os.path.join(DIR_PATH, "data", "hk_monsters.json")) as f:
    HK_MONSTERS = json.load(f)
with open(os.path.join(DIR_PATH, "data", "hk_items.json")) as f:
    HK_ITEMS = json.load(f)

retry_strategy = Retry(
    total=4,
    status_forcelist=[429, 500],
    allowed_methods=frozenset(
        {"DELETE", "GET", "HEAD", "OPTIONS", "PUT", "TRACE", "POST"}
    ),
)
adapter = HTTPAdapter(max_retries=retry_strategy)

session = requests.Session()
session.mount("http://", adapter)
session.mount("https://", adapter)


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


class Monster(pydantic.BaseModel):
    name: str
    char: str
    level: int
    color: Color
    type1: PokemonType
    type2: PokemonType | None = None
    attack_type: PokemonType
    description: str


class Area(pydantic.BaseModel):
    name: str
    blurb: str
    enemies: list[str]
    equipment: list[str]
    melee_weapons: list[str]


class ItemSlot(str, Enum):
    equipment = "equipment"
    weapon = "weapon"


class Item(pydantic.BaseModel):
    name: str
    level: int
    color: Color
    type: PokemonType
    description: str


def ask_mistral(prompt_parts: list[str]) -> str:
    messages = [{"role": "user", "content": "".join(prompt_parts)}]
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": f"Bearer {MISTRAL_API_KEY}",
    }
    payload = {"model": "open-mixtral-8x7b", "messages": messages, "max_tokens": 2048}
    response = session.post(
        f"{MISTRAL_API_URL}/v1/chat/completions",
        headers=headers,
        json=payload,
    )
    response.raise_for_status()
    return response.json()["choices"][0]["message"]["content"]


def ask_google(prompt_parts: list[str]) -> str:
    url = f"https://generativelanguage.googleapis.com/v1/models/gemini-1.0-pro:generateContent?key={AISTUDIO_API_KEY}"
    headers = {"Content-Type": "application/json"}
    payload = {
        "contents": [
            {
                "parts": [{"text": part} for part in prompt_parts],
            }
        ],
        "generationConfig": {
            "temperature": 0.9,
            "topK": 1,
            "topP": 1,
            "maxOutputTokens": 8192,
            "stopSequences": ["--"],
        },
    }
    response = session.post(url, headers=headers, json=payload)
    response.raise_for_status()
    try:
        text = response.json()["candidates"][0]["content"]["parts"][0]["text"]
        text = text.strip("--")
        logging.info(text)
        return text
    except KeyError:
        logging.error(response.json())
        raise


def ask_google_structured(
    instructions: str,
    examples: list[tuple[dict, list[dict]]],
    input: dict,
    num_outputs: int,
    model: pydantic.BaseModel,
) -> dict:
    # Build prompt
    prompt_parts = [instructions, "--"]
    for ex_input, ex_outputs in examples:
        ex_input = dict(ex_input)
        ex_input["num_outputs"] = len(ex_outputs)
        prompt_parts.append(json.dumps(ex_input))
        prompt_parts.append("\n")
        for ex_output in ex_outputs:
            prompt_parts.append(json.dumps(ex_output))
            prompt_parts.append("\n")
        prompt_parts.append("--")
    input = dict(input)
    input["num_outputs"] = num_outputs
    prompt_parts.append(json.dumps(input))
    prompt_parts.append("\n")

    prompt = "".join(prompt_parts)
    response_text = ask_google([prompt])
    logging.info(response_text)
    responses = response_text.split("\n")
    output = []
    for response in responses:
        try:
            response_json = json.loads(response)
            model(**response_json)
            output.append(response_json)
            prompt_parts.append(response_text)
        except Exception as e:
            logging.error(f"Bad response: {response}: {e}")
    return output


def gen_monsters(theme: str, setting_desc: str, areas: list[dict]):
    instructions = "You are the game master for a difficult permadeath roguelike. For each input theme and level, output JSON monster definitions. Valid types and attack types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. Valid colors are: lightgray yellow gold orange pink red maroon green lime skyblue blue purple violet beige brown white magenta. Output fields include name, the name of the monster; level, a number between 1 and 3 indicating how powerful the monster is; char, the single character to represent it as; color, one of the valid colors above; type1, the pokemon type of the monster; type2, an optional second type; attack_type, the pokemon the creature attacks as; and description, a two sentence description of the monster. Output each monster JSON on its own line."
    examples = [
        (
            {
                "theme": "Hollow Knight",
                "setting_desc": HK_SETTING_DESC,
                "enemy_names": list(
                    set(name for area in HK_AREAS for name in area["enemies"])
                ),
            },
            HK_MONSTERS,
        )
    ]
    enemy_names = list(set(name for area in areas for name in area["enemies"]))
    input = {"theme": theme, "enemy_names": enemy_names}
    count = len(enemy_names)
    return ask_google_structured(instructions, examples, input, count, Monster)


def gen_items(theme: str, setting_desc: str, areas: list[dict]):
    instructions = "You are the game master for a difficult permadeath roguelike. Output JSON item definitions for each weapon and equipment in the given game description. Valid types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. Valid colors are: lightgray yellow gold orange pink red maroon green lime skyblue blue purple violet beige brown white magenta. Output fields include name, the name of the item; level, a number between 1 and 3 indicating how powerful the weapon or equipment is; color, one of the valid colors above; type, the pokemon type of the equipment or weapon; and description, a two sentence description of the item. Output each item JSON on its own line. DO NOT reference gameplay mechanics that aren't in the game; instead, focus on appearance and lore."
    examples = [
        (
            {
                "theme": "Hollow Knight",
                "setting_desc": HK_SETTING_DESC,
                "item_names": list(
                    set(
                        name
                        for area in HK_AREAS
                        for name in area["equipment"] + area["melee_weapons"]
                    )
                ),
            },
            HK_ITEMS,
        )
    ]
    item_names = list(
        set(
            name for area in areas for name in area["equipment"] + area["melee_weapons"]
        )
    )
    input = {"theme": theme, "item_names": item_names}
    count = len(item_names)
    return ask_google_structured(instructions, [], input, count, Item)


def gen_setting_desc(theme: str):
    instructions = f"Write a two paragraph setting description for a roguelike game based off of the following theme: {theme}. The game has three levels and features melee attacks and crafting. should describe the setting and discuss the kinds of monsters, items, the setting of each level, and the final boss."
    return ask_google([instructions])


def gen_areas(theme: str, setting_desc: str):
    instructions = f"You are the game master for a difficult permadeath roguelike. Based on the provided theme and high-level setting descriptions, produce JSON data describing the contents of each of the levels: name, blurb (a moody message presented to the user as they enter the level), names of 10 possible enemies, names of 5 pieces of equipment (i.e. armor or accessories), and names of 5 melee weapons that may be found on that level."
    examples = [
        (
            {
                "theme": "Hollow Knight",
                "setting_desc": HK_SETTING_DESC,
            },
            HK_AREAS,
        )
    ]
    return ask_google_structured(
        instructions,
        examples,
        {"theme": theme, "setting_desc": setting_desc},
        3,
        Area,
    )
