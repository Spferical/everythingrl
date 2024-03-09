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
    item1 = next(x for x in items if x["name"] == item1)
    item2 = next(x for x in items if x["name"] == item2)
    print(json.dumps(ai.craft(theme, setting_desc, items, item1, item2)))


@cli.command()
@click.argument("theme")
@click.argument("level")
@click.argument("count")
def gen_monster(theme: str, level: str, count: int):
    print(json.dumps(ai.gen_monster(theme, int(level), int(count)), indent=2))


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
    monsters = ai.gen_monsters(theme, setting_desc, areas)
    print(json.dumps(monsters, indent=2))


@cli.command()
@click.argument("theme")
@click.argument("setting_desc_file", type=click.File("r"))
@click.argument("areas_file", type=click.File("r"))
def gen_items(theme: str, setting_desc_file, areas_file):
    setting_desc = setting_desc_file.read()
    areas = json.load(areas_file)
    items = ai.gen_items(theme, setting_desc, areas)
    print(json.dumps(items, indent=2))


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
    monsters = ai.gen_monsters(theme, setting_desc, areas)
    print(json.dumps(monsters, indent=2))
    if output_dir is not None:
        with open(os.path.join(output_dir, "monsters.json"), "w") as f:
            json.dump(monsters, f)
    items = ai.gen_items(theme, setting_desc, areas)
    print(json.dumps(items, indent=2))
    if output_dir is not None:
        with open(os.path.join(output_dir, "items.json"), "w") as f:
            json.dump(items, f)


def main():
    cli()


if __name__ == "__main__":
    main()
