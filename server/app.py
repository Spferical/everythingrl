#!/usr/bin/env python
import logging
import os
import itertools

import flask
from flask import Flask, send_from_directory, jsonify
from flask_sqlalchemy import SQLAlchemy
from sqlalchemy.orm import Mapped, mapped_column
from sqlalchemy.orm import DeclarativeBase
from werkzeug.middleware.proxy_fix import ProxyFix

import v0
import v1
import ai
import game_types


logging.basicConfig(level=logging.DEBUG)

app = Flask(__name__)

# needed for running under reverse proxy
app.wsgi_app = ProxyFix(app.wsgi_app, x_proto=1, x_host=1)

app.config["SQLALCHEMY_DATABASE_URI"] = os.environ.get(
    "DATABASE_URI", "sqlite:///db.sqlite"
)

PREGEN_THEME = "pregen"


class Base(DeclarativeBase):
    pass


@app.errorhandler(ai.AiError)
def ai_error(e):
    return jsonify({"error": str(e)}), 500


# Legacy 7drl version API
app.add_url_rule(
    "/setting/<path:theme>", view_func=v0.app.get_setting, methods=["POST"]
)
app.add_url_rule("/craft", view_func=v0.app.craft, methods=["POST"])
app.add_url_rule("/areas", view_func=v0.app.get_areas, methods=["POST"])
app.add_url_rule("/boss", view_func=v0.app.get_boss, methods=["POST"])
app.add_url_rule("/monsters", view_func=v0.app.monsters, methods=["POST"])
app.add_url_rule("/items", view_func=v0.app.items, methods=["POST"])

# Legacy v1 API
app.add_url_rule(
    "/v1/setting/<path:theme>", view_func=v1.app.v1_get_setting, methods=["POST"]
)
app.add_url_rule("/v1/craft", view_func=v1.app.v1_craft, methods=["POST"])
app.add_url_rule("/v1/areas", view_func=v1.app.v1_get_areas, methods=["POST"])
app.add_url_rule("/v1/boss", view_func=v1.app.v1_get_boss, methods=["POST"])
app.add_url_rule("/v1/monsters", view_func=v1.app.v1_monsters, methods=["POST"])
app.add_url_rule("/v1/items", view_func=v1.app.v1_items, methods=["POST"])


@app.post("/v1/actions")
def v1_actions():
    game_state = flask.request.get_json()["state"]
    game_state = game_types.GameState(**game_state)
    ask = flask.request.get_json()["ask"]
    logging.info('/v1/actions: theme="%s", ask="%s"', ask, game_state.theme)

    actions = ai.gen_actions(ask, game_state)
    # Get the first action to catch any exceptions from status errors
    first_action = next(actions)

    def generate():
        for action in itertools.chain([first_action], actions):
            yield action.model_dump_json(exclude_defaults=True)
            yield "\n"

    return generate(), {"Content-Type": "text/jsonl"}


@app.route("/")
def root():
    return send_from_directory("../dist", "index.html")


@app.route("/<path:path>")
def serve_static(path):
    return send_from_directory("../dist", path)


print(f"Using database {app.config['SQLALCHEMY_DATABASE_URI']}")

db = SQLAlchemy(model_class=Base)


class Game(db.Model):
    id: Mapped[int] = mapped_column(primary_key=True)
    theme: Mapped[str] = mapped_column()
    setting_desc: Mapped[str] = mapped_column()
    areas: Mapped[str] = mapped_column()
    monsters: Mapped[str] = mapped_column()
    items: Mapped[str] = mapped_column()


if __name__ == "__main__":
    db.init_app(app)
    app.run(debug=True)
