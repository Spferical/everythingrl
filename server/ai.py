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


def ask_mistral(prompt_parts: list[str]) -> str:
    messages = [{"role": "system", "content": "".join(prompt_parts)}]
    model = "mistral-small"

    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": f"Bearer {AISTUDIO_API_KEY}",
    }

    payload = {"model": "mistral-tiny", "messages": messages, "max_tokens": 2048}

    response = session.post(
        f"{MISTRAL_API_URL}/v1/chat/completions",
        headers=headers,
        json=payload,
    )

    response.raise_for_status()

    return response.json()["choices"][0]["message"]["content"]


def ask_google(prompt_parts: list[str]) -> str:
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-1.0-pro:generateContent?key={AISTUDIO_API_KEY}"
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


def gen_setting_desc(theme: str):
    instructions = f"Write a two paragraph setting description for a roguelike game based off of the following theme: {theme}. The game has three levels and features melee attacks and crafting. should describe the setting and discuss the kinds of monsters, items, the setting of each level, and the final boss."
    return ask_google([instructions])


def gen_areas(theme: str, setting_desc: str):
    instructions = f"You are the game master for a difficult permadeath roguelike. Based on the provided theme and high-level setting descriptions, produce JSON data describing the contents of each of the levels: name, blurb (a moody message presented to the user as they enter the level), names of 10 possible enemies, names of 5 pieces of equipment (i.e. armor or accessories), and names of 5 melee weapons that may be found on that level."
    examples = [
        (
            {
                "theme": "Hollow Knight",
                "setting_desc": "In the depths of Hallownest, a desolate and forsaken kingdom, a roguelike adventure awaits. Journey through three treacherous levels, each teeming with grotesque creatures and dilapidated ruins. As you delve deeper, the horrors that lurk in the shadows grow more terrifying, from ghostly wisps to venomous spiders. Engage in visceral melee combat, mastering your swordsmanship and dodging enemy attacks with precision.\n\nAlong the way, gather materials to craft powerful items and abilities. From healing potions to explosive traps, these creations will aid your survival. The level design is intricate and interconnected, with multiple paths and hidden secrets to uncover. Navigate treacherous chasms, navigate crumbling caverns, and uncover the remnants of Hallownest's tragic past. As you approach the end, prepare to face the Shadow King, a formidable adversary who guards the final secrets of the kingdom. Only by overcoming this formidable foe can you escape the clutches of Hallownest and uncover its forgotten lore.",
            },
            [
                {
                    "name": "Forgotten Crossroads",
                    "blurb": "Emerging from the darkness, you step into the forgotten crossroads of Hallownest. Vines cling to dilapidated walls, casting long shadows across the crumbling stone. A chilling wind whispers secrets of a forgotten past.",
                    "enemies": [
                        "Husk Sentry",
                        "Crawler",
                        "Gloomwing",
                        "Vengefly",
                        "Leaper",
                        "Husk Warrior",
                        "Tiktik",
                        "Flukemunga",
                        "Baldur",
                        "Aspid Warrior",
                    ],
                    "equipment": [
                        "Ruined Cloak",
                        "Fungal Boots",
                        "Nail Sharpener",
                        "Lantern",
                        "Baldur Shell",
                    ],
                    "melee_weapons": [
                        "Ancient Nail",
                        "Nail of Shadows",
                        "Bone Needle",
                        "Coiled Sword",
                        "Thorned Whip",
                    ],
                },
                {
                    "name": "Crystal Peak",
                    "blurb": "As you ascend the winding path, the air grows heavy with the scent of minerals. Towering crystalline formations shimmer in the dim light, casting eerie reflections upon the jagged walls. A faint glow emanates from deep within the mine, beckoning you further.",
                    "enemies": [
                        "Crystal Crawler",
                        "Crystal Hunter",
                        "Vengefly",
                        "Great Husk Sentinel",
                        "Crystal Guardian",
                        "Primal Aspid",
                        "Mantis Petra",
                        "Mantis Warrior",
                        "Stalking Devout",
                        "Mage",
                    ],
                    "equipment": [
                        "Crystal Shell",
                        "Quartz Breastplate",
                        "Luminous Leggings",
                        "Reflective Cloak",
                        "Prism Ring",
                    ],
                    "weapons": [
                        "Prismatic Sword",
                        "Geode Mace",
                        "Laser Cutter",
                        "Shard Arrow",
                        "Pickaxe",
                    ],
                },
                {
                    "name": "The Abyss",
                    "blurb": "You stumble into a realm of eternal darkness, where gravity seems to play tricks upon your senses. Strange sounds echo through the chasm, stirring primal fears deep within your soul. A sense of ancient evil lingers in the air, as if the abyss itself is watching your every move.",
                    "enemies": [
                        "Voidwalker",
                        "Abyss Shrieker",
                        "Abyssal Shade",
                        "Darkness Devourer",
                        "Shadow Lurker",
                        "Nightmare Wisp",
                        "Void Stalker",
                        "Obsidian Assassin",
                        "Dusk Bringer",
                        "Silence Weaver",
                    ],
                    "equipment": [
                        "Abyssal Armor",
                        "Void Cloak",
                        "Phantom Boots",
                        "Void Heart",
                        "Weaversong",
                    ],
                    "weapons": [
                        "Shadow Blade",
                        "Obsidian Dagger",
                        "Eclipse Scythe",
                        "Pure Nail",
                        "Dream Nail",
                    ],
                },
            ],
        )
    ]
    return ask_google_structured(
        instructions,
        examples,
        {"theme": theme, "setting_desc": setting_desc},
        3,
        Area,
    )
