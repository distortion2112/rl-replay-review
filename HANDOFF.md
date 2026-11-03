# Handoff: picking up on a new machine (2026-11-03)

Temporary note for continuing work on another PC. **Delete this file before #58 merges to `main`.**

## Where things stand

| Branch | PR | State |
|---|---|---|
| `feature/coaching-screen` | #56 → `main` | Coaching tab: report, findings, playstyle card. CI green, mergeable. |
| `feature/coaching-site-search` | #58 → `feature/coaching-screen` | Stacked on #56. Pull coaching games from a ballchasing name search (`?player-name=…`); Player id + Player name fields with **Find games**; tick which accounts are you; extra accounts count as aliases. Closes #57. Tested by hand in `tauri dev`. |

Merge order: **#56 first**, then retarget #58 to `main` (GitHub may do it automatically) and merge.
Everything else (#50, #51, #53, #54, #55) is already merged.

## Set up the new machine

Prerequisites (versions this was last tested with): Node 26, Rust 1.98 (stable, 2024 edition), Python 3.14 (only for `tools/`, stdlib only), `gh` logged in as `distortion2112`. On macOS, Xcode command line tools; on Linux/WSL, the Tauri v2 system packages.

```bash
git clone https://github.com/distortion2112/rl-replay-review && cd rl-replay-review
git checkout feature/coaching-site-search      # contains #56 + #58
npm install
npm run tauri dev
```

- Re-enter the ballchasing API key in Settings. It lives in app data, not the repo.
- Set up the coaching profile again (player id and/or name, rank, games).
- Checks before a PR: `cargo test` and `cargo clippy` (in `src-tauri/`), `npx tsc --noEmit`, `npx vitest run`, `npm run build`.

### Benchmark raw data (only needed to rebuild benchmarks)

`tools/benchmark-generator/data/` is gitignored (about 19 MB, mostly `collector.sqlite`). It's attached to a **draft** release for a one-time copy:

```bash
gh release download tmp-benchmark-data -p benchmark-data.zip
unzip benchmark-data.zip -d tools/benchmark-generator && rm benchmark-data.zip
gh release delete tmp-benchmark-data --yes      # once copied
```

The app itself uses `src-tauri/resources/benchmarks.json`, which is committed.

The Python tools read the ballchasing key from `BALLCHASING_API_KEY`, or from the app's saved settings once you've entered it in the app. The account is on the **Diamond** patron tier; the collector detects the tier and paces itself. Never print the key.

Verified 2026-11-03 on a fresh clone of this branch with a clean build: `cargo test` (114 passed), `cargo clippy --all-targets -D warnings`, `tsc`, `vitest` (49 passed), `npm run build`, the release zip restore, `collector.py status`, and `test_collector` (20 passed).

## Unfinished work outside epic #59

**Benchmark anchors, round 2 (waiting on a manual step).** Current-season (f24) tiers are labeled by ranks looked up by hand on tracker.gg ("anchors"); 47 are imported. Diamond and SSL need more before they switch from f22 to f24. The 21-player list is in the restored data:

```bash
cd tools/benchmark-generator
python3 anchors.py lookup data/anchor_candidates_f24_round2.csv   # opens each tracker link; type the rank shown
python3 anchors.py import data/anchor_candidates_f24_round2.csv
python3 anchors.py collect --season f24                           # fetch their games (needs the API key)
python3 benchmarks.py --season f22 --anchor-season f24 --min-anchors 3 --out data/benchmarks.json
cp data/benchmarks.json ../../src-tauri/resources/benchmarks.json # then cargo test, and open a PR
```

Lookups are manual on purpose: automating tracker.gg means getting past Cloudflare, which breaks its terms. Bronze/Silver/Gold tiers are deferred (anchors aren't reliable there and there's no rank source yet). Retention: keep only seasons n and n−1 plus the newest season with ballchasing ranks (`collector.py prune`).

**#11, second half: the optional LLM narrator.** Templates (`coaching/text.rs`) and the verifier (`coaching/verify.rs`) are merged. The narrator would rewrite template text, and `verify(text, pack, true)` must pass or the app falls back to the templates.

## What's next: epic #59, situational coaching

Goal: coaching that names the moments that cost games (missed open nets, bad 50s, bad positioning, what happened just before each goal against), explains what each stat says about how you play and why it matters (raw numbers in a details pane as evidence), and shows the profile's playstyle in Replay Review, with how this game compares with your usual.

Suggested order:
1. Specs: **#60** cue definitions (starting thresholds are in the issue) and **#64** stat meanings. Foundation: **#66** goal attribution (`GoalEvent` has no scoring team or scorer yet) → **#67** event engine (possession, first/second/last man, geometry).
2. **#70** goals-against breakdown → **#72** Key moments panel on the loaded replay (no extra downloads, so it ships fastest).
3. **#75** insight-first findings + details pane, in parallel (needs only #64).
4. **#62** hand-labelled validation set, then detectors **#69** 50s, **#68** open nets, **#71** positioning.
5. **#61** frame data for the whole sample + **#63** rank norms → **#73** cues in the evidence pack → **#74** open a finding's clips.
6. **#65** → **#76** → **#77** playstyle in Replay Review (any time).

Research issues carry the `research` label. #31 is the overall architecture; #18 and #3 have comments pointing at the new issues.

## Things to know

- Parser touches come from ball speed jumps (> 250 uu/s with a car within 300 uu): soft touches are missed, and a 50 can't be told from a clean hit. #66 addresses this.
- Coaching findings are built from ballchasing aggregate stats per game. Nothing frame-level is in coaching yet. `ClipRef` in `coaching/models.rs` is the empty hook for it.
- The evidence pack is locked by a snapshot test. Bump `coaching::SCHEMA_VERSION` when changing it on purpose.
- Identity rule (see CLAUDE.md): a name never proves identity. The user ticks accounts, the typed id (or the most-played ticked id) is primary, and the rest are aliases.
- Use the GitHub API, not `gh pr merge`, when merging from a worktree.
