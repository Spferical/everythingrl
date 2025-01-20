#!/usr/bin/env python
import json
import structlog

import flask
from flask import jsonify

from . import ai


PREGEN_THEME = "pregen"
LOG = structlog.get_logger()


def get_setting(theme):
    if theme == PREGEN_THEME:
        setting_desc = ai.get_test_str("hk.txt")
    else:
        setting_desc = ai.gen_setting_desc(theme)
    return jsonify(setting_desc)


def craft():
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
        LOG.info(json.dumps(new_item))
        return new_item


def get_areas():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    if theme == PREGEN_THEME:
        areas = ai.get_test_json("hk_areas.json")
    else:
        areas = ai.gen_areas(theme, setting_desc)
    LOG.info(json.dumps(areas))
    return areas


def get_boss():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    if theme == PREGEN_THEME:
        boss = ai.get_test_json("hk_boss.json")
    else:
        boss = ai.gen_boss(theme, setting_desc)
    LOG.info(json.dumps(boss))
    return boss


def monsters():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    names = flask.request.get_json()["names"]
    if theme == PREGEN_THEME:
        monsters = ai.get_test_json("hk_monsters.json")
    else:
        monsters = ai.gen_monsters(theme, setting_desc, names)
    LOG.info(json.dumps(monsters))
    return monsters


def items():
    theme = flask.request.get_json()["theme"]
    setting_desc = flask.request.get_json()["setting"]
    names = flask.request.get_json()["names"]
    if theme == PREGEN_THEME:
        items = ai.get_test_json("hk_items.json")
    else:
        items = ai.gen_items(theme, setting_desc, names)
    LOG.info(json.dumps(items))
    return items
