# RL Replay Review

A desktop app for Rocket League players who want to improve. Analyze replays from [ballchasing.com](https://ballchasing.com) with frame-level playback, heatmaps, and coaching insights.

Built with Tauri (Rust) + React + TypeScript.

## Features

### Replay Analysis
- **Stat Breakdown** - Per-player core stats, boost usage, movement, positioning, and demos pulled from the ballchasing.com API
- **Frame-Level Playback** - Download and parse `.replay` files for frame-by-frame player/ball positions with boost tracking
- **Live Field View** - Real-time 2D field rendering showing all players and the ball, synced to a timeline scrubber
- **Heatmaps** - Player, team, and ball position heatmaps generated from frame data
- **Movement Trails** - Configurable trail visualization showing player paths with fade effect
- **Player Comparison** - Side-by-side metric comparison across all players in a match

### Automated Insights
- **Rotation Detection** - Identifies ball-chasing tendencies using offensive positioning, ball distance, and closest-to-ball time
- **Boost Efficiency Scoring** - Rates boost management (0-100) based on small pad usage, overfill, zero-boost time, and stealing

### Charts & Visualization
- Boost amount timeline across the full match
- Movement speed categories over time
- Goal event markers on the timeline with replay navigation

## Screenshots

<!-- TODO: Add screenshots -->

## Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Node.js](https://nodejs.org/) (v18+)
- A [ballchasing.com](https://ballchasing.com) API key (free tier works)

### Install & Run
```bash
# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

On first launch, the app will prompt you to enter your ballchasing.com API key. Get one at [ballchasing.com/upload](https://ballchasing.com/upload) after logging in.

### Usage
1. Paste a ballchasing.com replay URL or ID
2. View stats, analysis, and player comparison
3. Click "Load Frame Data" for live field playback, heatmaps, and trails

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 19, TypeScript, Tailwind CSS, Recharts, Zustand |
| Backend | Rust, Tauri 2, Boxcars (replay parser), Reqwest |
| Persistence | tauri-plugin-store |
| API | ballchasing.com |

## Roadmap

The next major feature set introduces **user profiling** and **rank-based coaching** — comparing your play to players at your rank (and above) to generate actionable coaching insights.

### M1: Foundation
> API infrastructure to support profile and coaching features

- [ ] Replay search endpoint ([#1](../../issues/1))
- [ ] Request rate limiter with budget tracking ([#2](../../issues/2))
- [ ] Persistent replay data cache ([#3](../../issues/3))

### M2: User Profile
> Identify yourself, auto-detect your rank and playstyle from replay history

- [ ] Profile data models and storage ([#4](../../issues/4))
- [ ] Player aggregate stats from replay history ([#6](../../issues/6))
- [ ] Profile Zustand store ([#7](../../issues/7))
- [ ] Playstyle computation — 5-axis profiling with archetypes ([#9](../../issues/9))
- [ ] Profile setup wizard UI ([#13](../../issues/13))
- [ ] Playstyle radar chart ([#14](../../issues/14))

### M3: Benchmarks
> Pre-computed rank benchmark data for every tier in 2v2

- [ ] Generate benchmark dataset — all ranks, ranked-doubles ([#5](../../issues/5))
- [ ] Benchmark loader and storage ([#8](../../issues/8))
- [ ] Optional benchmark refresh for patron-tier users ([#19](../../issues/19))
- [ ] Benchmark auto-update from remote ([#20](../../issues/20))

### M4: Coaching Engine
> Compare your stats to your rank, identify strengths and weaknesses, get actionable advice

- [ ] Core comparison engine ([#10](../../issues/10))
- [ ] Coaching text generation with templates ([#11](../../issues/11))
- [ ] Coaching Zustand store ([#12](../../issues/12))
- [ ] Coaching dashboard UI ([#15](../../issues/15))
- [ ] Rank comparison bar charts ([#16](../../issues/16))
- [ ] App navigation — Replay Review / Coaching tabs ([#17](../../issues/17))
- [ ] Per-replay coaching overlay ([#18](../../issues/18))

### Playstyle Axes

The profiling system classifies players across five axes (0-100 each):

| Axis | What it measures |
|------|-----------------|
| Aggression | Offensive positioning, shots, demos, distance to ball |
| Mechanical | Aerial time, speed, shooting accuracy, powerslides |
| Boost Discipline | Small pad usage, overfill avoidance, zero-boost time |
| Team Play | Assists, saves, teammate spacing, defensive coverage |
| Aerial Tendency | Time in air vs on ground |

These combine into an **archetype** label (e.g., "Aggressive Striker", "Efficient Rotator", "Defensive Anchor") displayed alongside a radar chart.

## Architecture

```
src/                          # React frontend
  components/
    field/                    # Field rendering, heatmaps, trails
    stats/                    # Stat panels, player comparison
    charts/                   # Boost/movement charts
    timeline/                 # Playback controls, scrubber, goal replays
    input/                    # API key setup, replay input, settings
  hooks/                      # Zustand stores (useReplay, usePlayback)
  types/                      # TypeScript interfaces (replay, frames, analysis)
  lib/                        # Tauri command wrappers, field math, trails

src-tauri/src/                # Rust backend
  api/                        # ballchasing.com client and models
  analysis/                   # Rotation and boost analysis engines
  parser/                     # .replay file parsing via Boxcars
```

## API Budget

The app is designed to be efficient with ballchasing.com API calls. Free-tier users get ~70 calls/week:

| Action | API Calls |
|--------|-----------|
| Load a replay | 1-2 |
| Profile setup (one-time) | ~11 |
| Coaching report | 0 (uses cached data + bundled benchmarks) |

## Contributing

Check the [issues](../../issues) for the current backlog. Issues are organized by milestone and labeled by area (`backend`, `frontend`, `data`) and priority (`P0`, `P1`, `P2`). Each issue includes implementation details, acceptance criteria, and dependency notes for agent handoff.

## License

<!-- TODO: Choose license -->
