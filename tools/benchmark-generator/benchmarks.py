#!/usr/bin/env python3
"""Build benchmarks.json from collected rows.

Per rank tier: sample size and, per stat, mean, stddev and empirical quantiles
(p5, p10, p25, p50, p75, p90, p95). Per stat: a rank signal (Spearman
correlation with rank, giving direction and weight) so coaching can rank gaps by
whether they separate ranks at all.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from datetime import datetime, timezone
from pathlib import Path

from collector import DEFAULT_DB, PLAYLIST, TIERS, Store, player_rows, season_number

SCHEMA_VERSION = 1
QUANTILES = {"p5": 0, "p10": 1, "p25": 4, "p50": 9, "p75": 14, "p90": 17, "p95": 18}  # indices into 5% cut points
MIN_TIER_ROWS = 30
# Already a rate or a ratio; never divided by game length.
RATE_STATS = {"boost_bpm", "boost_bcpm", "core_shooting_percentage"}


def stat_unit(name: str) -> str:
    """'as_is' for percentages, averages and rates. 'per_minute' for totals."""
    _, _, short = name.partition("_")
    if name in RATE_STATS or short.startswith(("percent_", "avg_")):
        return "as_is"
    return "per_minute"


def redundant(name: str, all_names: set[str]) -> bool:
    """time_X is dropped when percent_X carries the same information."""
    group, _, short = name.partition("_")
    return short.startswith("time_") and f"{group}_percent_{short[len('time_'):]}" in all_names


def sig(x: float, digits: int = 4) -> float:
    return float(f"{x:.{digits}g}")


def collect_values(store: Store, seasons: list[int]) -> tuple[dict[int, dict[str, list[float]]], list[tuple[int, dict[str, float]]], dict[int, set[str]]]:
    """Returns per-tier stat values, (rank_steps, stats) pairs for rank signal, and games per tier."""
    per_tier: dict[int, dict[str, list[float]]] = {}
    pairs: list[tuple[int, dict[str, float]]] = []
    games: dict[int, set[str]] = {}
    for season in seasons:
        # Only the per-tier sample: anchor-test games are concentrated on a few
        # players and would skew the distributions.
        ids = [r[0] for r in store.db.execute(
            "SELECT id FROM replays WHERE season = ? AND source LIKE 'tier:%' AND detail IS NOT NULL", (season,)
        )]
        for _, detail in store.details(ids=ids):
            for row in player_rows(detail):
                if row.rank_steps is None or not row.full_game or row.duration <= 0:
                    continue
                minutes = row.duration / 60
                values = {
                    k: (v / minutes if stat_unit(k) == "per_minute" else v) for k, v in row.stats.items()
                }
                tier = row.rank_steps // 4
                bucket = per_tier.setdefault(tier, {})
                for k, v in values.items():
                    bucket.setdefault(k, []).append(v)
                games.setdefault(tier, set()).add(row.replay_id)
                pairs.append((row.rank_steps, values))
    return per_tier, pairs, games


def distribution(values: list[float]) -> dict[str, float] | None:
    if len(values) < 2:
        return None
    cuts = statistics.quantiles(values, n=20, method="inclusive")
    out = {"n": len(values), "mean": sig(statistics.fmean(values)), "stddev": sig(statistics.stdev(values))}
    out |= {name: sig(cuts[i]) for name, i in QUANTILES.items()}
    return out


def rank_signal(pairs: list[tuple[int, dict[str, float]]], stat: str) -> dict[str, float | str] | None:
    xs, ys = [], []
    for steps, values in pairs:
        if stat in values:
            xs.append(values[stat])
            ys.append(steps)
    if len(xs) < 30 or len(set(xs)) < 2:
        return None
    rho = statistics.correlation(xs, ys, method="ranked")
    return {
        "spearman": sig(rho, 3),
        "direction": "higher" if rho > 0 else "lower",
        "weight": sig(abs(rho), 3),
    }


def build(store: Store, seasons: list[str]) -> dict:
    nums = [season_number(s) for s in seasons]
    per_tier, pairs, games = collect_values(store, nums)
    if not per_tier:
        sys.exit("No ranked full-game rows found for " + ", ".join(seasons))
    names = set().union(*(b.keys() for b in per_tier.values()))
    stats = sorted(n for n in names if not redundant(n, names))

    stat_meta = {}
    for s in stats:
        meta: dict = {"group": s.partition("_")[0], "unit": stat_unit(s)}
        signal = rank_signal(pairs, s)
        if signal:
            meta["rank_signal"] = signal
        stat_meta[s] = meta

    tiers = []
    for t in sorted(per_tier):
        bucket = per_tier[t]
        n_rows = max(len(v) for v in bucket.values())
        entry = {
            "rank": TIERS[t],
            "tier_index": t,
            "sample_size": n_rows,
            "games": len(games[t]),
            "low_sample": n_rows < MIN_TIER_ROWS,
            "stats": {s: d for s in stats if (d := distribution(bucket.get(s, [])))},
        }
        tiers.append(entry)

    return {
        "schema_version": SCHEMA_VERSION,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "source": {
            "site": "ballchasing.com",
            "playlist": PLAYLIST,
            "seasons": seasons,
            "rank_source": "ballchasing per-player rank",
            "row_filter": "per-tier sample (min-rank = max-rank lobbies); ranked players present for >= 90% of the game",
        },
        "stats": stat_meta,
        "tiers": tiers,
    }


def main(argv: list[str] | None = None) -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--db", default=str(DEFAULT_DB))
    p.add_argument("--season", action="append", required=True, help="repeatable, e.g. --season f22")
    p.add_argument("--out", default=str(Path(DEFAULT_DB).parent / "benchmarks.json"))
    args = p.parse_args(argv)

    data = build(Store(Path(args.db)), args.season)
    out = Path(args.out)
    out.write_text(json.dumps(data, separators=(",", ":")))
    size_kb = out.stat().st_size / 1024
    print(f"wrote {out} ({size_kb:.0f} KB): {len(data['tiers'])} tiers, {len(data['stats'])} stats")
    for t in data["tiers"]:
        flag = "  (low sample)" if t["low_sample"] else ""
        print(f"  {t['rank']:18} rows {t['sample_size']:5}  games {t['games']:4}{flag}")
    top = sorted(
        ((s, m["rank_signal"]) for s, m in data["stats"].items() if "rank_signal" in m),
        key=lambda kv: -kv[1]["weight"],
    )[:12]
    print("strongest rank signals:")
    for s, sigl in top:
        print(f"  {s:45} {sigl['spearman']:+.2f}  ({sigl['direction']} at higher ranks)")


if __name__ == "__main__":
    main()
