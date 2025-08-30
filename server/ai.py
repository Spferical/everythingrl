import json
from typing import Any
import structlog
import os
import time
from functools import cache
from collections.abc import Iterator

from google import genai
from google.genai import types as genai_types
from google.genai.errors import APIError as GenaiError
from google.auth import default

import game_types

DIR_PATH = os.path.dirname(os.path.realpath(__file__))
LOG = structlog.stdlib.get_logger()


credentials, _ = default(scopes=["https://www.googleapis.com/auth/cloud-platform"])
CLIENT = genai.Client(
    vertexai=True,
    project=os.getenv("GCLOUD_PROJECT"),
    location="us-central1",
    credentials=credentials,
)
SAFETY_SETTINGS = [
    genai_types.SafetySetting(
        category=genai_types.HarmCategory.HARM_CATEGORY_HATE_SPEECH,
        threshold=genai_types.HarmBlockThreshold.BLOCK_LOW_AND_ABOVE,
    ),
    genai_types.SafetySetting(
        category=genai_types.HarmCategory.HARM_CATEGORY_DANGEROUS_CONTENT,
        threshold=genai_types.HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
    ),
    genai_types.SafetySetting(
        category=genai_types.HarmCategory.HARM_CATEGORY_SEXUALLY_EXPLICIT,
        threshold=genai_types.HarmBlockThreshold.BLOCK_LOW_AND_ABOVE,
    ),
    genai_types.SafetySetting(
        category=genai_types.HarmCategory.HARM_CATEGORY_HARASSMENT,
        threshold=genai_types.HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
    ),
]


@cache
def get_test_str(fname: str) -> str:
    with open(os.path.join(DIR_PATH, "data", fname)) as f:
        return f.read()


@cache
def get_test_json(fname: str) -> Any:
    with open(os.path.join(DIR_PATH, "data", fname)) as f:
        return json.load(f)


class AiError(Exception):
    pass


def get_safety_error(
    safety_ratings: list[genai_types.SafetyRating],
) -> AiError:
    safety_issues: list[str] = []
    for rating in safety_ratings:
        if rating.probability != "NEGLIGIBLE":
            match rating.category:
                case genai_types.HarmCategory.HARM_CATEGORY_HATE_SPEECH:
                    safety_issues.append("hateful")
                case genai_types.HarmCategory.HARM_CATEGORY_DANGEROUS_CONTENT:
                    safety_issues.append("dangerous")
                case genai_types.HarmCategory.HARM_CATEGORY_HARASSMENT:
                    safety_issues.append("harrassment")
                case genai_types.HarmCategory.HARM_CATEGORY_SEXUALLY_EXPLICIT:
                    safety_issues.append("too horny")
                case genai_types.HarmCategory.HARM_CATEGORY_CIVIC_INTEGRITY:
                    safety_issues.append("politics")
                case genai_types.HarmCategory.HARM_CATEGORY_UNSPECIFIED:
                    safety_issues.append("idk")
                case None:
                    safety_issues.append("idk")
    return AiError(", ".join(safety_issues))


def get_response_text(response: genai_types.GenerateContentResponse) -> str:
    if response.prompt_feedback is not None:
        LOG.error(
            "Response blocked",
            block_reason=response.prompt_feedback.block_reason,
            block_message=response.prompt_feedback.block_reason_message,
        )
        if response.prompt_feedback.safety_ratings:
            raise get_safety_error(response.prompt_feedback.safety_ratings)
        else:
            raise AiError("blocked: {response.prompt_feedback.block_reason}")
    if response.candidates is not None:
        for candidate in response.candidates:
            if (
                candidate.finish_reason == "SAFETY"
                and candidate.safety_ratings is not None
            ):
                raise get_safety_error(candidate.safety_ratings)
    if response.text is None:
        LOG.debug(f"bad response: {response.model_dump_json(exclude_defaults=True)}")
        raise AiError("bad AI API response")
    return response.text


def log_spend(chars_in: int, chars_out: int):
    # TODO: these are 1.5 numbers, adjust this when 2.0 is GA
    price_est = 0.00000001875 * chars_in + 0.000000075 * chars_out
    LOG.info(f"generated {chars_out} characters from {chars_in} for ${price_est:.4f}")


def ask_google_streaming(
    prompt: str, system_prompt: str | None = None, model: str = "gemini-2.0-flash-exp"
) -> Iterator[str]:
    total_response_length = 0
    try:
        response_iter = CLIENT.models.generate_content_stream(
            model=model,
            contents=prompt,
            config=genai_types.GenerateContentConfig(
                system_instruction=system_prompt,
                safety_settings=SAFETY_SETTINGS,
            ),
        )
        for response in response_iter:
            text = get_response_text(response)
            total_response_length += len(text)
            yield text

    except GenaiError as e:
        if e.code == 429:
            LOG.error("Got 429. Retrying...")
            time.sleep(1)
            if model == "gemini-2.0-flash-exp":
                # experimental model has tight rate limits, fall back to 1.5
                model = "gemini-1.5-flash-002"
            yield from ask_google_streaming(
                prompt, system_prompt=system_prompt, model=model
            )
        else:
            raise
    finally:
        if total_response_length != 0:
            characters_in = len(prompt) + len(system_prompt or "")
            log_spend(characters_in, total_response_length)


def ask_google_stream_actions(
    instructions: str,
    examples: list[tuple[str, str]],
    input: game_types.GameState,
) -> Iterator[game_types.AiAction]:
    system_prompt = """You are the game master for a difficult permadeath roglike. You are responsible for creating and curating the content definitions for the game according to the (fixed) mechanics of the game and the desires of the player.

The game mechanics are fixed and are as follows. The player explores three randomly-generated dungeon levels (areas) and aims to defeat the boss on a small fourth final level. The player may equip up to two pieces of armor (equipment) and one melee weapon and one ranged weapon. They may store extra equipment in their inventory and eat food items to regain health. Weapons, armor, food, and enemies all have Pokemon types that influence their effectiveness. All also have levels, which make them directly more effective.

Some monsters and weapons may have a special `modifiers` list that applies status effects on-hit. For monsters, this is `MonsterModifier`, and for items, it is `ItemModifier`. These can be `poison` (damage over time), `burn` (high damage over a short time), `bleed` (medium damage over a medium time), or `stun` (prevents the target from acting for a turn). Use these to create more interesting and dangerous encounters and items.

There are three levels (areas) in the game. Each should have at least 5 possible monsters found in that level, 5 pieces of armor, 3 melee weapons, 2 ranged weapons, and 3 food items. Try to avoid common or generic roguelike items.

The player may choose between 5 varied starting characters. These may be named people or classes. Each should include a name, a paragraph-long backstory that includes their motivation, and 5 starting items that include at least 1 piece of armor and a melee weapon.

The player may combine any two items in the game to create a third item, similar to Homestuck captchalogue code alchemy. The two input items must be the same level; when asked to generate a crafting recipe, assign a level to the output item that is the level of each input item plus one; e.g. 2xL1->L2, 2xL2->L3, etc. DO NOT output crafting recipes unless specifically requested.

You will be given a JSON object describing the current content definitions for a game. Produce JSON-L, i.e. one or multiple compact JSON objects separated by newlines, describing each _change_ that should be done to the content definitions according to the given instructions. Output according to the jsonschema definition given below. NEVER output markdown or backticks. NEVER output definitions that exist in the Game JSON already, unless they must be replaced. AVOID bland or generic descriptions; prefer short and poignant quips.

At any time, you may ABORT generation if the user-provided theme is truly heinous.
"""
    for example_input, example_output in examples:
        system_prompt += (
            f"\n\nExample input: {example_input}\nExample output: {example_output}"
        )
    prompt = f"Game JSON: {input.model_dump_json(exclude_defaults=True)}\nOutput schema: {game_types.AiAction.model_json_schema()}\nInstructions: {instructions}"
    LOG.debug(f"ASKING: {system_prompt}\n{prompt}")
    response_text = ""
    actions_emitted = 0
    last_exc = None
    for text_chunk in ask_google_streaming(prompt, system_prompt=system_prompt):
        response_text += text_chunk
        [*complete_lines, response_text] = response_text.split("\n")
        for complete_line in complete_lines:
            try:
                LOG.debug(f"RECEIVED: {complete_line}")
                if not complete_line or complete_line.startswith("```"):
                    # No use logging this junk
                    continue
                yield game_types.AiAction(**json.loads(complete_line))
                actions_emitted += 1
            except Exception as e:
                LOG.debug("Bad response", line=complete_line, exc_info=e)
                last_exc = e
    if actions_emitted == 0:
        if last_exc is not None:
            raise last_exc
        else:
            raise AiError("Failed to generate any actions")


def gen_actions(
    instructions: str, state: game_types.GameState
) -> Iterator[game_types.AiAction]:
    examples: list[tuple[str, str]] = []
    if instructions == "Generate everything":
        pirates_input = game_types.GameState(theme="Pirates").model_dump_json(
            exclude_defaults=True
        )
        pirates_state = game_types.GameState(**get_test_json("pirates.json"))
        pirates_output: list[game_types.AiAction] = []
        pirates_output.append(
            game_types.AiAction(set_setting_desc=pirates_state.setting_desc)
        )
        for area in pirates_state.areas:
            pirates_output.append(game_types.AiAction(add_area=area))
        for monster_def in pirates_state.monsters:
            pirates_output.append(game_types.AiAction(add_monster_def=monster_def))
        for item_def in pirates_state.items:
            pirates_output.append(game_types.AiAction(add_item_def=item_def))
        pirates_output.append(game_types.AiAction(set_boss=pirates_state.boss))
        for character in pirates_state.characters:
            pirates_output.append(game_types.AiAction(add_character=character))
        pirates_output_str = "\n".join(
            x.model_dump_json(exclude_defaults=True) for x in pirates_output
        )
        examples.append((pirates_input, pirates_output_str))
    yield from ask_google_stream_actions(instructions, examples, state)
