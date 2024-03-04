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
API_KEY = os.getenv("AISTUDIO_API_KEY")


def ask_mistral(messages):
    model = "mistral-small"

    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Authorization": f"Bearer {API_KEY}",
    }

    payload = {"model": "mistral-tiny", "messages": messages, "max_tokens": 300}

    response = requests.post(
        f"{API_URL}/v1/chat/completions", headers=headers, json=payload
    )

    response.raise_for_status()

    return response.json()["choices"][0]["message"]["content"]


def ask_google(prompt_parts: list[str]):
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-1.0-pro:generateContent?key={API_KEY}"
    headers = {"Content-Type": "application/json"}
    payload = {
        "contents": [
            {
                "parts": [{"text": part} for part in prompt_parts],
            }
        ],
        "generationConfig": {
            "temperature": 0.9,
            "topK": 1,
            "topP": 1,
            "maxOutputTokens": 2048,
            "stopSequences": [],
        },
        "safetySettings": [
            {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_ONLY_HIGH"},
            {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_ONLY_HIGH"},
            {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "threshold": "BLOCK_ONLY_HIGH",
            },
            {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "threshold": "BLOCK_ONLY_HIGH",
            },
        ],
    }
    response = requests.post(url, headers=headers, json=payload)
    response.raise_for_status()
    return response.json()['candidates'][0]['content']['parts'][0]['text']


class Base(DeclarativeBase):
    pass


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
    print(ask_google(prompt_parts))


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
