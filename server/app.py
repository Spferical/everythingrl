#!/usr/bin/env python
import os
import json
import structlog
from typing import Any
import collections

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

LOGLEVEL = os.environ.get("EVERYTHINGRL_LOGLEVEL", "debug")
structlog.configure(wrapper_class=structlog.make_filtering_bound_logger(LOGLEVEL))
LOG = structlog.get_logger()
app = Flask(__name__)

# needed for running under reverse proxy
app.wsgi_app = ProxyFix(app.wsgi_app, x_proto=1, x_host=1)

app.config["SQLALCHEMY_DATABASE_URI"] = os.environ.get(
    "DATABASE_URI", "sqlite:///db.sqlite"
)

PREGEN_THEME = "pregen"


def bind_request_details(_sender: Flask, **extras: dict[str, Any]) -> None:
    structlog.contextvars.clear_contextvars()


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
    structlog.contextvars.bind_contextvars(theme=game_state.theme)
    LOG.info("/v1/actions generating from state", game_state=game_state)

    actions = ai.gen_actions(ask, game_state)

    def generate():
        raised_exception = False
        action_stats = collections.defaultdict(int)
        try:
            for action in actions:
                for key in action.model_dump(exclude_defaults=True):
                    action_stats[key] += 1
                yield action.model_dump_json(exclude_defaults=True)
                yield "\n"
        except Exception as e:
            # We may fail at any point during streaming, so we can't rely on
            # setting HTTP status.
            yield json.dumps({"error": str(e)})
            yield "\n"
            raised_exception = True
        action_stats = dict(sorted(action_stats.items()))
        status = "success"
        if raised_exception or list(action_stats.keys()) == "error":
            status = "failed"
        elif "error" in action_stats:
            status = "partial_error"
        LOG.info("/v1/actions done", status=status, **action_stats)

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
    flask.request_started.connect(bind_request_details, app)
    app.run(debug=True)
