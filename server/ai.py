import json
import os

import jsonschema
import requests
from requests.adapters import Retry, HTTPAdapter

API_URL = "https://api.mistral.ai"
API_KEY = os.getenv("AISTUDIO_API_KEY")

retry_strategy = Retry(
    total=4,
    status_forcelist=[429, 500, 502, 503, 504],
    allowed_methods=frozenset(
        {"DELETE", "GET", "HEAD", "OPTIONS", "PUT", "TRACE", "POST"}
    ),
)
adapter = HTTPAdapter(max_retries=retry_strategy)

session = requests.Session()
session.mount("http://", adapter)
session.mount("https://", adapter)


def ask_mistral(messages):
    model = "mistral-small"

    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": f"Bearer {API_KEY}",
    }

    payload = {"model": "mistral-tiny", "messages": messages, "max_tokens": 300}

    response = session.post(
        f"{API_URL}/v1/chat/completions",
        headers=headers,
        json=payload,
    )

    response.raise_for_status()

    return response.json()["choices"][0]["message"]["content"]


def ask_google(prompt_parts: list[str]) -> str:
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-1.0-pro:generateContent?key={API_KEY}"
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
            "maxOutputTokens": 2048,
            "stopSequences": [],
        },
    }
    print(payload)
    response = session.post(url, headers=headers, json=payload)
    response.raise_for_status()
    return response.json()["candidates"][0]["content"]["parts"][0]["text"]


def ask_google_structured(
    instructions: str,
    examples: list[tuple[dict, list[dict]]],
    input: dict,
    num_outputs: int,
    schema: dict,
) -> dict:
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
    response_text = ask_google(["".join(prompt_parts)])
    responses = response_text.split("\n")
    output = []
    for response in responses:
        try:
            response_json = json.loads(response)
            jsonschema.validate(response_json, schema)
            output.append(response_json)
        except Exception as e:
            print(f"Bad response: {response}: {e}")
    return output


def gen_monster(theme: str, level: int, count: int = 3):
    instructions = "You are the game master for a difficult permadeath roguelike. For each input theme and level, output JSON monster definitions. Valid types and attack types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. Valid colors are: lightgray yellow gold orange pink red maroon green lime skyblue blue purple violet beige brown white magenta. Output fields include name, the name of the monster; char, the single character to represent it as; color, one of the valid colors above; type1, the pokemon type of the monster; type2, an optional second type; attack_type, the pokemon the creature attacks as; and description, a one-or-two sentence description of the monster."
    color_schema = {
        "enum": [
            "lightgray",
            "yellow",
            "gold",
            "orange",
            "pink",
            "red",
            "maroon",
            "green",
            "lime",
            "skyblue",
            "blue",
            "purple",
            "violet",
            "beige",
            "brown",
            "white",
            "magenta",
        ]
    }
    type_schema = {
        "enum": [
            "normal",
            "fire",
            "water",
            "electric",
            "grass",
            "ice",
            "fighting",
            "poison",
            "ground",
            "flying",
            "psychic",
            "bug",
            "rock",
            "ghost",
            "dragon",
            "dark",
            "steel",
            "fairy",
        ]
    }
    schema = {
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "char": {"type": "string"},
            "color": color_schema,
            "type1": type_schema,
            "type2": type_schema,
            "attack_type": type_schema,
            "description": {"type": "string"},
        },
        "required": ["name", "char", "color", "type1", "attack_type", "description"],
    }
    examples = [
        (
            {
                "theme": "nethack",
                "level": 1,
            },
            [
                {
                    "name": "grid bug",
                    "char": "x",
                    "color": "purple",
                    "type1": "bug",
                    "type2": "electric",
                    "attack_type": "electric",
                    "description": "These electronically based creatures are not native to this universe. They appear to come from a world whose laws of motion are radically different from ours.",
                },
                {
                    "name": "floating eye",
                    "char": "e",
                    "color": "blue",
                    "type1": "psychic",
                    "attack_type": "psychic",
                    "description": "Floating eyes, not surprisingly, are large, floating eyeballs which drift about the dungeon. Though not dangerous in and of themselves, their power to paralyse those who gaze at their large eye in combat is widely feared.",
                },
                {
                    "name": "yellow mold",
                    "char": "m",
                    "color": "yellow",
                    "type1": "poison",
                    "attack_type": "poison",
                    "description": "Mold, multicellular organism of the division Fungi, typified by plant bodies composed of a network of cottony filaments.",
                },
            ],
        ),
        (
            {
                "theme": "nethack",
                "level": 2,
            },
            [
                {
                    "name": "water nymph",
                    "char": "n",
                    "color": "skyblue",
                    "type1": "water",
                    "type2": "fairy",
                    "attack_type": "water",
                    "description": "A nymph's beauty is beyond words: an ever-young woman with sleek figure and long, thick hair, radiant skin and perfect teeth, full lips and gentle eyes.",
                },
                {
                    "name": "centipede",
                    "char": "s",
                    "color": "yellow",
                    "type1": "bug",
                    "type2": "poison",
                    "attack_type": "poison",
                    "description": "Here they have light reddish bodies and blue legs; great myriapedes are seen crawling every where.",
                },
                {
                    "name": "plains centaur",
                    "char": "c",
                    "color": "orange",
                    "type1": "normal",
                    "attack_type": "normal",
                    "description": "Centaurs are peculiar in that their nature, which unites the body of a horse with the trunk and head of a man, involves an unthinkable duplication of vital organs and important members.",
                },
            ],
        ),
    ]
    input = {"theme": theme, "level": level}
    return ask_google_structured(instructions, examples, input, count, schema)


def gen_setting_desc(theme: str):
    instructions = f"Write a two paragraph summary of a roguelike game based off of the following theme: {theme}. The game has three levels. Describe the setting, the kinds of monsters, items, level design, and the final boss."
    return ask_google([instructions])
