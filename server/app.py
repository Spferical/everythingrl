#!/usr/bin/env python
import os

from flask import Flask, send_from_directory
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


class Base(DeclarativeBase):
    pass


@app.route("/test")
def root():
    return "hi"


@app.route("/<path:path>")
def serve_static(path):
    return send_from_directory("../dist", path)


def run_server():
    print(f"Using database {app.config['SQLALCHEMY_DATABASE_URI']}")
    db = SQLAlchemy(model_class=Base)
    db.init_app(app)

    app.run(debug=True)
