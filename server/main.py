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
    tries = 0
    while True:
        remaining_requirements = []
        if state.theme is None:
            remaining_requirements.append(
                "a setting_desc string describing the style, mood, substance, and high-level ideas to inform the artistic direction and content for the game.")
        if len(state.areas) < 3:
            remaining_requirements.append("three areas i.e. levels for the player to explore on his way to the final boss")
        for area in state.areas:
            missing_monster_defs = set()
            for monster_name in area.enemies:
                if not any(m.name == monster_name for m in state.monsters):
                    missing_monster_defs.add(monster_name)
            for monster_name in missing_monster_defs:
                remaining_requirements.append(f"a monster definition for {monster_name}")
            missing_item_defs = set()
            for item_list in [area.equipment, area.melee_weapons, area.ranged_weapons, area.food]:
                for item_name in item_list:
                    if not any(item.name == item_name for item in state.items):
                        missing_item_defs.add(item_name)
            for item_name in missing_item_defs:
                remaining_requirements.append(f"an item definition for {item_name}")
        if state.boss is None:
            remaining_requirements.append("a final boss")
        if not remaining_requirements: break
        instructions = f"Generate any missing data for the game. The game still requires at least:\n"
        instructions += "\n".join(f"- {req}" for req in remaining_requirements)
        ai.ask_google_json_merge(instructions, [], state)
        tries += 1
    print(f'done after {tries} tries')
    print(state)
    if output_dir is not None:
        with open(os.path.join(output_dir, "game.json"), "w") as f:
            f.write(state.model_dump_json())


def main():
    cli()


if __name__ == "__main__":
    main()
