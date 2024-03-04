#!/usr/bin/env python
import os

import click
import requests
from flask import Flask
from flask_sqlalchemy import SQLAlchemy
from sqlalchemy.orm import DeclarativeBase
from werkzeug.middleware.proxy_fix import ProxyFix
from werkzeug.serving import is_running_from_reloader


app = Flask(__name__)

# needed for running under reverse proxy
app.wsgi_app = ProxyFix(app.wsgi_app, x_proto=1, x_host=1)

app.config["SQLALCHEMY_DATABASE_URI"] = os.environ.get(
    "DATABASE_URI", "sqlite:///db.sqlite"
)


API_URL = "https://api.mistral.ai"
API_KEY = os.getenv("MISTRAL_API_KEY")


def ask_mistral(messages):
    model = "mistral-small"

    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": f"Bearer {API_KEY}",
    }

    payload = {"model": "mistral-medium", "messages": messages, "max_tokens": 300}

    response = requests.post(
        f"{API_URL}/v1/chat/completions", headers=headers, json=payload
    )

    return response.json()["choices"][0]["message"]["content"]


class Base(DeclarativeBase):
    pass


@click.group()
def cli():
    pass


@cli.command()
@click.argument("item1")
@click.argument("item2")
def craft(item1: str, item2: str):
    print(ask_mistral(
        [
            {
                "role": "system",
                "content": 'You are the game master for a difficult permadeath roguelike with a crafting system. The player may combine any two items in the game to create a third item, similar to Homestuck captchalogue code alchemy. They will give you two items, and you will respond with the name of the result and a one-sentence visual description of it. Your responses will be JSON conforming to: `{"name": ..., "description": ...}`',
            },
            {
                "role": "user",
                "content": f"- {item1}\n- {item2}",
            },
        ]
    ))


@cli.command()
def server():
    print(f"Using database {app.config['SQLALCHEMY_DATABASE_URI']}")
    db = SQLAlchemy(model_class=Base)
    db.init_app(app)

    app.run(debug=True)


def main():
    cli()


if __name__ == "__main__":
    main()
