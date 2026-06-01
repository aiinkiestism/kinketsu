# kinketsu

A self-hosted subscription manager that connects to your **inbox** — not your bank — to discover and track every recurring charge across services and currencies.

> "kinketsu" is the Japanese word for being broke. A wink at where subscriptions take us.

## Why kinketsu

Today's options force a trade-off:

- **Money Forward ME / Rocket Money** — discover subscriptions through bank/card aggregation. Recurring fee, generic per-service metadata, regional lock-in (Rocket Money is US-only; Money Forward pulls Japanese banks but barely surfaces rich subscription identity).
- **Bobby / Subby / Wallos** — beautiful subscription-first UIs, but every entry is manual. No discovery.

**kinketsu treats your email inbox as the universal subscription registry.** Netflix, Adobe, OpenAI, Spotify, GitHub, U-NEXT, d Magazine, Nikkei — virtually every subscription sends confirmation and renewal emails. A user-selected LLM (Claude / OpenAI / Gemini / Ollama / LM Studio) reads them and proposes subscription entries; you confirm or reject in a review queue. No bank API, no electronic-settlement-agent license, no recurring vendor lock-in.

## Status

v0.1 feature-complete plus a polish + infrastructure pass. All Rust crates
type-check and pass clippy + tests; the SvelteKit frontend type-checks and
builds. A live `pnpm tauri dev` run, a `pnpm tauri build` release bundle,
and `pnpm android:init` are the remaining manual smoke checks (each
requires local OS / SDK state outside of CI's reach).

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
| Server | Axum 0.8 (`kinketsu-server`) | Single-user self-host. Bearer-token auth; Docker image. |
| Type bindings | `specta` + `tauri-specta` (RC) | Tauri commands and the Rust model universe export typed TS wrappers to `app/src/lib/bindings.ts` on every dev build |
| i18n | English source dictionary + LLM-driven translation, cached per locale in SQLite | Adding a locale costs no translator: at first switch the chosen LLM translates the dictionary; subsequent sessions read from cache |
| Desktop / Mobile | Tauri 2.0 | Single native shell over the same Rust core, macOS + Android |
| Web | SvelteKit + Tailwind 4 | Shared Svelte codebase across all shells; static SPA build |
| Design | Glassmorphism on animated blob background | visionOS / iOS 18 era; subtle palette in `oklch()` |

## v0.1 scope

- [x] Workspace + UI shell scaffolding
- [x] Manual subscription CRUD
- [x] Categories + payment methods
- [x] Settings: LLM provider selection + API keys (Claude / OpenAI / Gemini / Ollama / LM Studio)
- [x] Multi-currency with daily exchange-rate cache (open.er-api.com, JPY-anchored conversion)
- [x] iCalendar (`.ics`) export of renewal dates
- [x] Renewal reminder notifications (system, 7-day window, daily background check)
- [x] Past-scan UI: year / month multi-select (`MonthRangeSelect`) driving Gmail scans
- [x] Inbox / review queue for detected subscriptions (Confirm / Reject)
- [x] Gmail OAuth + LLM-based receipt extraction (real Gmail API, paginated, dedupe)
- [x] PayPal OAuth flow (Log In with PayPal); personal-account scan via Gmail-parsed PayPal emails + CSV import (PayPal Transaction Search API is business-tier only)
- [x] Inline editing for subscriptions, payment methods, categories
- [x] Edit-before-confirm on detection events
- [x] Base currency derived from locale, overridable in Settings
- [x] Manual locale switcher in Settings; secret-input show/hide toggles; `MonthRangeSelect` presets
- [x] CSV bulk import (paste text → LLM identifies recurring rows → DetectionEvent queue)
- [x] `specta` + `tauri-specta` typed bindings exported on `pnpm tauri dev`
- [x] LLM-driven runtime translation (no shipped JA dictionary)
- [x] Single-user HTTP server with Bearer-token auth + Dockerfile
- [x] cargo test suite (15 unit + 8 integration) + GitHub Actions CI

## Out of scope (deliberately)

- **Bank / credit-card aggregation** — requires Japan's electronic settlement agent license (*denshi kessai-tō dairi-gyō*); non-starter for an individual project.
- **Crypto wallet address monitoring** — a different problem domain (on-chain flows are not subscriptions).
- **App Store / Play Store subscription sync** — no usable public API for third-party purchases.
- **PayPal Transaction Search API for personal accounts** — PayPal restricts that API to business-tier accounts. Personal users rely on Gmail-parsed PayPal email receipts and the CSV bulk import on the Scan page.

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

### Run on Android

Prerequisites: Android Studio with SDK + NDK + JDK 17. Set
`ANDROID_HOME` (e.g. `~/Library/Android/sdk`) and `NDK_HOME`
(e.g. `$ANDROID_HOME/ndk/<version>`).

```sh
# one-time scaffold (creates app/src-tauri/gen/android/)
pnpm android:init

# dev with hot reload on a connected device or emulator
pnpm android:dev

# release AAB / APK
pnpm android:build
```

### Run the standalone server (self-host target)

The server is single-user. All mutating routes sit behind a Bearer token —
the value of `KINKETSU_SECRET` — passed as `Authorization: Bearer <secret>`.

```sh
export KINKETSU_SECRET=$(openssl rand -hex 32)
export KINKETSU_DB=./kinketsu.db
cargo run -p kinketsu-server
# Listens on $KINKETSU_BIND (default 0.0.0.0:3000)
#   GET /health → {"status":"ok","service":"kinketsu-server"}  (no auth)
#   GET /subscriptions  (auth required)
#   POST /scan/text  (auth required)
#   …etc
```

#### Docker

```sh
docker build -t kinketsu-server .
docker run --rm \
  -e KINKETSU_SECRET=$(openssl rand -hex 32) \
  -p 3000:3000 \
  -v $(pwd)/data:/data \
  kinketsu-server
```

Mount `/data` to persist `kinketsu.db`. Put the API behind a reverse proxy
(Caddy / nginx / Traefik) for TLS.

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

### Tests + lints

```sh
cargo fmt --all -- --check
cargo clippy --workspace --exclude kinketsu-app --all-targets -- -D warnings
cargo test -p kinketsu-core --all-targets   # 15 unit + 8 integration
```

CI (`.github/workflows/ci.yml`) runs the Rust gate (fmt + clippy + test,
excluding the Tauri app because Linux runners don't carry webkit) plus
`pnpm check` and `pnpm web:build` on every push and pull request.

## Internationalization

English is the source of truth. The application bundles only the English
dictionary (`app/src/lib/i18n.svelte.ts`). When the user picks a non-English
locale — auto-detected from `navigator.language` or chosen in Settings —
kinketsu sends the dictionary to the configured LLM provider once, caches
the translations in SQLite (`settings.translations.<locale>`), and reads
from cache on subsequent loads. Newly added English keys auto-translate on
their next reference. Failures fall back silently to English.

Backend renewal notifications honour the same translation cache, so the
system notification arrives in the user's language too.

The product name **kinketsu** is never localized — it always appears as the
bare ASCII string.

## Privacy

kinketsu never stores email bodies. The Gmail integration fetches receipts,
extracts structured fields via the user-configured LLM, and persists only
the parsed result plus the upstream `message-id` (used to skip
already-seen messages). The LLM-driven UI translation sends each dictionary
key exactly once per locale and caches the result; pick Ollama or LM Studio
if even that traffic should stay on your machine. CSV bulk imports send the
text you paste to the configured LLM in a single round-trip — same
provider, same privacy posture.

## License

[AGPL-3.0-or-later](./LICENSE). Forks, hosted versions, and derivative SaaS must publish source.
