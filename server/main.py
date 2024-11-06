#!/usr/bin/env python
import json
import logging
import os

import click

import ai
import game_types


@click.group()
def cli():
    logging.basicConfig(level=logging.DEBUG)


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
@click.option("--output-dir", default=None)
def gen_all(theme: str, output_dir: str | None):
    state = game_types.GameState(theme=theme)
    ai.gen_anything("Generate everything", state)
    print(state)
    if output_dir is not None:
        os.makedirs(output_dir, exist_ok=True)
        with open(os.path.join(output_dir, "game.json"), "w") as f:
            f.write(state.model_dump_json())


def main():
    cli()


if __name__ == "__main__":
    main()
