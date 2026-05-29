# kinketsu

A self-hosted subscription manager that connects to your **inbox** — not your bank — to discover and track every recurring charge across services and currencies.

> "kinketsu" is the Japanese word for being broke. A wink at where subscriptions take us.

## Why kinketsu

Today's options force a trade-off:

- **Money Forward ME / Rocket Money** — discover subscriptions through bank/card aggregation. Recurring fee, generic per-service metadata, regional lock-in (Rocket Money is US-only; Money Forward pulls Japanese banks but barely surfaces rich subscription identity).
- **Bobby / Subby / Wallos** — beautiful subscription-first UIs, but every entry is manual. No discovery.

**kinketsu treats your email inbox as the universal subscription registry.** Netflix, Adobe, OpenAI, Spotify, GitHub, U-NEXT, d Magazine, Nikkei — virtually every subscription sends confirmation and renewal emails. A user-selected LLM (Claude / OpenAI / Gemini / Ollama / LM Studio) reads them and proposes subscription entries; you confirm or reject in a review queue. No bank API, no electronic-settlement-agent license, no recurring vendor lock-in.

## Status

Pre-alpha — workspace and UI shell scaffolded, v0.1 data layer in progress. Not yet usable end-to-end.

## Architecture

```
kinketsu/
├── Cargo.toml                # Rust workspace
├── crates/
│   ├── core/                 # Shared lib: models, DB, LLM, parsers, currency, iCal
│   │   └── migrations/       # SQLite schema migrations
│   └── server/               # Axum HTTP server (self-host Docker, future SaaS)
└── app/                      # SvelteKit frontend + Tauri 2 shell
    ├── src/                  # Svelte UI
    └── src-tauri/            # Native shell (macOS desktop + Android), embeds crates/core
```

| Layer | Stack | Why |
|---|---|---|
| Core logic | Rust library (`kinketsu-core`) | Models, persistence, LLM routing, parsing live in one place; reused by every target |
| Persistence | SQLite via `sqlx` | Local-first, single file, WAL mode |
| LLM | Enum-dispatched providers: Claude / OpenAI / Gemini / Ollama / LM Studio | User picks provider + supplies key in settings; cloud and local both supported |
| Server | Axum 0.8 (`kinketsu-server`) | Self-host Docker image target; SaaS-ready when needed |
| Desktop / Mobile | Tauri 2.0 | Single native shell over the same Rust core, macOS + Android |
| Web | SvelteKit + Tailwind 4 | Shared Svelte codebase across all shells; static SPA build |
| Design | Glassmorphism on animated blob background | visionOS / iOS 18 era; subtle palette in `oklch()` |

## v0.1 scope

- [x] Workspace + UI shell scaffolding
- [ ] Manual subscription CRUD (in progress)
- [ ] Categories + payment methods
- [ ] Multi-currency with daily exchange-rate cache
- [ ] Renewal reminder notifications
- [ ] iCalendar (`.ics`) export of renewal dates
- [ ] Gmail OAuth + LLM-based receipt extraction
- [ ] PayPal OAuth (補完 for PayPal-routed overseas subs)
- [ ] Past-scan UI: year / month multi-select to bound LLM cost
- [ ] Settings: LLM provider selection + API keys

## Out of scope (deliberately)

- **Bank / credit-card aggregation** — requires Japan's electronic settlement agent license (*denshi kessai-tō dairi-gyō*); non-starter for an individual project.
- **Crypto wallet address monitoring** — a different problem domain (on-chain flows are not subscriptions).
- **App Store / Play Store subscription sync** — no usable public API for third-party purchases.

## Quick start

Prerequisites:

- Rust ≥ 1.93 (`rustup install stable`)
- Node ≥ 22, pnpm ≥ 10
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Android target: Android Studio + NDK (only needed for Android builds)

```sh
# install all frontend deps (pnpm workspace at repo root)
pnpm install
```

All `pnpm` commands below can be run from the repo root; they proxy to `app/` via the workspace.

### Run the desktop app (hot reload)

```sh
pnpm dev
# equivalent to: cd app && pnpm tauri dev
```

### Run the standalone server (self-host target)

```sh
cargo run -p kinketsu-server
# Listens on $KINKETSU_BIND (default 0.0.0.0:3000)
#   GET /health → {"status":"ok","service":"kinketsu-server"}
```

### Frontend-only dev (browser, no Tauri shell)

```sh
pnpm web:dev
# http://localhost:5173 — UI renders, but Tauri commands will not resolve
```

### Type-check and build

```sh
cargo check --workspace   # check all Rust crates
pnpm check                # SvelteKit + TypeScript
pnpm web:build            # static SvelteKit bundle into app/build/
pnpm build                # full Tauri release bundle
```

## Privacy

kinketsu never stores email bodies. The Gmail integration fetches receipts, extracts structured fields via the user-configured LLM, and persists only the parsed result plus the upstream `message-id` (used to skip already-seen messages). Choose Ollama or LM Studio for fully local processing if even structured extraction shouldn't leave your machine.

## License

[AGPL-3.0-or-later](./LICENSE). Forks, hosted versions, and derivative SaaS must publish source.
