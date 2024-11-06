#!/usr/bin/env python
import json
import logging

import flask
from sqlalchemy.orm import DeclarativeBase
from flask import jsonify

import ai


PREGEN_THEME = "pregen"


class Base(DeclarativeBase):
    pass


def ai_error(e):
    return jsonify({"error": str(e)}), 500


def v1_get_setting(theme):
    if theme == PREGEN_THEME:
        setting_desc = ai.get_test_str("hk.txt")
    else:
        setting_desc = ai.gen_setting_desc(theme)
    return jsonify(setting_desc)


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


def v1_get_areas():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    if theme == PREGEN_THEME:
        areas = ai.get_test_json("hk_areas.json")
    else:
        areas = ai.gen_areas(theme, setting_desc)
    logging.info(json.dumps(areas))
    return areas


def v1_get_boss():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    if theme == PREGEN_THEME:
        boss = ai.get_test_json("hk_boss.json")
    else:
        boss = ai.gen_boss(theme, setting_desc)
    logging.info(json.dumps(boss))
    return boss


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
