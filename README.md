# nERP

**nERP** ("not an ERP") is an internal operating system for the everyday work life of a
company's employees. It targets **small companies** — where everyone wears many hats — so
it favors generalist, low-friction workflows over heavyweight, role-siloed ERP modules.

> **Status:** early prototype. The foundation (server, UI toolkit, database, build &
> deploy) is in place; domain features are next.

## Tech stack

- **Backend:** [Rust](https://www.rust-lang.org) + [loco](https://loco.rs) (on axum),
  server-side rendered with [Tera](https://keats.github.io/tera) templates.
- **Frontend:** [htmx](https://htmx.org) + the [Tabler](https://tabler.io) UI kit
  (+ Tabler Icons), compiled from SCSS/JS by [Vite](https://vite.dev). No SPA —
  hypermedia over the wire.
- **Database:** PostgreSQL via [SeaORM](https://www.sea-ql.org/SeaORM) + loco migrations.
- **Tooling:** cargo · npm (Node) · Docker.

## Prerequisites

- Rust (stable) + Cargo
- Node 20+ and npm — e.g. via [nvm](https://github.com/nvm-sh/nvm)
- Docker (for Postgres and the production image)

## Quick start (Docker)

Run the whole stack — Postgres **and** the app — in one command:

```sh
make docker-up      # build + start; app on http://localhost:5150
make docker-logs    # follow logs
make docker-down    # stop
```

## Local development

```sh
make setup          # install npm deps + fetch cargo crates (run once)
make dev            # Postgres (Docker) + Vite asset watch + the loco server
```

`make dev` serves http://localhost:5150 — the loco server hot-reloads Tera templates and
Vite rebuilds CSS/JS on change. Stop with `Ctrl-C`.

Other tasks (`make help` lists them all):

```sh
make build          # build frontend assets (Vite) + compile the app (cargo)
make test           # run the Rust test suite
make clean          # remove build artifacts (target/ + assets/static/dist/)
```

> Frontend source lives in [frontend/](frontend/) (`main.js`, `main.scss`) and compiles to
> `assets/static/dist/` (gitignored), served at `/static/dist/`. Theme Tabler by overriding
> Sass variables in `frontend/main.scss` — never hand-edit `assets/static/dist/`.

## Project layout

```
frontend/          Vite source (main.js, main.scss) → assets/static/dist
assets/
  views/           Tera templates (layout/base.html, home/…)
  static/          served at /static (dist/ is the Vite build output)
  i18n/            Fluent locale files
src/
  controllers/     auth (JSON API) + home (server-rendered pages)
  models/ views/   SeaORM models, response types, initializers, workers, …
config/            development / test / production YAML
migration/         SeaORM migrations
docker/            Dockerfile (prod image) + compose.yaml (local stack)
```

## Routes

- `GET /` — home page (Tabler + htmx).
- `GET /htmx-probe` — demo HTML fragment swapped in by htmx.
- `POST|GET /api/auth/*` — JWT auth (register, login, verify, …).

## Production

A single, self-contained image — multi-stage build (Node compiles assets → Rust compiles
the binary → slim runtime with **no** Node/cargo/node_modules):

```sh
make docker-build               # → nerp:latest
docker run -p 5150:5150 \
  -e LOCO_ENV=production \
  -e DATABASE_URL=postgres://user:pass@host:5432/nerp \
  nerp:latest
```

loco loads `config/<LOCO_ENV>.yaml` (default `development`); supply `config/production.yaml`
(gitignored — it holds secrets) and inject values such as `DATABASE_URL` via environment at
deploy time. See [config/development.yaml](config/development.yaml) for the available knobs.
