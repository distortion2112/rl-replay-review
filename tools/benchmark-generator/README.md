# Benchmark generator

Collects ranked-doubles match stats from [ballchasing.com](https://ballchasing.com) into a local SQLite store. Rows are one per player per game, tagged with **that player's own rank**. These rows are the source data for the per-rank `benchmarks.json` the app's coaching features compare against (#5, #8).

Python 3.12+ (for `statistics.correlation(method="ranked")`), standard library only.

## Setup

The API key comes from `BALLCHASING_API_KEY`, or from the key you saved in the app. The tool reads your ballchasing tier from `GET /api/` and matches that tier's published rate limits:

| Tier | Match stats (`GET /replays/{id}`) | List (`GET /replays`) |
|---|---|---|
| regular | 2/s, 1,000/h | 2/s, 500/h |
| gold | 2/s, 2,000/h | 2/s, 1,000/h |
| diamond | 4/s, 5,000/h | 4/s, 2,000/h |
| champion | 8/s | 8/s |
| gc | 16/s | 16/s |

Data lives in `data/collector.sqlite` (gitignored). Every command is resumable. Re-running skips what is already queued or fetched.

## Commands

```bash
python3 collector.py status                                   # tier, current season, what's stored
python3 collector.py collect --season f22 --per-tier 200      # queue + fetch per-tier games
python3 collector.py collect --season f22 --tiers gold-2 --per-tier 20 --limit 10   # small test
python3 collector.py anchor-test --season f22                 # validate anchor labeling (below)
python3 collector.py export-rows --season f22 --out rows.csv --ranked-only
python3 collector.py prune --dry-run                          # apply the retention rule
python3 benchmarks.py --season f22 --out data/benchmarks.json # build benchmarks (repeat --season to pool)
python3 -m unittest test_collector.py
```

`collect` lists games where every ranked player is in the target tier (`min-rank = max-rank = tier`). It spreads sampling across `--windows` date ranges in the season, so the end-of-season state doesn't dominate. API cost is about 1 list call per tier per window plus 1 call per game.

## Ranks

- **Rank scale:** `rank_steps` puts each player on one division-level scale: `(tier - 1) * 4 + (division - 1)`. Bronze 1 Div 1 is 0, so 1 tier = 4 steps.
- **Rank names:** `rank_id` is normalized from the tier number, because ballchasing reports GC1 as both `grand-champion` and `grand-champion-1`.
- **Unranked players:** some players in rank-filtered games have no rank (most often in SSL lobbies). Their rows have an empty `rank_id` and are excluded by `--ranked-only`.

## Ranks stopped in June 2026, and the anchor method

ballchasing stopped attaching ranks to ranked replays around **2026-06-05** (mid season f22). f23 has none, and f24 has a handful. See #5 for the survey. Current seasons need another rank source, so the plan is **anchors**:

1. Pick a few players per tier whose rank is known from an external source.
2. Fetch their games from the same period. Matchmaking keeps teammates and opponents near the anchor's MMR, so those lobbies are labeled with the anchor's rank.

`anchor-test` checks this on a season that *has* true per-player ranks (f22):

1. Pick `--anchors-per-tier` players per tier from the collected games.
2. Fetch up to `--games-per-anchor` of each anchor's games within `--window-days` of the game where their rank was observed.
3. Report how far lobby-mates' true ranks are from the anchor's reference rank, overall and per tier. It also reports how far the anchor's own rank drifts over the window.

## benchmarks.json

`benchmarks.py` builds the file the app loads (#8 has the full schema):

- **Rows:** only ranked players present for at least 90% of the game.
- **Units:** totals such as goals, boost collected, distance and demos are converted to **per minute** of game time, so overtime games don't inflate them. Percentages, averages and rates (`bpm`, `bcpm`, shooting %) are kept as they are. Each stat's `unit` records which.
- **Dropped stats:** `time_*` stats are dropped when a matching `percent_*` stat exists.
- **Per tier:** `sample_size`, `games`, `low_sample` (fewer than 30 rows), and per stat: `n`, `mean`, `stddev`, `p5`–`p95`.
- **Per stat:** `rank_signal` is the Spearman correlation with rank across all tiers. `direction` is whether the stat rises or falls at higher ranks, and `weight` is |ρ|. Coaching uses it to prioritize gaps that actually separate ranks.

## Manual anchors for current seasons (`anchors.py`)

There's no working rank API yet, so current-season anchors are looked up by hand on tracker.gg:

```bash
python3 anchors.py validate --season f22          # how good the stat-based tier guess is
python3 anchors.py candidates --season f24 --out data/anchor_candidates_f24.csv
# fill in rank_2v2 (+ division, checked_on) for each row from its tracker_url
python3 anchors.py import data/anchor_candidates_f24.csv
```

- **`candidates`.** Pools one list page per day over `--days`. It keeps frequent uploaders, so each lookup labels many games, and fetches a few of their games. It then spreads the list across tiers from `--min-tier` (default Diamond 1) using a rough tier guess: the weighted median of where the player's top rank-signal stats sit along the tier medians.
- **The guess only spreads the list.** It is never used as a label. On f22 it is unbiased per game but noisy: about 57% within one tier and 71% within two, from Diamond up.
- **`import`.** Accepts ranks typed as `diamond-2`, `Diamond 2`, `Diamond II`, `D2`, `GC1`, `SSL`. Rows without a rank are skipped. Anchors are stored with `rank_source = 'manual'` and `as_of = checked_on`.
- **Diamond and up only.** The f22 anchor test showed lobby labels from anchors are reliable from about Diamond up, and not in Bronze or Silver.

## Retention

The skill ceiling moves quickly, so `prune` keeps only:

- the current season *n* and *n − 1*
- the newest season with rank labels (currently f22), as validation and fallback data until a newer labeled season exists

`--current f24` skips season detection, and `--dry-run` shows what would be dropped.

## Row schema (`export-rows`)

| Column | Meaning |
|---|---|
| `replay_id`, `season`, `date` | Game identity |
| `duration`, `overtime` | Game length in seconds, and whether it went to overtime |
| `team`, `won`, `score_diff` | Context, so game state isn't read as playstyle |
| `player`, `name` | `<platform>:<id>`, and display name |
| `rank_id`, `rank_steps` | That player's own rank, empty if unknown |
| `teammate_rank_steps` | Teammate's rank on the same scale |
| `full_game` | Present for at least 90% of the game |
| `<group>_<stat>` | Every numeric ballchasing stat (`core`, `boost`, `movement`, `positioning`, `demo`) |
