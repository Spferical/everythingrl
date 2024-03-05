import json
import os

import requests

API_URL = "https://api.mistral.ai"
API_KEY = os.getenv("AISTUDIO_API_KEY")


def ask_mistral(messages):
    model = "mistral-small"

    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": f"Bearer {API_KEY}",
    }

    payload = {"model": "mistral-tiny", "messages": messages, "max_tokens": 300}

    response = requests.post(
        f"{API_URL}/v1/chat/completions", headers=headers, json=payload
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
        "safetySettings": [
            {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_ONLY_HIGH"},
            {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_ONLY_HIGH"},
            {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "threshold": "BLOCK_ONLY_HIGH",
            },
            {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "threshold": "BLOCK_ONLY_HIGH",
            },
        ],
    }
    response = requests.post(url, headers=headers, json=payload)
    response.raise_for_status()
    return response.json()["candidates"][0]["content"]["parts"][0]["text"]


def ask_google_structured(input_fields: list[str], output_fields: list[str], instructions: str, examples: list[dict], input: dict) -> dict:
    prompt_parts = [instructions]
    for example in examples:
        for field in input_fields:
            prompt_parts.append(f"{field} {example[field]}")
        for field in output_fields:
            prompt_parts.append(f"{field} {example[field]}")
    for field in input_fields:
        prompt_parts.append(f"{field} {input[field]}")
    response_text = ask_google(prompt_parts)
    response_text = response_text.split(f"{output_fields[0]} ")[1]
    output = {}
    for i, field in enumerate(output_fields):
        if i == len(output_fields) - 1:
            output[field] = response_text
        else:
            next_field = output_fields[i+1]
            field_output, response_text = response_text.split(f"{next_field}")
            output[field] = field_output
    return output


def gen_monster(theme: str, level: int):
    input_fields = ["theme", "level"]
    output_fields = ["monster 1", "monster 2", "monster 3"]
    instructions = "You are the game master for a difficult permadeath roguelike. For each input theme and level, output JSON monster definitions. Valid types and attack types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. Valid colors are: lightgray yellow gold orange pink red maroon green lime skyblue blue purple violet beige brown white magenta. DO NOT generate invalid types or colors."
    examples = [
        {
            "theme": "nethack",
            "level": 1,
            "monster 1": '{"name": "grid bug", "char": "x", "color": "purple", "type1": "bug", "type2": "electric", "attack_type": "electric", "description": "These electronically based creatures are not native to this universe. They appear to come from a world whose laws of motion are radically different from ours."}',
            "monster 2": '{"name": "floating eye", "char": "e", "color": "blue", "type1": "psychic", "attack_type": "psychic", "description": "Floating eyes, not surprisingly, are large, floating eyeballs which drift about the dungeon. Though not dangerous in and of themselves, their power to paralyse those who gaze at their large eye in combat is widely feared."}',
            "monster 3": '{"name": "yellow mold", "char": "m", "color": "yellow", "type1": "poison", "attack_type": "poison", "description": "Mold, multicellular organism of the division Fungi, typified by plant bodies composed of a network of cottony filaments."}',
        },
        {
            "theme": "nethack",
            "level": 2,
            "monster 1": '{"name": "water nymph", "char": "n", "color": "skyblue", "type1": "water", "type2": "fairy", "attack_type": "water", "description": "A nymph\'s beauty is beyond words: an ever-young woman with sleek figure and long, thick hair, radiant skin and perfect teeth, full lips and gentle eyes."}',
            "monster 2": '{"name": "centipede", "char": "s", "color": "yellow", "type1": "bug", "type2": "poison", "attack_type": "poison", "description": "Here they have light reddish bodies and blue legs; great myriapedes are seen crawling every where."}',
            "monster 3": '{"name": "plains centaur", "char": "c", "color": "orange", "type1": "normal", "attack_type": "normal", "description": "Centaurs are peculiar in that their nature, which unites the body of a horse with the trunk and head of a man, involves an unthinkable duplication of vital organs and important members."}',
        },
    ]
    input = {"theme": theme, "level": level}
    response = ask_google_structured(
        input_fields, output_fields, instructions, examples, input
    )
    monsters = []
    for monster_info in (
        response["monster 1"],
        response["monster 2"],
        response["monster 3"],
    ):
        try:
            monsters.append(json.loads(monster_info))
        except Exception as e:
            print(f"Failed to load {monster_info}: {e}")
    return monsters
