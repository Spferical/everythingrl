FROM python:3.11-alpine

RUN apk update && apk add python3-dev gcc libc-dev g++ geos-dev

WORKDIR /docker-flask/server

RUN ["pip3", "install", "pipenv"]

COPY server/Pipfile .
COPY server/Pipfile.lock .

RUN ["pipenv", "install"]

WORKDIR /docker-flask

COPY . .

WORKDIR /docker-flask/server/

EXPOSE 5000

ENV FLASK_ENV=production
CMD pipenv run gunicorn -b 0.0.0.0:5000 app:app
