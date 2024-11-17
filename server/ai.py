import json
import logging
import os
import time
from functools import cache
from typing import cast, Type

import pydantic
import requests
from requests.adapters import Retry, HTTPAdapter

import google.api_core.exceptions
import vertexai
from vertexai.preview.generative_models import GenerativeModel, GenerationResponse
import vertexai.preview.generative_models as generative_models
import game_types

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


def ask_google_vertex_ai(prompt_parts: list[str], system_instruction=None) -> str:
    model = GenerativeModel(
        "gemini-1.5-flash-001", system_instruction=system_instruction
    )
    try:
        responses = cast(
            GenerationResponse,
            model.generate_content(
                "".join(prompt_parts),
                generation_config={
                    "max_output_tokens": 8192,
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
        input_tokens = responses.usage_metadata.prompt_token_count
        output_tokens = responses.usage_metadata.candidates_token_count
        price_est = 0.0000003125 * input_tokens + 0.00000125 * output_tokens
        candidate = responses.candidates[0]
        if candidate.finish_reason == generative_models.FinishReason.SAFETY:
            logging.error(candidate)
            raise get_safety_error(candidate.safety_ratings)
        text = candidate.content.parts[0].text
        text = text.strip("--").strip("```json").strip("```").strip("\n")
        logging.info(f"generated {output_tokens} tokens for ${price_est:.4f}")
        return text
    except KeyError:
        logging.error(responses)
        raise
    except IndexError:
        logging.error(responses)
        raise


def ask_google(prompt_parts: list[str], system_instruction: str | None = None):
    return ask_google_vertex_ai(prompt_parts, system_instruction=system_instruction)


def ask_google_structured(
    instructions: str,
    examples: list[tuple[dict, list[dict]]],
    input: dict,
    num_outputs: int,
    model: Type[pydantic.BaseModel],
) -> list[dict]:
    # Build prompt
    prompt_parts = [instructions, "--"]
    prompt_parts.append(
        "Produce JSON-L output, that is, one JSON object per line. Expected JSON schema of each output line: "
    )
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


def ask_google_big_prompt(
    instructions: str,
    examples: list[tuple[dict, dict]],
    input: game_types.GameState,
) -> game_types.AiAction:
    system_prompt = """You are the game master for a difficult permadeath roguelike. You are responsible for creating and curating the content definitions for the game according to the (fixed) mechanics of the game and the desires of the player.

You will be given a JSON object describing the current content definitions for a game. Produce JSON describing the _change_ that should be done to the content definitions according to the given instructions. Output according to the jsonschema definition given below.

The game mechanics are fixed and are as follows. The player explores three randomly-generated dungeon levels (areas) and aims to defeat the boss on a small fourth final level. The player may equip up to two pieces of armor (equipment) and one melee weapon and one ranged weapon. They may store extra equipment in their inventory and eat food items to regain health. Weapons, armor, food, and enemies all have Pokemon types that influence their effectiveness. All also have levels, which make them directly more effective.

There are three levels (areas) in the game. Each should have at least 5 possible monsters found in that level, 5 pieces of armor, 3 melee weapons, 2 ranged weapons, and 3 food items. Try to avoid common or generic roguelike items.

The player may choose between 5 varied starting characters. These may be named people or classes. Each should include a name, a paragraph-long backstory that includes their motivation, and 5 starting items that include at least 1 piece of armor and a melee weapon."""
    for example_input, example_output in examples:
        system_prompt += (
            f"\n\nExample input: {example_input}\nExample output: {example_output}"
        )
    prompt = f"Game JSON: {input.model_dump_json()}\nOutput schema: {game_types.AiAction.model_json_schema()}\nInstructions: {instructions}"
    logging.debug(f"ASKING: {prompt}")
    response_text = ask_google([prompt], system_instruction=system_prompt)
    logging.info(f"RECEIVED: {response_text}")
    try:
        response_json = json.loads(response_text)
        return game_types.AiAction(**response_json)
    except Exception as e:
        logging.error(f"Bad response: {response_text}: {e}")
        raise


def ask_google_json_merge(
    instructions: str, examples: list[tuple[dict, dict]], state: game_types.GameState
):
    """Mutates and returns the game state."""
    output = ask_google_big_prompt(instructions, examples, state)
    logging.info(f"AI Action: {output}")
    state.apply_action(output)


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
        game_types.Item,
    )[0]


def get_missing_requirements(state: game_types.GameState) -> list[str]:
    missing_requirements = []
    if state.setting_desc is None:
        missing_requirements.append(
            "a setting_desc string describing the style, mood, substance, and high-level ideas to inform the artistic direction and content for the game."
        )
    if len(state.areas) < 3:
        missing_requirements.append(
            "three areas i.e. levels for the player to explore on his way to the final boss"
        )
    mentioned_monsters = set(name for area in state.areas for name in area.enemies)
    defined_monsters = set(m.name for m in state.monsters)
    for monster_name in mentioned_monsters - defined_monsters:
        missing_requirements.append(f"a monster definition for {monster_name}")
    mentioned_items = set(
        name
        for area in state.areas
        for item_list in [
            area.equipment,
            area.melee_weapons,
            area.ranged_weapons,
            area.food,
        ]
        for name in item_list
    )
    mentioned_items.update(
        name for character in state.characters for name in character.starting_items
    )
    defined_items = set(item.name for item in state.items)
    for item_name in mentioned_items - defined_items:
        missing_requirements.append(f"an item definition for {item_name}")
    if state.boss is None:
        missing_requirements.append("a final boss")
    if len(state.characters) < 3:
        missing_requirements.append(
            "at least 3 characters or character classes available to the player"
        )
    return missing_requirements


def gen_anything(instructions: str, state: game_types.GameState):
    ask_google_json_merge(instructions, [], state)
    generations = 1
    while True:
        missing_requirements = get_missing_requirements(state)
        if not missing_requirements:
            break
        instructions = f"Generate any missing data for the game. The game still requires at least:\n"
        instructions += "\n".join(f"- {req}" for req in missing_requirements)
        ask_google_json_merge(instructions, [], state)
        generations += 1
    print(f"done after {generations} generations")
    print(state)


vertexai.init(project=os.getenv("GCLOUD_PROJECT"), location="us-east4")
