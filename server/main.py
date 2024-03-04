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
    prompt_parts = [
      "You are the game master for a difficult permadeath roguelike. Monsters have a level and one or two pokemon types, i.e. one of: normal fire water electric grass ice fighting poison ground flying psychic bug rock ghost dragon dark steel fairy. For each input theme and level, output JSON monster descriptions.",
      "theme nethack",
      "level 1",
      "monster 1 {\"name\": \"grid bug\", \"type1\": \"bug\", \"type2\": \"electric\", \"attack_type\": \"electric\", \"description\": \"These electronically based creatures are not native to this universe. They appear to come from a world whose laws of motion are radically different from ours.\"}",
      "monster 2 {\"name\": \"floating eye\", \"type1\": \"psychic\", \"attack_type\": \"psychic\", \"description\": \"Floating eyes, not surprisingly, are large, floating eyeballs which drift about the dungeon. Though not dangerous in and of themselves, their power to paralyse those who gaze at their large eye in combat is widely feared.\"}",
      "monster 3 {\"name\": \"yellow mold\", \"type1\": \"poison\", \"attack_type\": \"poison\", \"description\": \"Mold, multicellular organism of the division Fungi, typified by plant bodies composed of a network of cottony filaments.\"}",
      "theme nethack",
      "level 2",
      "monster 1 {\"name\": \"water nymph\", \"type1\": \"water\", \"type2\": \"fairy\", \"attack_type\": \"water\", \"description\": \"A nymph's beauty is beyond words: an ever-young woman with sleek figure and long, thick hair, radiant skin and perfect teeth, full lips and gentle eyes.\"}",
      "monster 2 {\"name\": \"centipede\", \"type1\": \"bug\", \"type2\": \"poison\", \"attack_type\": \"poison\", \"description\": \"Here they have light reddish bodies and blue legs; great myriapedes are seen crawling every where.\"}",
      "monster 3 {\"name\": \"plains centaur\", \"type1\": \"normal\", \"attack_type\": \"normal\", \"description\": \"Centaurs are peculiar in that their nature, which unites the body of a horse with the trunk and head of a man, involves an unthinkable duplication of vital organs and important members.\"}",
      f"theme {theme}",
      f"level {level}",
      "monster 1 ",
    ]
    response_text = ai.ask_google(prompt_parts)
    response1, etc = response_text.split('monster 2')
    response2, response3 = etc.split('monster 3')
    for resp in (response1, response2, response3):
        try:
            print(json.loads(resp))
        except Exception as e:
            print(e)

@cli.command()
def server():
    import web

    web.run_server()


def main():
    cli()


if __name__ == "__main__":
    main()
