#!/usr/bin/env python
import os

from flask import Flask, send_from_directory
from flask_sqlalchemy import SQLAlchemy
from sqlalchemy.orm import DeclarativeBase
from werkzeug.middleware.proxy_fix import ProxyFix
from werkzeug.serving import is_running_from_reloader

import ai


app = Flask(__name__)

# needed for running under reverse proxy
app.wsgi_app = ProxyFix(app.wsgi_app, x_proto=1, x_host=1)

app.config["SQLALCHEMY_DATABASE_URI"] = os.environ.get(
    "DATABASE_URI", "sqlite:///db.sqlite"
)


class Base(DeclarativeBase):
    pass


@app.route("/monsters/<theme>/<int:level>")
def monsters(theme, level):
    monsters = ai.gen_monster(theme, level)
    print(monsters)
    return monsters

@app.route("/")
def root():
    return send_from_directory("../dist", "index.html")

@app.route("/<path:path>")
def serve_static(path):
    return send_from_directory("../dist", path)


print(f"Using database {app.config['SQLALCHEMY_DATABASE_URI']}")
db = SQLAlchemy(model_class=Base)

if __name__ == "__main__":
    db.init_app(app)
    app.run(debug=True)
