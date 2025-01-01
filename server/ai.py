import json
import logging
import os
import time
from functools import cache
from typing import Type

import pydantic
from google import genai
from google.genai import types as genai_types
from google.genai.errors import APIError as GenaiError
from google.auth import default

import game_types

DIR_PATH = os.path.dirname(os.path.realpath(__file__))


credentials, _ = default(scopes=["https://www.googleapis.com/auth/cloud-platform"])
CLIENT = genai.Client(
    vertexai=True,
    project=os.getenv("GCLOUD_PROJECT"),
    location="us-central1",
    credentials=credentials,
)
SAFETY_SETTINGS = [
    genai_types.SafetySetting(
        category="HARM_CATEGORY_HATE_SPEECH",
        threshold="BLOCK_LOW_AND_ABOVE",
    ),
    genai_types.SafetySetting(
        category="HARM_CATEGORY_DANGEROUS_CONTENT",
        threshold="BLOCK_MEDIUM_AND_ABOVE",
    ),
    genai_types.SafetySetting(
        category="HARM_CATEGORY_SEXUALLY_EXPLICIT",
        threshold="BLOCK_LOW_AND_ABOVE",
    ),
    genai_types.SafetySetting(
        category="HARM_CATEGORY_HARASSMENT",
        threshold="BLOCK_MEDIUM_AND_ABOVE",
    ),
]


@cache
def get_test_str(fname):
    with open(os.path.join(DIR_PATH, "data", fname)) as f:
        return f.read()


@cache
def get_test_json(fname):
    with open(os.path.join(DIR_PATH, "data", fname)) as f:
        return json.load(f)


class AiError(Exception):
    pass


def get_safety_error(
    safety_ratings: list[genai_types.SafetyRating],
) -> AiError:
    safety_issues = []
    for rating in safety_ratings:
        if rating.probability != "NEGLIGIBLE":
            match rating.category:
                case "HARM_CATEGORY_HATE_SPEECH":
                    safety_issues.append("hateful")
                case "HARM_CATEGORY_DANGEROUS_CONTENT":
                    safety_issues.append("dangerous")
                case "HARM_CATEGORY_HARASSMENT":
                    safety_issues.append("harrassment")
                case "HARM_CATEGORY_SEXUALLY_EXPLICIT":
                    safety_issues.append("too horny")
    return AiError(", ".join(safety_issues))


def ask_google(
    prompt_parts: list[str], system_instruction=None, model="gemini-2.0-flash-exp"
) -> str:
    try:
        full_prompt_contents = "".join(prompt_parts)
        response = CLIENT.models.generate_content(
            model=model,
            contents=full_prompt_contents,
            config=genai_types.GenerateContentConfig(
                system_instruction=system_instruction,
                stop_sequences=["--"],
                safety_settings=SAFETY_SETTINGS,
            ),
        )
    except GenaiError as e:
        if e.code == 429:
            logging.error("Got 429. Retrying...")
            time.sleep(1)
            if model == "gemini-2.0-flash-exp":
                # experimental model has tight rate limits, fall back to 1.5
                model = "gemini-1.5-flash-002"
            return ask_google(
                prompt_parts, system_instruction=system_instruction, model=model
            )
        else:
            raise

    # TODO: these are 1.5 numbers, adjust this when 2.0 is GA
    characters_in = len(full_prompt_contents) + len(system_instruction or "")
    characters_out = len(response.text or "")
    price_est = 0.00000001875 * characters_in + 0.000000075 * characters_out
    logging.info(
        f"generated {characters_out} characters from {characters_in} for ${price_est:.4f}"
    )

    if response.prompt_feedback is not None:
        logging.error(
            f"Response blocked: {response.prompt_feedback.block_reason}: {response.prompt_feedback.block_reason_message}"
        )
        if response.prompt_feedback.safety_ratings:
            raise get_safety_error(response.prompt_feedback.safety_ratings)
        else:
            raise RuntimeError("blocked: {response.prompt_feedback.block_reason}")
    if response.candidates is not None:
        for candidate in response.candidates:
            if (
                candidate.finish_reason == "SAFETY"
                and candidate.safety_ratings is not None
            ):
                raise get_safety_error(candidate.safety_ratings)
    if response.text is None:
        logging.error(
            f"bad response: {response.model_dump_json(exclude_defaults=True)}"
        )
        raise RuntimeError("bad AI API response")
    text = response.text.strip("--").strip("```json").strip("```").strip("\n")
    return text


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
    logging.debug(f"RECEIVED: {response_text}")
    responses = response_text.split("\n")
    output = []
    for response in responses:
        try:
            response_json = json.loads(response)
            output.append(model(**response_json).model_dump())
            prompt_parts.append(response_text)
        except Exception as e:
            logging.debug(f"Bad response: {response}: {e}")
    return output


def ask_google_big_prompt(
    instructions: str,
    examples: list[tuple[dict, dict]],
    input: game_types.GameState,
) -> list[game_types.AiAction]:
    system_prompt = """You are the game master for a difficult permadeath roguelike. You are responsible for creating and curating the content definitions for the game according to the (fixed) mechanics of the game and the desires of the player.

The game mechanics are fixed and are as follows. The player explores three randomly-generated dungeon levels (areas) and aims to defeat the boss on a small fourth final level. The player may equip up to two pieces of armor (equipment) and one melee weapon and one ranged weapon. They may store extra equipment in their inventory and eat food items to regain health. Weapons, armor, food, and enemies all have Pokemon types that influence their effectiveness. All also have levels, which make them directly more effective.

There are three levels (areas) in the game. Each should have at least 5 possible monsters found in that level, 5 pieces of armor, 3 melee weapons, 2 ranged weapons, and 3 food items. Try to avoid common or generic roguelike items.

The player may choose between 5 varied starting characters. These may be named people or classes. Each should include a name, a paragraph-long backstory that includes their motivation, and 5 starting items that include at least 1 piece of armor and a melee weapon.

You will be given a JSON object describing the current content definitions for a game. Produce JSON-L, i.e. one or multiple compact JSON objects separated by newlines, describing each _change_ that should be done to the content definitions according to the given instructions. Output according to the jsonschema definition given below. NEVER output markdown or backticks. NEVER output definitions that exist in the Game JSON already, unless they must be replaced. AVOID bland or generic descriptions; prefer short and poignant quips.
"""
    for example_input, example_output in examples:
        system_prompt += (
            f"\n\nExample input: {example_input}\nExample output: {example_output}"
        )
    prompt = f"Game JSON: {input.model_dump_json(exclude_defaults=True)}\nOutput schema: {game_types.AiAction.model_json_schema()}\nInstructions: {instructions}"
    logging.debug(f"ASKING: {system_prompt}\n{prompt}")
    response_text = ask_google([prompt], system_instruction=system_prompt)
    logging.debug(f"RECEIVED: {response_text}")
    actions = []
    last_exc = None
    for line in response_text.splitlines():
        try:
            response_json = json.loads(line)
            actions.append(game_types.AiAction(**response_json))
        except Exception as e:
            logging.debug(f"Bad response: {response_text}: {e}")
            last_exc = e
    if len(actions) == 0:
        if last_exc is not None:
            raise last_exc
        else:
            raise RuntimeError("Failed to generate any actions")
    return actions


def ask_google_json_merge(
    instructions: str, examples: list[tuple[dict, dict]], state: game_types.GameState
):
    """Mutates and returns the game state."""
    output = ask_google_big_prompt(instructions, examples, state)
    for action in output:
        logging.debug(f"AI Action: {action.model_dump_json(exclude_defaults=True)}")
        state.apply_action(action)


def craft(theme: str, setting_desc: str, items: list[str], item1: dict, item2: dict):
    instructions = f"You are the game master for a difficult permadeath roguelike with a crafting system. The player may combine any two items in the game to create a third item, similar to Homestuck captchalogue code alchemy. As input, you will be given a theme, a long-form description of the setting, descriptions of each item, and a list of items already in the game (do not copy any of these). Output each item JSON on its own line. DO NOT reference gameplay mechanics that aren't in the game; instead, focus on appearance and lore. The two input items must be the same level; assign a level to the output item that is the level of each input item plus one; e.g. 2xL1->L2, 2xL2->L3, etc."
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
    examples = []
    if instructions == "Generate everything":
        pirates_input = game_types.GameState(theme="Pirates").model_dump_json(
            exclude_defaults=True
        )
        pirates_state = game_types.GameState(**get_test_json("pirates.json"))
        pirates_output = []
        pirates_output.append(game_types.AiAction(set_setting_desc=pirates_state.setting_desc))
        for area in pirates_state.areas:
            pirates_output.append(game_types.AiAction(add_area=area))
        for monster_def in pirates_state.monsters:
            pirates_output.append(game_types.AiAction(add_monster_def=monster_def))
        for item_def in pirates_state.items:
            pirates_output.append(game_types.AiAction(add_item_def=item_def))
        pirates_output.append(game_types.AiAction(set_boss=pirates_state.boss))
        for character in pirates_state.characters:
            pirates_output.append(game_types.AiAction(add_character=character))
        pirates_output = "\n".join(
            x.model_dump_json(exclude_defaults=True) for x in pirates_output
        )
        examples.append((pirates_input, pirates_output))
    ask_google_json_merge(instructions, examples, state)
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
