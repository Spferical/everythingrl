#!/usr/bin/env python
import json
import logging
import os

import click

import ai


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
@click.argument("level")
@click.argument("count")
def gen_monster(theme: str, level: str, count: int):
    raise NotImplementedError()


@cli.command()
@click.argument("theme")
def gen_setting_desc(theme: str):
    setting_desc = ai.gen_setting_desc(theme)
    print(setting_desc)


@cli.command()
@click.argument("theme")
@click.argument("setting_desc_file", type=click.File("r"))
def gen_areas(theme: str, setting_desc_file):
    setting_desc = setting_desc_file.read()
    areas = ai.gen_areas(theme, setting_desc)
    print(json.dumps(areas, indent=2))


@cli.command()
@click.argument("theme")
@click.argument("setting_desc_file", type=click.File("r"))
@click.argument("areas_file", type=click.File("r"))
def gen_monsters(theme: str, setting_desc_file, areas_file):
    setting_desc = setting_desc_file.read()
    areas = json.load(areas_file)
    names_needed = set(m for area in areas for m in area["enemies"])
    monsters = []
    while names_needed:
        logging.info(f"need {names_needed}")
        for monster in ai.gen_monsters(theme, setting_desc, list(names_needed)):
            monsters.append(monster)
            if monster["name"] in names_needed:
                names_needed.remove(monster["name"])
    print(json.dumps(monsters, indent=2))


@cli.command()
@click.argument("theme")
@click.argument("setting_desc_file", type=click.File("r"))
@click.argument("areas_file", type=click.File("r"))
def gen_items(theme: str, setting_desc_file, areas_file):
    setting_desc = setting_desc_file.read()
    areas = json.load(areas_file)
    names_needed = set(
        name
        for area in areas
        for name in area["equipment"]
        + area["melee_weapons"]
        + area["ranged_weapons"]
        + area["food"]
    )
    items = []
    while names_needed:
        print("need", names_needed)
        for item in ai.gen_items(theme, setting_desc, list(names_needed)):
            items.append(item)
            if item["name"] in names_needed:
                names_needed.remove(item["name"])
    print(json.dumps(items, indent=2))


@cli.command()
@click.argument("theme")
@click.argument("setting_desc_file", type=click.File("r"))
def gen_boss(theme: str, setting_desc_file):
    setting_desc = setting_desc_file.read()
    boss = ai.gen_boss(theme, setting_desc)
    print(json.dumps(boss, indent=2))


@cli.command()
@click.argument("theme")
@click.option("--output-dir", default=None)
def gen_all(theme: str, output_dir: str | None):
    setting_desc = ai.gen_setting_desc(theme)
    print(setting_desc)
    if output_dir is not None:
        with open(os.path.join(output_dir, "setting.txt"), "w") as f:
            f.write(setting_desc)
    areas = ai.gen_areas(theme, setting_desc)
    print(json.dumps(areas, indent=2))
    if output_dir is not None:
        with open(os.path.join(output_dir, "areas.json"), "w") as f:
            json.dump(areas, f)
    monster_names = set(name for area in areas for name in area["enemies"])
    monsters = ai.gen_monsters(theme, setting_desc, list(monster_names))
    print(json.dumps(monsters, indent=2))
    if output_dir is not None:
        with open(os.path.join(output_dir, "monsters.json"), "w") as f:
            json.dump(monsters, f)

    item_names = set(
        name
        for area in areas
        for name in area["equipment"]
        + area["melee_weapons"]
        + area["ranged_weapons"]
        + area["food"]
    )
    items = ai.gen_items(theme, setting_desc, list(item_names))
    print(json.dumps(items, indent=2))
    if output_dir is not None:
        with open(os.path.join(output_dir, "items.json"), "w") as f:
            json.dump(items, f)
    boss = ai.gen_boss(theme, setting_desc)
    if output_dir is not None:
        with open(os.path.join(output_dir, "boss.json"), "w") as f:
            json.dump(boss, f)
    print(json.dumps(boss, indent=2))


def main():
    cli()


if __name__ == "__main__":
    main()
