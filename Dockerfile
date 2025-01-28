FROM ghcr.io/astral-sh/uv:python3.12-bookworm-slim AS builder
ENV UV_COMPILE_BYTECODE=1 UV_LINK_MODE=copy

RUN apt-get update && apt-get install -y \
    python3-dev libgeos-dev build-essential git && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app/server

# Install dependencies
RUN --mount=type=cache,target=/root/.cache/uv \
    --mount=type=bind,source=server/uv.lock,target=uv.lock \
    --mount=type=bind,source=server/pyproject.toml,target=pyproject.toml \
    uv sync --frozen --no-install-project --no-dev

# Copy all data including the webclient build.
WORKDIR /app
COPY . .

# Install the project.
WORKDIR /app/server
RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --no-dev


FROM docker.io/python:3.12-slim-bookworm

RUN apt-get update && apt-get install -y \
    libgeos-dev && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app /app

WORKDIR /app/server
ENV PATH="/app/server/.venv/bin:$PATH"
ENV FLASK_ENV=production
EXPOSE 5000
CMD gunicorn --workers 5 -b 0.0.0.0:5000 app:app
