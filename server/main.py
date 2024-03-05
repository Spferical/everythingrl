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
@click.argument("item1")
@click.argument("item2")
def craft(item1: str, item2: str):
    prompt_parts = [
        'You are the game master for a difficult permadeath roguelike with a crafting system. The player may combine any two items in the game to create a third item, similar to Homestuck captchalogue code alchemy. They will give you two items, and you will respond with the name of the result and a one-sentence visual description of it. Your responses will be JSON conforming to: `{"name": ..., "description": ...}`',
        "input: - Pogo ride\n- Sledgehammer",
        'output: {"name": "Pogo Hammer", "description": "A pogo ride with a sledgehammer welded to the handlebars."}',
        "input: - Sword\n- Corpse of snake",
        'output: {"name": "Serpent Sword", "description": "A long blade with a leather-wrapped grip and a hilt carved to resemble a snarling serpent\'s head."}',
        f"input: - {item1}\n- {item2}",
        "output: ",
    ]
    print(ai.ask_google(prompt_parts))


@cli.command()
@click.argument("theme")
@click.argument("level")
@click.argument("count")
def gen_monster(theme: str, level: str, count: int):
    print(json.dumps(ai.gen_monster(theme, int(level), int(count)), indent=2))


@cli.command()
@click.argument("theme")
def gen_setting(theme: str):
    setting_desc = ai.gen_setting_desc(theme)
    print(setting_desc)
    areas = ai.gen_areas(theme, setting_desc)
    print(json.dumps(areas, indent=2))


def main():
    cli()


if __name__ == "__main__":
    main()
