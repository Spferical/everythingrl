import os

from flask import Flask
from flask_sqlalchemy import SQLAlchemy
from sqlalchemy.orm import DeclarativeBase

from werkzeug.middleware.proxy_fix import ProxyFix
from werkzeug.serving import is_running_from_reloader


class Base(DeclarativeBase):
    pass


def main():
    app = Flask(__name__)

    # needed for running under reverse proxy
    app.wsgi_app = ProxyFix(app.wsgi_app, x_proto=1, x_host=1)

    app.config["SQLALCHEMY_DATABASE_URI"] = os.environ.get(
        "DATABASE_URI", "sqlite:///db.sqlite"
    )
    print(f"Using database {app.config['SQLALCHEMY_DATABASE_URI']}")

    db = SQLAlchemy(model_class=Base)
    db.init_app(app)

    app.run(debug=True)


if __name__ == "__main__":
    main()
