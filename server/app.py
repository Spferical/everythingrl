#!/usr/bin/env python
import json
import logging
import os

import flask
from flask import Flask, send_from_directory, jsonify
from flask_sqlalchemy import SQLAlchemy
from sqlalchemy.orm import Mapped, mapped_column
from sqlalchemy.orm import DeclarativeBase
from werkzeug.middleware.proxy_fix import ProxyFix
from werkzeug.serving import is_running_from_reloader

import v0
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


@app.post("/v1/setting/<path:theme>")
def v1_get_setting(theme):
    if theme == PREGEN_THEME:
        setting_desc = ai.get_test_str("hk.txt")
    else:
        setting_desc = ai.gen_setting_desc(theme)
    return jsonify(setting_desc)


@app.post("/v1/craft")
def v1_craft():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    items = flask.request.get_json()["items"]
    item1 = flask.request.get_json()["item1"]
    item2 = flask.request.get_json()["item2"]
    if theme == PREGEN_THEME:
        return {
            "name": "PREGEN",
            "level": item1["level"] + 1,
            "type": item1["type"],
            "description": "who knows",
            "kind": item1["kind"],
        }
    else:
        new_item = ai.craft(theme, setting_desc, items, item1, item2)
        logging.info(json.dumps(new_item))
        return new_item


@app.post("/v1/areas")
def v1_get_areas():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    if theme == PREGEN_THEME:
        areas = ai.get_test_json("hk_areas.json")
    else:
        areas = ai.gen_areas(theme, setting_desc)
    logging.info(json.dumps(areas))
    return areas


@app.post("/v1/boss")
def v1_get_boss():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    if theme == PREGEN_THEME:
        boss = ai.get_test_json("hk_boss.json")
    else:
        boss = ai.gen_boss(theme, setting_desc)
    logging.info(json.dumps(boss))
    return boss


@app.post("/v1/monsters")
def v1_monsters():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    names = flask.request.get_json()["names"]
    if theme == PREGEN_THEME:
        monsters = ai.get_test_json("hk_monsters.json")
    else:
        monsters = ai.gen_monsters(theme, setting_desc, names)
    logging.info(json.dumps(monsters))
    return monsters


@app.post("/v1/items")
def v1_items():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    names = flask.request.get_json()["names"]
    if theme == PREGEN_THEME:
        items = ai.get_test_json("hk_items.json")
    else:
        items = ai.gen_items(theme, setting_desc, names)
    logging.info(json.dumps(items))
    return items


@app.post("/v1/anything")
def v1_anything():
    game_state = flask.request.get_json()["state"]
    game_state = game_types.GameState(**game_state)
    ask = flask.request.get_json()["ask"]
    if game_state.theme == PREGEN_THEME:
        return ai.get_test_json("hk.json")
    else:
        ai.gen_anything(ask, game_state)
        return game_state.model_dump_json()


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
