#!/usr/bin/env python
import ai
import game_types


def test_generates_everything_from_scratch():
    theme = "Microcenter"
    state = game_types.GameState(theme=theme)
    instructions = "Generate everything"
    ai.gen_anything(instructions, state)
    assert len(state.areas) == 3
