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

import ai


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


@app.post("/setting/<theme>")
def get_setting(theme):
    if theme == PREGEN_THEME:
        setting_desc = ai.get_test_str("hk.txt")
    else:
        setting_desc = ai.gen_setting_desc(theme)
    return jsonify(setting_desc)


@app.post("/areas")
def get_areas():
    theme = flask.request.json["theme"]
    setting_desc = flask.request.json["setting"]
    if theme == PREGEN_THEME:
        areas = ai.get_test_json("hk_areas.json")
    else:
        areas = ai.gen_areas(theme, setting_desc)
    logging.info(json.dumps(areas))
    return areas


@app.post("/monsters")
def monsters():
    theme = flask.request.json["theme"]
    setting_desc = flask.request.json["setting"]
    areas = flask.request.json["areas"]
    if theme == PREGEN_THEME:
        monsters = ai.get_test_json("hk_monsters.json")
    else:
        monsters = ai.gen_monsters(theme, setting_desc, areas)
    logging.info(json.dumps(monsters))
    return monsters


@app.post("/items")
def items():
    theme = flask.request.json["theme"]
    setting_desc = flask.request.json["setting"]
    areas = flask.request.json["areas"]
    if theme == PREGEN_THEME:
        items = ai.get_test_json("hk_items.json")
    else:
        items = ai.gen_items(theme, setting_desc, areas)
    logging.info(json.dumps(items))
    return items


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
