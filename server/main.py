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
@click.argument("instruction")
def gen_actions(instruction: str):
    state = game_types.GameState(**json.load(sys.stdin))
    for action in ai.gen_actions(instruction, state):
        print(action.model_dump_json(exclude_defaults=True))


def main():
    cli()


if __name__ == "__main__":
    main()
