FROM python:3.11-slim

RUN apt-get update && apt-get install -y \
    python3-dev libgeos-dev build-essential git && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /docker-flask/server

RUN pip3 install pipenv --no-cache-dir

COPY server/Pipfile .
COPY server/Pipfile.lock .

RUN PIP_NO_CACHE_DIR=off pipenv install --deploy --system

WORKDIR /docker-flask

COPY . .

WORKDIR /docker-flask/server/

EXPOSE 5000

ENV FLASK_ENV=production
CMD pipenv run gunicorn -b 0.0.0.0:5000 app:app
