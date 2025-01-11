FROM ghcr.io/astral-sh/uv:python3.13-bookworm-slim

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

ENV PATH="/app/server/.venv/bin:$PATH"

EXPOSE 5000

ENV FLASK_ENV=production
CMD gunicorn --workers 5 -b 0.0.0.0:5000 app:app
