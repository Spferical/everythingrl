#!/usr/bin/env python
import os
import json

import click

import ai


@click.group()
def cli():
    pass


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
def gen_monster(theme: str, level: str):
    input_fields = ["theme", "level"]
    output_fields = ["monster 1", "monster 2", "monster 3"]
    instructions = "You are the game master for a difficult permadeath roguelike. For each input theme and level, output JSON monster definitions. Valid types are pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. Valid colors are: lightgray yellow gold orange pink red maroon green lime skyblue blue purple violet beige brown white magenta."
    examples = [
        {
            "theme": "nethack",
            "level": 1,
            "monster 1": '{"name": "grid bug", "char": "x", "color": "purple", "type1": "bug", "type2": "electric", "attack_type": "electric", "description": "These electronically based creatures are not native to this universe. They appear to come from a world whose laws of motion are radically different from ours."}',
            "monster 2": '{"name": "floating eye", "char": "e", "color": "blue", "type1": "psychic", "attack_type": "psychic", "description": "Floating eyes, not surprisingly, are large, floating eyeballs which drift about the dungeon. Though not dangerous in and of themselves, their power to paralyse those who gaze at their large eye in combat is widely feared."}',
            "monster 3": '{"name": "yellow mold", "char": "m", "color": "yellow", "type1": "poison", "attack_type": "poison", "description": "Mold, multicellular organism of the division Fungi, typified by plant bodies composed of a network of cottony filaments."}',
        },
        {
            "theme": "nethack",
            "level": 2,
            "monster 1": '{"name": "water nymph", "char": "n", "color": "skyblue", "type1": "water", "type2": "fairy", "attack_type": "water", "description": "A nymph\'s beauty is beyond words: an ever-young woman with sleek figure and long, thick hair, radiant skin and perfect teeth, full lips and gentle eyes."}',
            "monster 2": '{"name": "centipede", "char": "s", "color": "yellow", "type1": "bug", "type2": "poison", "attack_type": "poison", "description": "Here they have light reddish bodies and blue legs; great myriapedes are seen crawling every where."}',
            "monster 3": '{"name": "plains centaur", "char": "c", "color": "orange", "type1": "normal", "attack_type": "normal", "description": "Centaurs are peculiar in that their nature, which unites the body of a horse with the trunk and head of a man, involves an unthinkable duplication of vital organs and important members."}',
        },
    ]
    input = {"theme": theme, "level": level}
    response = ai.ask_google_structured(
        input_fields, output_fields, instructions, examples, input
    )
    for monster_info in (
        response["monster 1"],
        response["monster 2"],
        response["monster 3"],
    ):
        try:
            print(json.loads(monster_info))
        except Exception as e:
            print(f"Failed to load {monster_info}: {e}")


@cli.command()
def server():
    import web

    web.run_server()


def main():
    cli()


if __name__ == "__main__":
    main()
