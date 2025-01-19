#!/usr/bin/env python
import json
import logging
import os
import sys

import click

import ai
import game_types


@click.group()
def cli():
    loglevel = os.environ.get("LOGLEVEL", "INFO").upper()
    logging.basicConfig(level=loglevel)


@cli.command()
@click.argument("theme")
@click.option("--initial-state-path", default=None)
@click.option("--output-dir", default=None)
def gen_all(theme: str, initial_state_path: str | None, output_dir: str | None):
    if initial_state_path is not None:
        with open(initial_state_path) as f:
            init = json.load(f)
            assert init["theme"] == theme
            state = game_types.GameState(**init)
    else:
        state = game_types.GameState(theme=theme)
    instruction = (
        "Generate everything" if not initial_state_path else "Generate everything else"
    )
    ai.gen_anything(instruction, state)
    print(state.model_dump_json(exclude_defaults=True))
    if output_dir is not None:
        os.makedirs(output_dir, exist_ok=True)
        with open(os.path.join(output_dir, "game.json"), "w") as f:
            f.write(state.model_dump_json(exclude_defaults=True))


@cli.command()
@click.argument("instruction")
def gen_actions(instruction: str):
    state = game_types.GameState(**json.load(sys.stdin))
    for action in ai.gen_actions(instruction, state):
        print(action.model_dump_json(exclude_defaults=True))


def main():
    cli()


if __name__ == "__main__":
    main()
