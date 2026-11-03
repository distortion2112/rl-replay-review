#!/usr/bin/env python3
"""Anchor players for seasons without rank data.

ballchasing stopped tagging ranks in mid-2026 (see README). Anchors are players
whose rank is looked up manually (e.g. on tracker.gg); their recent lobbies are
then labeled with that rank, which the f22 anchor test showed is reliable from
about Diamond up.

  validate   How well the stat-based tier estimate matches true ranks (f22)
  candidates Pick frequent uploaders from recent games, spread across estimated
             tiers, and write a CSV to fill in with looked-up ranks
  import     Load the filled-in CSV as anchors (rank_source 'manual')
"""

from __future__ import annotations

import argparse
import collections
import csv
import json
import statistics
import sys
import urllib.parse
from datetime import datetime, timedelta, timezone
from pathlib import Path

from benchmarks import stat_unit
from collector import (
    DEFAULT_DB, PLAYLIST, TIER_INDEX, TIERS, Ballchasing, Store, fetch_pending, load_api_key, log,
    player_rows, season_number,
)

HERE = Path(__file__).resolve().parent
DEFAULT_BENCHMARKS = HERE / "data" / "benchmarks.json"
TOP_STATS = 10
TRACKER = "https://rocketleague.tracker.network/rocket-league/profile/{platform}/{ident}/overview"


# --- Tier estimate -----------------------------------------------------------

def load_benchmarks(path: Path) -> dict:
    if not path.exists():
        sys.exit(f"{path} not found: run benchmarks.py first")
    return json.loads(path.read_text())


def game_values(row) -> dict[str, float]:
    minutes = row.duration / 60 if row.duration else 0
    out = {}
    for k, v in row.stats.items():
        if stat_unit(k) == "per_minute":
            if minutes:
                out[k] = v / minutes
        else:
            out[k] = v
    return out


def _tier_position(value: float, medians: list[tuple[int, float]]) -> float:
    """Fractional tier index where `value` falls along the tier medians.

    `medians` is (tier_index, p50) ascending by tier. Medians aren't strictly
    monotone, so this uses the best-fitting segment: the first crossing in
    the stat's direction, clamped to the ends of the ladder.
    """
    rising = medians[-1][1] >= medians[0][1]
    sign = 1 if rising else -1
    if sign * value <= sign * medians[0][1]:
        return float(medians[0][0])
    for (t0, m0), (t1, m1) in zip(medians, medians[1:]):
        if sign * m0 <= sign * value <= sign * m1 and m1 != m0:
            return t0 + (t1 - t0) * (value - m0) / (m1 - m0)
    return float(medians[-1][0])


def estimate_tier(games: list[dict[str, float]], bench: dict) -> tuple[int, float] | None:
    """Weighted median, across the strongest rank-signal stats, of where the
    player's median value sits along the tier medians. Returns (tier, position)."""
    signals = sorted(
        ((name, meta["rank_signal"]["weight"]) for name, meta in bench["stats"].items() if meta.get("rank_signal")),
        key=lambda kv: -kv[1],
    )[:TOP_STATS]
    tiers = [t for t in bench["tiers"] if not t["low_sample"]]
    positions: list[tuple[float, float]] = []
    for name, weight in signals:
        vals = [g[name] for g in games if name in g]
        medians = [(t["tier_index"], t["stats"][name]["p50"]) for t in tiers if name in t["stats"]]
        if vals and len(medians) >= 2:
            positions.append((_tier_position(statistics.median(vals), medians), weight))
    if len(positions) < TOP_STATS // 2:
        return None
    positions.sort()
    half, acc = sum(w for _, w in positions) / 2, 0.0
    for pos, w in positions:
        acc += w
        if acc >= half:
            return round(pos), pos
    return round(positions[-1][0]), positions[-1][0]


# --- validate ----------------------------------------------------------------

def cmd_validate(args) -> None:
    store = Store(Path(args.db))
    bench = load_benchmarks(Path(args.benchmarks))
    snum = season_number(args.season)
    per_player: dict[str, list] = collections.defaultdict(list)
    for (src,) in store.db.execute(
        "SELECT DISTINCT source FROM replays WHERE season = ? AND source LIKE 'anchor:%'", (snum,)
    ):
        player = src[len("anchor:"):]
        ids = [r[0] for r in store.db.execute("SELECT id FROM replays WHERE source = ? AND detail IS NOT NULL", (src,))]
        for _, detail in store.details(ids=ids):
            for row in player_rows(detail):
                if row.player == player and row.full_game and row.rank_steps is not None:
                    per_player[player].append(row)
    errs = []
    for player, rows in per_player.items():
        if len(rows) < args.min_games:
            continue
        est = estimate_tier([game_values(r) for r in rows], bench)
        if est is None:
            continue
        true_tier = int(statistics.median(r.rank_steps for r in rows)) // 4
        errs.append((true_tier, est[0] - true_tier))
    if not errs:
        sys.exit("No anchor players with enough games; run collector.py anchor-test first")
    a = [abs(e) for _, e in errs]
    print(f"Tier estimate vs true rank, {len(errs)} players with >= {args.min_games} games:")
    print(f"  exact tier {100 * sum(x == 0 for x in a) / len(a):.0f}%   within 1 tier {100 * sum(x <= 1 for x in a) / len(a):.0f}%"
          f"   within 2 tiers {100 * sum(x <= 2 for x in a) / len(a):.0f}%   bias {statistics.mean(e for _, e in errs):+.1f} tiers")
    hi = [abs(e) for t, e in errs if t >= TIER_INDEX["diamond-1"]]
    if hi:
        print(f"  Diamond 1 and up ({len(hi)}): within 1 tier {100 * sum(x <= 1 for x in hi) / len(hi):.0f}%"
              f"   within 2 tiers {100 * sum(x <= 2 for x in hi) / len(hi):.0f}%")


# --- candidates --------------------------------------------------------------

def tracker_url(platform: str, ident: str, name: str) -> str:
    """tracker.gg profile URL: Steam by SteamID64, other platforms by display name."""
    if platform == "steam":
        return TRACKER.format(platform="steam", ident=ident)
    mapped = {"epic": "epic", "ps4": "psn", "xbox": "xbl"}.get(platform)
    if not mapped:
        return ""
    return TRACKER.format(platform=mapped, ident=urllib.parse.quote(name, safe=""))


def cmd_candidates(args) -> None:
    store = Store(Path(args.db))
    bench = load_benchmarks(Path(args.benchmarks))
    api = Ballchasing(load_api_key())
    season = args.season
    snum = season_number(season)
    min_tier = TIER_INDEX[args.min_tier]
    end = datetime.now(timezone.utc)
    start = end - timedelta(days=args.days)

    # 1. Pool: one list page per day; count appearances per player
    appearances: dict[str, list[str]] = collections.defaultdict(list)
    names: dict[str, str] = {}
    dates: dict[str, str] = {}
    for day in range(args.days):
        w_start = start + timedelta(days=day)
        page = api.list_replays(
            playlist=PLAYLIST, season=season, count=200, sort_by="replay-date", sort_dir="desc",
            replay_date_after=w_start.isoformat(), replay_date_before=(w_start + timedelta(days=1)).isoformat(),
        )
        for item in page.get("list", []):
            dates[item["id"]] = item["date"]
            for color in ("blue", "orange"):
                for p in item.get(color, {}).get("players", []):
                    pid = p.get("id") or {}
                    if not pid.get("platform") or not pid.get("id"):
                        continue
                    key = f"{pid['platform']}:{pid['id']}"
                    appearances[key].append(item["id"])
                    names[key] = p.get("name", "")
    log(f"pool: {len(dates)} games, {len(appearances)} players over {args.days} days")

    frequent = sorted(
        (k for k, ids in appearances.items() if len(set(ids)) >= args.min_games and not k.startswith("psynet:")),
        key=lambda k: -len(set(appearances[k])),
    )[: args.max_candidates]
    log(f"{len(frequent)} players with >= {args.min_games} games")

    # 2. Fetch a few of each candidate's games for a tier estimate
    for key in frequent:
        for rid in list(dict.fromkeys(appearances[key]))[: args.games_per_candidate]:
            store.queue(rid, snum, dates[rid], f"candidate:{key}")
    store.commit()
    fetch_pending(api, store, snum, None)

    # 3. Estimate each candidate's tier
    estimates = []
    for key in frequent:
        ids = list(dict.fromkeys(appearances[key]))[: args.games_per_candidate]
        rows = [r for _, d in store.details(ids=ids) for r in player_rows(d) if r.player == key and r.full_game]
        est = estimate_tier([game_values(r) for r in rows], bench) if rows else None
        if est:
            estimates.append((key, est[0], len(set(appearances[key]))))

    # 4. Spread the list across tiers from min_tier up
    by_tier: dict[int, list] = collections.defaultdict(list)
    for key, tier, games in estimates:
        if tier >= min_tier:
            by_tier[tier].append((games, key))
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    picked = 0
    with out.open("w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["player", "name", "platform", "tracker_url", "season", "games_last_%dd" % args.days,
                    "estimated_tier", "rank_2v2", "division", "checked_on", "notes"])
        for tier in sorted(by_tier):
            for games, key in sorted(by_tier[tier], reverse=True)[: args.per_tier]:
                platform, ident = key.split(":", 1)
                w.writerow([key, names[key], platform, tracker_url(platform, ident, names[key]), season, games,
                            TIERS[tier], "", "", "", ""])
                picked += 1
    log(f"wrote {picked} candidates to {out}")
    for tier in range(min_tier, len(TIERS)):
        print(f"  {TIERS[tier]:18} {min(len(by_tier.get(tier, [])), args.per_tier)}")


# --- import ------------------------------------------------------------------

_ROMAN = {"i": 1, "ii": 2, "iii": 3}
_SHORT = {"b": "bronze", "s": "silver", "g": "gold", "p": "platinum", "d": "diamond", "c": "champion", "gc": "grand-champion"}


def parse_rank(text: str) -> int | None:
    """Tier index from a rank as typed: 'diamond-2', 'Diamond 2', 'Diamond II',
    'D2', 'C3', 'GC1', 'Grand Champion II', 'SSL', 'Supersonic Legend'."""
    t = " ".join(text.strip().lower().replace("-", " ").replace("_", " ").split())
    if not t:
        return None
    if t in ("ssl", "supersonic legend"):
        return TIER_INDEX["supersonic-legend"]
    parts = t.split(" ")
    num_text = parts[-1]
    num = _ROMAN.get(num_text) or (int(num_text) if num_text.isdigit() else None)
    if num is None:
        # compact forms like d2, gc1
        for prefix in sorted(_SHORT, key=len, reverse=True):
            if t.startswith(prefix) and t[len(prefix):].isdigit():
                return TIER_INDEX.get(f"{_SHORT[prefix]}-{int(t[len(prefix):])}")
        return None
    name = " ".join(parts[:-1])
    name = _SHORT.get(name, name).replace(" ", "-")
    return TIER_INDEX.get(f"{name}-{num}")


def cmd_import(args) -> None:
    store = Store(Path(args.db))
    loaded, skipped, errors = 0, 0, []
    with open(args.csv, newline="") as fh:
        for line, row in enumerate(csv.DictReader(fh), start=2):
            rank_text = (row.get("rank_2v2") or "").strip()
            if not rank_text:
                skipped += 1
                continue
            tier = parse_rank(rank_text)
            if tier is None:
                errors.append(f"line {line}: can't read rank {rank_text!r} for {row.get('name')}")
                continue
            div_text = (row.get("division") or "").strip()
            division = int(div_text) if div_text.isdigit() and 1 <= int(div_text) <= 4 else 1
            if tier == TIER_INDEX["supersonic-legend"]:
                division = 1
            as_of = (row.get("checked_on") or "").strip() or datetime.now(timezone.utc).date().isoformat()
            store.db.execute(
                "INSERT OR REPLACE INTO anchors VALUES (?, ?, ?, 'manual', ?)",
                (season_number(row["season"]), row["player"], tier * 4 + division - 1, as_of),
            )
            loaded += 1
    store.commit()
    for e in errors:
        print("  " + e)
    log(f"imported {loaded} anchors, {skipped} rows without a rank skipped, {len(errors)} errors")
    if errors:
        sys.exit(1)


def main(argv: list[str] | None = None) -> None:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--db", default=str(DEFAULT_DB))
    p.add_argument("--benchmarks", default=str(DEFAULT_BENCHMARKS))
    sub = p.add_subparsers(dest="cmd", required=True)

    v = sub.add_parser("validate", help="Check the tier estimate against true ranks (needs anchor-test data)")
    v.add_argument("--season", default="f22")
    v.add_argument("--min-games", type=int, default=5)

    c = sub.add_parser("candidates", help="Write a CSV of players to look up manually")
    c.add_argument("--season", required=True, help="current season, e.g. f24")
    c.add_argument("--days", type=int, default=14)
    c.add_argument("--min-games", type=int, default=8)
    c.add_argument("--max-candidates", type=int, default=200)
    c.add_argument("--games-per-candidate", type=int, default=6)
    c.add_argument("--min-tier", default="diamond-1", choices=TIERS)
    c.add_argument("--per-tier", type=int, default=5)
    c.add_argument("--out", required=True)

    i = sub.add_parser("import", help="Load a filled-in candidates CSV as manual anchors")
    i.add_argument("csv")

    args = p.parse_args(argv)
    {"validate": cmd_validate, "candidates": cmd_candidates, "import": cmd_import}[args.cmd](args)


if __name__ == "__main__":
    main()
