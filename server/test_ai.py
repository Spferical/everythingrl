#!/usr/bin/env python
import json
from unittest.mock import patch
import ai
import game_types


@patch("ai.ask_google_stream_actions")
def test_generates_everything_from_scratch_mocked(mock_ask_google):
    with open("data/cyberpunk_actions.json") as f:
        actions_json = json.load(f)

    mock_ask_google.return_value = (game_types.AiAction(**action) for action in actions_json)

    theme = "Cyberpunk"
    state = game_types.GameState(theme=theme)
    instructions = "Generate everything"

    for action in ai.gen_actions(instructions, state):
        state.apply_action(action)

    with open("data/cyberpunk.json") as f:
        expected_state = game_types.GameState(**json.load(f))

    assert state == expected_state
