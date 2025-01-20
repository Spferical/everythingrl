from enum import Enum
import json
import structlog
import os
from functools import cache
from typing import Annotated

import pydantic
import requests
from requests.adapters import Retry, HTTPAdapter

import vertexai
from vertexai.preview.generative_models import GenerativeModel, Part
import vertexai.preview.generative_models as generative_models

MISTRAL_API_URL = "https://api.mistral.ai"
MISTRAL_API_KEY = os.getenv("MISTRAL_API_KEY")
AISTUDIO_API_KEY = os.getenv("AISTUDIO_API_KEY")

DIR_PATH = os.path.dirname(os.path.realpath(__file__))

USE_VERTEX_AI = True

LOG = structlog.get_logger()


@cache
def get_test_str(fname):
    with open(os.path.join(DIR_PATH, "data", fname)) as f:
        return f.read()


@cache
def get_test_json(fname):
    with open(os.path.join(DIR_PATH, "data", fname)) as f:
        return json.load(f)


retry_strategy = Retry(
    total=4,
    status_forcelist=[429, 500],
    backoff_factor=2,
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


def fix_color(v, handler, info):
    try:
        Color(v)
    except ValueError:
        v = "lightgray"
    return handler(v)


Color = Annotated[Color, pydantic.WrapValidator(fix_color)]


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


def fix_type(v, handler, info):
    try:
        PokemonType(v)
    except ValueError:
        v = "normal"
    return handler(v)


PokemonType = Annotated[PokemonType, pydantic.WrapValidator(fix_type)]


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


def init_vertex_ai():
    vertexai.init(project=os.getenv("GCLOUD_PROJECT"), location="us-east4")


def ask_google_vertex_ai(prompt_parts: list[str]) -> str:
    model = GenerativeModel("gemini-1.0-pro-001")
    responses = model.generate_content(
        "".join(prompt_parts),
        generation_config={
            "max_output_tokens": 2048,
            "temperature": 0.9,
            "top_p": 1,
            "stop_sequences": ["--"],
        },
        safety_settings={
            generative_models.HarmCategory.HARM_CATEGORY_HATE_SPEECH: generative_models.HarmBlockThreshold.BLOCK_LOW_AND_ABOVE,
            generative_models.HarmCategory.HARM_CATEGORY_DANGEROUS_CONTENT: generative_models.HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
            generative_models.HarmCategory.HARM_CATEGORY_SEXUALLY_EXPLICIT: generative_models.HarmBlockThreshold.BLOCK_LOW_AND_ABOVE,
            generative_models.HarmCategory.HARM_CATEGORY_HARASSMENT: generative_models.HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
        },
        stream=False,
    )
    try:
        text = responses.candidates[0].content.parts[0].text
        text = text.strip("--")
        LOG.info(text)
        return text
    except KeyError:
        LOG.error(responses)
        raise
    except IndexError:
        LOG.error(responses)
        raise


def ask_google_ai_studio(prompt_parts: list[str]) -> str:
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
        LOG.info(text)
        return text
    except (IndexError, KeyError):
        LOG.error(response.json())
        raise


def ask_google(prompt_parts: list[str]):
    if USE_VERTEX_AI:
        return ask_google_vertex_ai(prompt_parts)
    else:
        return ask_google_ai_studio(prompt_parts)


def ask_google_structured(
    instructions: str,
    examples: list[tuple[dict, list[dict]]],
    input: dict,
    num_outputs: int,
    model: pydantic.BaseModel,
) -> dict:
    # Build prompt
    prompt_parts = [instructions, "--"]
    prompt_parts.append("Expected JSON schema of each output line: ")
    prompt_parts.append(json.dumps(model.model_json_schema()))
    prompt_parts.append("--")
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
    LOG.debug(f"ASKING: {prompt}")
    response_text = ask_google([prompt])
    LOG.info(f"RECEIVED: {response_text}")
    responses = response_text.split("\n")
    output = []
    for response in responses:
        try:
            response_json = json.loads(response)
            output.append(model(**response_json).model_dump())
            prompt_parts.append(response_text)
        except Exception as e:
            LOG.error(f"Bad response: {response}: {e}")
    return output


def gen_monsters(theme: str, setting_desc: str, names: list[str]):
    instructions = "You are the game master for a difficult permadeath roguelike. For each input theme and level, output JSON monster definitions. Valid types and attack types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. Valid colors are: lightgray yellow gold orange pink red maroon green lime skyblue blue purple violet beige brown white magenta. Output fields include name, the name of the monster; level, a number between 1 and 3 indicating how powerful the monster is; char, the single character to represent it as; color, one of the valid colors above; type1, the pokemon type of the monster; type2, an optional second type; attack_type, the pokemon the creature attacks as; and description, a two sentence description of the monster, one sentence of narration or dialogue which occurs when the enemy sees the player, one sentence of narration which occurs when the enemy attacks the player, one sentence of dialogue or narration which occurs when the enemy dies, and whether or not the enemy performs ranged attacks, and a number from 1 to 3 indicating how fast the enemy is. Output each monster JSON on its own line."
    examples = [
        (
            {
                "theme": "Hollow Knight",
                "setting_desc": get_test_str("hk.txt"),
                "enemy_names": list(
                    set(m["name"] for m in get_test_json("hk_monsters.json"))
                ),
            },
            get_test_json("hk_monsters.json"),
        )
    ]
    input = {"theme": theme, "setting_desc": setting_desc, "enemy_names": names}
    count = len(names)
    return ask_google_structured(instructions, examples, input, count, Monster)


def gen_items(theme: str, setting_desc: str, names: list[str]):
    instructions = "You are the game master for a difficult permadeath roguelike. Output JSON item definitions for each given item name. Valid types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. Output fields include name, the name of the item; level, a number between 1 and 3 indicating how powerful the item is; type, the pokemon type of the equipment or weapon; kind, indicating the kind of item, one of: melee_weapon ranged_weapon armor food; and description, a two sentence description of the item. Output each item JSON on its own line. DO NOT mention abilities or gameplay mechanics in the description; instead, focus on appearance or lore."
    examples = [
        (
            {
                "theme": "Hollow Knight",
                "setting_desc": get_test_str("hk.txt"),
                "item_names": list(
                    set(x["name"] for x in get_test_json("hk_items.json"))
                ),
            },
            get_test_json("hk_items.json"),
        )
    ]
    item_names = list(set(name for name in names))
    input = {"theme": theme, "item_names": item_names}
    count = len(item_names)
    return ask_google_structured(instructions, examples, input, count, Item)


def gen_setting_desc(theme: str):
    instructions = f"Write a two paragraph setting description for a roguelike game based off of the following theme: {theme}. The game has three levels and features melee attacks, ranged attacks, and crafting. The description should describe the setting and discuss the kinds of monsters, items, the setting of each level, and the final boss."
    return ask_google([instructions])


def gen_areas(theme: str, setting_desc: str):
    instructions = f"You are the game master for a difficult permadeath roguelike. Based on the provided theme and high-level setting descriptions, produce JSON data describing the contents of each of the levels: name, blurb (a moody message presented to the user as they enter the level), mapgen (a string representing what map generation algorithm should be used for this level, one of: 'simple_rooms_and_corridors', 'caves', 'hive', or 'dense_rooms'), names of 20 possible enemies, names of 5 pieces of equipment (i.e. armor or accessories), names of 3 melee weapons, names of 2 ranged weapons, and names of 3 food items that may be found on that level. DO NOT generate the final boss; the final boss will be on a special fourth level. DO NOT generate the final boss level."
    examples = [
        (
            {
                "theme": "NetHack",
                "setting_desc": get_test_str("nethack.txt"),
            },
            get_test_json("nethack_areas.json"),
        )
    ]
    return ask_google_structured(
        instructions,
        examples,
        {"theme": theme, "setting_desc": setting_desc},
        3,
        Area,
    )


def gen_boss(theme: str, setting_desc: str):
    instructions = f"You are the game master for a difficult permadeath roguelike. Based on the provided theme and high-level setting descriptions, produce JSON data describing the final boss of the game. The final boss is a slow enemy with a ranged attack that may appear with other monsters. Valid types and attack types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. Valid colors are: lightgray yellow gold orange pink red maroon green lime skyblue blue purple violet beige brown white magenta. Output fields include name, the name of the boss; char, the single character to represent it as in-game; color, one of the valid colors above; type1, the pokemon type of the boss; type2, an optional second type; attack_type, the pokemon the creature attacks as; description, a two sentence description of the boss shown if clicked; intro_message, a message presented to the player when encountering the boss; attack_messages, a list of messages of which one will be randomly presented when the boss attacks the player with its ranged attack; periodic_messages, messages presented to the player randomly throughout the fight; and game_over_paragraph, a long-form message presented to the player when the boss is defeated and the game is won."
    examples = [
        (
            {
                "theme": "Hollow Knight",
                "setting_desc": get_test_str("hk.txt"),
            },
            [get_test_json("hk_boss.json")],
        )
    ]
    return ask_google_structured(
        instructions,
        examples,
        {
            "theme": theme,
            "setting_desc": setting_desc,
        },
        1,
        Boss,
    )[0]


def craft(theme: str, setting_desc: str, items: list[str], item1: dict, item2: dict):
    instructions = f"You are the game master for a difficult permadeath roguelike with a crafting system. The player may combine any two items in the game to create a third item, similar to Homestuck captchalogue code alchemy. As input, you will be given a theme, a long-form description of the setting, descriptions of each item, and a list of items already in the game (do not copy any of these). Output a JSON item definition for each weapon and equipment in the given game description. Valid types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. DO NOT output multiple types. Output fields include name, the name of the item; level, a number indicating how powerful the weapon or equipment is; type, the pokemon type of the equipment or weapon; kind, the kind of item it is, one of: melee_weapon ranged_weapon armor food; and description, a two sentence description of the item. Output each item JSON on its own line. DO NOT reference gameplay mechanics that aren't in the game; instead, focus on appearance and lore. The two input items must be the same level; assign a level to the output item that is the level of each input item plus one; e.g. 2xL1->L2, 2xL2->L3, etc."
    return ask_google_structured(
        instructions,
        [],
        {
            "theme": theme,
            "setting_desc": setting_desc,
            "existing_items": items,
            "item1": item1,
            "item2": item2,
        },
        1,
        Item,
    )[0]


if USE_VERTEX_AI:
    init_vertex_ai()
