from enum import Enum
import json
import logging
import os
import time
from functools import cache
from typing import Annotated, cast, Type

import pydantic
import requests
from requests.adapters import Retry, HTTPAdapter

import google.api_core.exceptions
import vertexai
from vertexai.preview.generative_models import GenerativeModel, Part, GenerationResponse
import vertexai.preview.generative_models as generative_models

MISTRAL_API_URL = "https://api.mistral.ai"
MISTRAL_API_KEY = os.getenv("MISTRAL_API_KEY")
AISTUDIO_API_KEY = os.getenv("AISTUDIO_API_KEY")

DIR_PATH = os.path.dirname(os.path.realpath(__file__))

USE_VERTEX_AI = True


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




class AiError(Exception):
    pass


def get_safety_error(safety_ratings) -> AiError:
    safety_issues = []
    for rating in safety_ratings:
        if rating.probability > 1:  # can't find this protobuf enum anywhere
            match rating.category:
                case generative_models.HarmCategory.HARM_CATEGORY_HATE_SPEECH:
                    safety_issues.append("hateful")
                case generative_models.HarmCategory.HARM_CATEGORY_DANGEROUS_CONTENT:
                    safety_issues.append("dangerous")
                case generative_models.HarmCategory.HARM_CATEGORY_HARASSMENT:
                    safety_issues.append("harrassment")
                case generative_models.HarmCategory.HARM_CATEGORY_SEXUALLY_EXPLICIT:
                    safety_issues.append("too horny")
    raise AiError(", ".join(safety_issues))


def ask_google_vertex_ai(prompt_parts: list[str]) -> str:
    model = GenerativeModel("gemini-1.5-flash-001")
    try:
        responses = cast(
            GenerationResponse,
            model.generate_content(
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
            ),
        )
    except google.api_core.exceptions.ResourceExhausted as e:
        logging.error(e)
        time.sleep(1)
        return ask_google_vertex_ai(prompt_parts)
    try:
        candidate = responses.candidates[0]
        if candidate.finish_reason == generative_models.FinishReason.SAFETY:
            logging.error(candidate)
            raise get_safety_error(candidate.safety_ratings)
        text = candidate.content.parts[0].text
        text = text.strip("--").strip("```json").strip("```").strip("\n")
        logging.info(text)
        return text
    except KeyError:
        logging.error(responses)
        raise
    except IndexError:
        logging.error(responses)
        raise


def ask_google(prompt_parts: list[str]):
    return ask_google_vertex_ai(prompt_parts)


def ask_google_structured(
    instructions: str,
    examples: list[tuple[dict, list[dict]]],
    input: dict,
    num_outputs: int,
    model: Type[pydantic.BaseModel],
) -> list[dict]:
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
    logging.debug(f"ASKING: {prompt}")
    response_text = ask_google([prompt])
    logging.info(f"RECEIVED: {response_text}")
    responses = response_text.split("\n")
    output = []
    for response in responses:
        try:
            response_json = json.loads(response)
            output.append(model(**response_json).model_dump())
            prompt_parts.append(response_text)
        except Exception as e:
            logging.error(f"Bad response: {response}: {e}")
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
    instructions = f"You are the game master for a difficult permadeath roguelike. Based on the provided theme and high-level setting descriptions, produce JSON data describing the contents of each of the levels: name, blurb (a moody message presented to the user as they enter the level), mapgen (a string representing what map generation algorithm should be used for this level, one of: 'simple_rooms_and_corridors', 'caves', 'hive', or 'dense_rooms'), names of 20 possible enemies, names of 5 pieces of equipment (i.e. armor or accessories), names of 3 melee weapons, names of 2 ranged weapons, and names of 3 food items that may be found on that level. Make sure that all generated weapons, armor, monsters, and food are appropriate for the provided theme, try to avoid common or generic roguelike items. DO NOT generate the final boss; the final boss will be on a special fourth level. DO NOT generate the final boss level."
    examples = [
        (
            {
                "theme": "NetHack",
                "setting_desc": get_test_str("nethack.txt"),
            },
            get_test_json("nethack_areas.json"),
        ),
        (
            {
                "theme": "Hollow Knight",
                "setting_desc": get_test_str("hk.txt"),
            },
            get_test_json("hk_areas.json"),
        ),
        (
            {
                "theme": "Alien Isolation",
                "setting_desc": get_test_str("alien.txt"),
            },
            get_test_json("alien_areas.json"),
        ),
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


vertexai.init(project=os.getenv("GCLOUD_PROJECT"), location="us-east4")
