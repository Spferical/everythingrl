#!/usr/bin/env python
import json
import logging
import os

import click

import ai
import game_types


@click.group()
def cli():
    loglevel = os.environ.get('LOGLEVEL', 'INFO').upper()
    logging.basicConfig(level=loglevel)


@cli.command()
@click.argument("theme")
@click.argument("setting_desc_file", type=click.File("r"))
@click.argument("items_file", type=click.File("r"))
@click.argument("item1")
@click.argument("item2")
def craft(theme: str, setting_desc_file, items_file, item1: str, item2: str):
    setting_desc = setting_desc_file.read()
    items = json.load(items_file)
    item1_obj = next(x for x in items if x["name"] == item1)
    item2_obj = next(x for x in items if x["name"] == item2)
    print(json.dumps(ai.craft(theme, setting_desc, items, item1_obj, item2_obj)))


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


def main():
    cli()


if __name__ == "__main__":
    main()
