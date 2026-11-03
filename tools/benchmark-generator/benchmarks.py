#!/usr/bin/env python3
"""Build benchmarks.json from collected rows.

Per rank tier: sample size and, per stat, mean, stddev and empirical quantiles
(p5, p10, p25, p50, p75, p90, p95). Per stat: a rank signal (Spearman
correlation with rank, giving direction and weight) so coaching can rank gaps by
whether they separate ranks at all.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
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


def ranked_rows(store: Store, season: int):
    """(row, rank_steps) from the per-tier sample, using ballchasing's per-player rank.

    Anchor-test games are excluded: they're concentrated on a few players.
    """
    ids = [r[0] for r in store.db.execute(
        "SELECT id FROM replays WHERE season = ? AND source LIKE 'tier:%' AND detail IS NOT NULL", (season,)
    )]
    for _, detail in store.details(ids=ids):
        for row in player_rows(detail):
            if row.rank_steps is not None and row.full_game:
                yield row, row.rank_steps


def aggregate(labeled) -> tuple[dict[int, dict[str, list[float]]], dict[int, list[tuple[int, dict[str, float]]]], dict[int, set[str]]]:
    """Per-tier stat values, per-tier (steps, values) pairs for rank signal, and games per tier."""
    per_tier: dict[int, dict[str, list[float]]] = {}
    pairs: dict[int, list[tuple[int, dict[str, float]]]] = {}
    games: dict[int, set[str]] = {}
    for row, steps in labeled:
        if row.duration <= 0:
            continue
        minutes = row.duration / 60
        values = {k: (v / minutes if stat_unit(k) == "per_minute" else v) for k, v in row.stats.items()}
        tier = steps // 4
        bucket = per_tier.setdefault(tier, {})
        for k, v in values.items():
            bucket.setdefault(k, []).append(v)
        games.setdefault(tier, set()).add(row.replay_id)
        pairs.setdefault(tier, []).append((steps, values))
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


@dataclass
class TierSource:
    season: str
    rank_source: str
    per_tier: dict
    pairs: dict
    games: dict


def build(store: Store, base_season: str, anchor_season: str | None = None,
          anchor_scope: str = "team", anchor_min_tier: str = "diamond-1", min_anchors: int = 3) -> dict:
    """Benchmarks from `base_season` (ballchasing ranks). With `anchor_season`,
    tiers from `anchor_min_tier` up use manual-anchor-labeled rows from that
    season when they have at least `min_anchors` anchors and MIN_TIER_ROWS rows,
    so a tier never reflects just one or two people's play."""
    base = TierSource(base_season, "ballchasing per-player rank",
                      *aggregate(ranked_rows(store, season_number(base_season))))
    sources = {t: base for t in base.per_tier}
    anchor = None
    if anchor_season:
        from anchors import anchors_with_games, labeled_rows  # anchors imports this module
        anchor = TierSource(anchor_season, f"manual anchor (tracker.gg), scope={anchor_scope}",
                            *aggregate(labeled_rows(store, season_number(anchor_season), anchor_scope)))
        min_idx = TIERS.index(anchor_min_tier)
        anchor_counts = anchors_with_games(store, season_number(anchor_season))
        for t, bucket in anchor.per_tier.items():
            enough_rows = max(len(v) for v in bucket.values()) >= MIN_TIER_ROWS
            if t >= min_idx and anchor_counts.get(t, 0) >= min_anchors and enough_rows:
                sources[t] = anchor
    if not sources:
        sys.exit(f"No ranked full-game rows found for {base_season}")

    names = set().union(*(src.per_tier[t].keys() for t, src in sources.items()))
    stats = sorted(n for n in names if not redundant(n, names))
    # Rank signal over the rows actually used for each tier
    pairs = [p for t, src in sources.items() for p in src.pairs[t]]
    stat_meta = {}
    for s in stats:
        meta: dict = {"group": s.partition("_")[0], "unit": stat_unit(s)}
        signal = rank_signal(pairs, s)
        if signal:
            meta["rank_signal"] = signal
        stat_meta[s] = meta

    tiers = []
    for t in sorted(sources):
        src = sources[t]
        bucket = src.per_tier[t]
        n_rows = max(len(v) for v in bucket.values())
        tiers.append({
            "rank": TIERS[t],
            "tier_index": t,
            "sample_size": n_rows,
            "games": len(src.games[t]),
            "low_sample": n_rows < MIN_TIER_ROWS,
            "season": src.season,
            "rank_source": src.rank_source,
            "stats": {s: d for s in stats if (d := distribution(bucket.get(s, [])))},
        })

    used = sorted({src.season for src in sources.values()}, key=season_number)
    return {
        "schema_version": SCHEMA_VERSION,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "source": {
            "site": "ballchasing.com",
            "playlist": PLAYLIST,
            "seasons": used,
            "rank_source": "; ".join(sorted({src.rank_source for src in sources.values()})),
            "row_filter": "players present for >= 90% of the game; per-tier rank source and season in each tier",
        },
        "stats": stat_meta,
        "tiers": tiers,
        "_compare": (base, anchor),
    }


def drift_report(base: TierSource, anchor: TierSource, stat_names: list[str]) -> None:
    """Median of key stats per tier, base season vs anchor season."""
    print(f"\nSkill drift, median {base.season} -> {anchor.season} (anchor tiers with >= {MIN_TIER_ROWS} rows):")
    header = "  " + f"{'tier':18}" + "".join(f"{n.split('_', 1)[1][:22]:>26}" for n in stat_names)
    print(header)
    for t in sorted(anchor.per_tier):
        a = anchor.per_tier[t]
        if t not in base.per_tier or max(len(v) for v in a.values()) < MIN_TIER_ROWS:
            continue
        b = base.per_tier[t]
        cells = []
        for n in stat_names:
            if a.get(n) and b.get(n):
                mb, ma = statistics.median(b[n]), statistics.median(a[n])
                cells.append(f"{mb:8.1f} -> {ma:8.1f} ({100 * (ma - mb) / mb:+4.0f}%)")
            else:
                cells.append(f"{'-':>26}")
        print("  " + f"{TIERS[t]:18}" + "".join(f"{c:>26}" for c in cells))


def main(argv: list[str] | None = None) -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--db", default=str(DEFAULT_DB))
    p.add_argument("--season", required=True, help="base season with ballchasing ranks, e.g. f22")
    p.add_argument("--anchor-season", help="season labeled by manual anchors, e.g. f24")
    p.add_argument("--anchor-scope", default="team", choices=["anchor", "team", "lobby"])
    p.add_argument("--anchor-min-tier", default="diamond-1", choices=TIERS)
    p.add_argument("--min-anchors", type=int, default=3, help="anchors a tier needs before its anchor data is used")
    p.add_argument("--out", default=str(Path(DEFAULT_DB).parent / "benchmarks.json"))
    args = p.parse_args(argv)

    data = build(Store(Path(args.db)), args.season, args.anchor_season, args.anchor_scope, args.anchor_min_tier,
                 args.min_anchors)
    base, anchor = data.pop("_compare")
    out = Path(args.out)
    out.write_text(json.dumps(data, separators=(",", ":")))
    size_kb = out.stat().st_size / 1024
    print(f"wrote {out} ({size_kb:.0f} KB): {len(data['tiers'])} tiers, {len(data['stats'])} stats, seasons {data['source']['seasons']}")
    for t in data["tiers"]:
        flag = "  (low sample)" if t["low_sample"] else ""
        print(f"  {t['rank']:18} {t['season']:4} rows {t['sample_size']:5}  games {t['games']:4}{flag}")
    top = sorted(
        ((s, m["rank_signal"]) for s, m in data["stats"].items() if "rank_signal" in m),
        key=lambda kv: -kv[1]["weight"],
    )[:8]
    print("strongest rank signals:")
    for s, sigl in top:
        print(f"  {s:45} {sigl['spearman']:+.2f}  ({sigl['direction']} at higher ranks)")
    if anchor:
        drift_report(base, anchor, [s for s, _ in top[:3]])


if __name__ == "__main__":
    main()
