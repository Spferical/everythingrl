FROM python:3.11-alpine

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
CMD pipenv run flask run --host 0.0.0.0
