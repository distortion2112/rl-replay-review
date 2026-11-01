# RL Replay Review - Development Guide

> Single source of truth for how work happens on this project.
> **All agents and contributors must read this before writing code.**

---

## What This Project Is

A Tauri v2 desktop app for reviewing Rocket League replays. Rust backend parses `.replay` files (via boxcars) and fetches stats from the ballchasing.com API. React/TypeScript frontend renders field visualizations, charts, and playback controls.

## Stack

| Layer | Tech |
|-------|------|
| Desktop shell | Tauri v2 |
| Backend | Rust (2024 edition) |
| Replay parsing | boxcars 0.13 |
| External API | ballchasing.com (stats + replay download) |
| Frontend | React 19, TypeScript, Vite |
| Styling | Tailwind CSS v4 |
| State | Zustand |
| Charts | Recharts |
| Icons | Lucide React |

## Architecture

```
src-tauri/src/
  commands.rs          <- Tauri command handlers (bridge to frontend)
  parser/
    replay_parser.rs   <- Two-pass boxcars parser (discover actors, then extract frames)
    frame_data.rs      <- Frame-level data structures
  api/
    ballchasing.rs     <- ballchasing.com API client
    models.rs          <- API response types
  analysis/
    boost.rs           <- Boost usage analysis
    rotation.rs        <- Rotation pattern analysis

src/
  App.tsx              <- Main app shell, tab routing
  components/
    field/             <- Canvas field renderer, heatmaps, trails
    charts/            <- Boost and movement charts
    stats/             <- Analysis panels, player comparison
    input/             <- API key setup, replay input, settings
    timeline/          <- Playback controls, scrubber, goal replays
  hooks/               <- useReplay, usePlayback, useKeyboard
  lib/                 <- Field math, heatmap logic, trail logic, Tauri command wrappers
  types/               <- TypeScript type definitions (replay, frames, analysis)
```

### Key Design Decisions

- **Two-tier data model**: Tier 1 = aggregate stats from ballchasing API (fast). Tier 2 = frame-by-frame from boxcars (detailed).
- **Two-pass parser**: Pass 1 discovers actor relationships (car-to-player mapping). Pass 2 extracts frame data using those mappings.
- **Car-PRI linking**: Uses generic `ActiveActor` sniffing because `MatchCar` attribute is absent in newer replay formats.
- **Boost tracking**: Maps `boost_component` to car actors in the second pass, not via pickup detection.

---

## Change Control

### Branch Strategy

```
main                   <- stable, always builds, only merged via PR
feature/<slug>         <- new features (feature/speed-chart)
fix/<slug>             <- bug fixes (fix/boost-parse-crash)
chore/<slug>           <- tooling, config, refactors (chore/add-clippy)
```

- **Never commit directly to `main`.** All changes go through a PR.
- Branch from `main`. Keep branches short-lived and focused.
- One issue per branch. One branch per issue.

### Commit Messages

```
<type>: <short imperative description>

feat: add per-goal mini-replay viewer
fix: handle missing PRI for disconnected players
chore: configure clippy lints
refactor: extract boost timeline into separate component
test: add parser tests for season 14 replays
docs: update CLAUDE.md with new architecture notes
```

Types: `feat`, `fix`, `chore`, `refactor`, `test`, `docs`

Keep commits atomic — one logical change per commit. Don't bundle unrelated changes.

### Pull Requests

1. Push your branch, open a PR against `main`
2. PR title: `<type>: <description>` (same as commit convention)
3. PR body must include:
   - **What** changed and **why**
   - `Closes #N` to auto-close the linked issue
   - Screenshots/recordings for visual changes
4. `npm run build` and `cargo build` must pass before merging
5. Squash merge preferred — keeps `main` history clean

### Issue Workflow

- Every non-trivial change starts with a GitHub issue
- Issues should be clear enough that someone (human or agent) can pick them up cold
- Use labels to categorize (see Labels section)
- When starting work: assign yourself, create a branch
- When done: PR closes the issue via `Closes #N`

---

## Code Quality Rules

### Rust (src-tauri/)

- `cargo build` must succeed with no errors
- No `unwrap()` on user-facing paths — use `Result` and propagate errors to the frontend via Tauri's error handling
- `unwrap()` is acceptable in tests and provably-safe internal logic
- Keep Tauri commands thin — they should delegate to parser/api/analysis modules
- New modules get a `mod.rs` that re-exports public items

### TypeScript (src/)

- `tsc --noEmit` must pass (strict mode is already on)
- No `any` types — use `unknown` and narrow, or define proper types in `src/types/`
- Components go in `src/components/<area>/` — don't dump everything in one folder
- Shared logic goes in `src/lib/`, hooks in `src/hooks/`
- Zustand stores: one store per domain concern, not one god store

### General

- Don't add dependencies without justification. This is a desktop app — bundle size matters less, but complexity still matters.
- Don't refactor code unrelated to your current issue. File a separate issue if you spot something.
- Don't add speculative features. If it's not in an issue, it doesn't get built.

---

## Labels

| Label | Purpose |
|-------|---------|
| `priority:high` | Critical path or blocking |
| `priority:medium` | Important, not blocking |
| `priority:low` | Nice to have |
| `area:parser` | Rust replay parser |
| `area:api` | ballchasing.com API integration |
| `area:frontend` | React UI components |
| `area:field-viz` | Field renderer, heatmaps, trails |
| `area:analysis` | Boost/rotation/positioning analysis |
| `area:playback` | Timeline, scrubber, frame playback |
| `area:infra` | Build, CI, Tauri config, tooling |
| `bug` | Something is broken |
| `enhancement` | New feature or improvement |
| `tech-debt` | Cleanup, refactoring, test coverage |

---

## Agent Guidelines

When working as an AI agent on this project:

1. **Read this file first.** Every session.
2. **Check open issues** before starting work. Don't invent work — pick up tracked issues.
3. **Create an issue first** if you identify new work that needs doing. Then branch, implement, PR.
4. **Don't make sweeping changes.** Small, focused PRs. If a change touches more than ~5 files across unrelated areas, break it up.
5. **Don't "improve" code you're not working on.** No drive-by refactors, no adding comments to untouched files, no reformatting.
6. **Test your changes.** At minimum: `cargo build` for Rust, `npm run build` for TypeScript. Run the app if you can.
7. **Update this file** if you make architectural decisions. Add them to the Key Design Decisions section.
8. **Coordinate via issues.** If another agent is working on something, don't start a conflicting branch. Check what's assigned.

---

## Known Issues

Tracked in GitHub Issues. Key ones:
- Disconnected players (e.g., "Heater") missing from frame data — no PRI found
- Boost pad pickup detection is proximity-based, not actor-state-based
- No tests exist yet (Rust or TypeScript)
- No CI/CD pipeline
- WSL2 EGL warnings are cosmetic (no GPU passthrough) — not a bug

---

## Local Development

```bash
# Frontend dev server (hot reload)
npm run dev

# Tauri dev (launches desktop app with hot reload)
npm run tauri dev

# Build checks
npm run build          # TypeScript check + Vite build
cargo build            # Rust build (from src-tauri/)
cargo clippy           # Rust lints (from src-tauri/)
```
