#!/usr/bin/env python3
"""Benchmark data collector for RL Replay Review.

Collects ranked-doubles replay stats from ballchasing.com into a local SQLite
store, one row per player per game, tagged with that player's own rank.
Standard library only. See README.md for usage.
"""

from __future__ import annotations

import argparse
import collections
import csv
import gzip
import json
import os
import random
import sqlite3
import statistics
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path

API = "https://ballchasing.com/api"
PLAYLIST = "ranked-doubles"
HERE = Path(__file__).resolve().parent
DEFAULT_DB = HERE / "data" / "collector.sqlite"
APP_SETTINGS = (
    Path.home() / "Library" / "Application Support" / "com.distortion2112.rl-replay-review" / "settings.json"
)

# ballchasing rank ids accepted by min-rank/max-rank, lowest to highest.
TIERS = [
    "bronze-1", "bronze-2", "bronze-3",
    "silver-1", "silver-2", "silver-3",
    "gold-1", "gold-2", "gold-3",
    "platinum-1", "platinum-2", "platinum-3",
    "diamond-1", "diamond-2", "diamond-3",
    "champion-1", "champion-2", "champion-3",
    "grand-champion-1", "grand-champion-2", "grand-champion-3",
    "supersonic-legend",
]
TIER_INDEX = {t: i for i, t in enumerate(TIERS)}

# ballchasing per-tier limits (https://ballchasing.com/doc/api), as
# (calls per second, calls per hour or None) for list / get / file endpoints.
RATE_LIMITS = {
    "regular":  {"list": (2, 500),   "get": (2, 1000),  "file": (1, 200)},
    "gold":     {"list": (2, 1000),  "get": (2, 2000),  "file": (2, 400)},
    "diamond":  {"list": (4, 2000),  "get": (4, 5000),  "file": (2, 1000)},
    "champion": {"list": (8, None),  "get": (8, None),  "file": (2, 2000)},
    "gc":       {"list": (16, None), "get": (16, None), "file": (2, None)},
}


def log(msg: str) -> None:
    print(f"[{datetime.now().strftime('%H:%M:%S')}] {msg}", flush=True)


# --- Ranks and seasons -------------------------------------------------------

def rank_steps(rank: dict | None) -> int | None:
    """Position on a single division-level scale: 4 steps per tier.

    ballchasing ranks carry `tier` (1 = bronze-1 ... 22 = supersonic-legend)
    and `division` (1-4). Returns None when unranked or missing.
    """
    if not rank:
        return None
    tier, div = rank.get("tier"), rank.get("division")
    if not isinstance(tier, int) or tier < 1:
        return None
    if not isinstance(div, int) or not 1 <= div <= 4:
        div = 1
    return (tier - 1) * 4 + (div - 1)


def season_number(season: str) -> int:
    """'f22' -> 22. Only free-to-play seasons are supported."""
    if not season.startswith("f") or not season[1:].isdigit():
        raise ValueError(f"Expected a free-to-play season like f22, got {season!r}")
    return int(season[1:])


def retained_seasons(current: int, labeled: list[int]) -> set[int]:
    """Seasons to keep: current n, n-1, and the newest season with rank labels."""
    keep = {current, current - 1}
    if labeled:
        keep.add(max(labeled))
    return keep


# --- HTTP client with tier-aware rate limiting --------------------------------

class RateLimiter:
    """Per-endpoint-group limiter enforcing both per-second and per-hour caps."""

    def __init__(self, per_second: float, per_hour: int | None):
        self.interval = 1.0 / per_second
        self.per_hour = per_hour
        self.last = 0.0
        self.window: collections.deque[float] = collections.deque()

    def wait(self) -> None:
        now = time.monotonic()
        if self.per_hour:
            while self.window and now - self.window[0] >= 3600:
                self.window.popleft()
            if len(self.window) >= self.per_hour:
                sleep_for = 3600 - (now - self.window[0]) + 1
                log(f"hourly cap ({self.per_hour}) reached, sleeping {sleep_for / 60:.1f} min")
                time.sleep(sleep_for)
                now = time.monotonic()
        gap = self.interval - (now - self.last)
        if gap > 0:
            time.sleep(gap)
        self.last = time.monotonic()
        if self.per_hour:
            self.window.append(self.last)


class Ballchasing:
    def __init__(self, key: str, tier: str | None = None):
        self.key = key
        self.tier = tier or self.detect_tier()
        limits = RATE_LIMITS.get(self.tier, RATE_LIMITS["regular"])
        self.limiters = {group: RateLimiter(*lim) for group, lim in limits.items()}

    def _request(self, url: str, group: str | None) -> dict:
        for attempt in range(6):
            if group:
                self.limiters[group].wait()
            req = urllib.request.Request(url, headers={"Authorization": self.key, "User-Agent": "rl-replay-review-benchmark/0.1"})
            try:
                with urllib.request.urlopen(req, timeout=60) as resp:
                    return json.load(resp)
            except urllib.error.HTTPError as e:
                if e.code == 429:
                    retry = int(e.headers.get("Retry-After") or 0) or 30 * (attempt + 1)
                    log(f"429 from ballchasing, backing off {retry}s")
                    time.sleep(retry)
                    continue
                if e.code in (500, 502, 503, 504):
                    time.sleep(5 * (attempt + 1))
                    continue
                raise
            except (urllib.error.URLError, TimeoutError) as e:
                log(f"network error ({e}), retrying")
                time.sleep(5 * (attempt + 1))
        raise RuntimeError(f"Giving up on {url.split('?')[0]}")

    def detect_tier(self) -> str:
        info = self._request(f"{API}/", None)
        return info.get("type", "regular")

    def list_replays(self, **params) -> dict:
        query = urllib.parse.urlencode({k.replace("_", "-"): v for k, v in params.items() if v is not None}, doseq=True)
        return self._request(f"{API}/replays?{query}", "list")

    def list_next(self, next_url: str) -> dict:
        return self._request(next_url, "list")

    def get_replay(self, replay_id: str) -> dict:
        return self._request(f"{API}/replays/{replay_id}", "get")


def load_api_key() -> str:
    key = os.environ.get("BALLCHASING_API_KEY")
    if key:
        return key.strip()
    if APP_SETTINGS.exists():
        key = json.loads(APP_SETTINGS.read_text()).get("api_key")
        if key:
            return key.strip()
    sys.exit("No API key: set BALLCHASING_API_KEY or save one in the app first.")


# --- Storage -----------------------------------------------------------------

SCHEMA = """
CREATE TABLE IF NOT EXISTS replays (
    id          TEXT PRIMARY KEY,
    season      INTEGER NOT NULL,
    date        TEXT NOT NULL,
    source      TEXT NOT NULL,          -- why it was queued: 'tier:<rank>' or 'anchor:<player>'
    detail      BLOB,                   -- gzipped JSON from GET /replays/{id}; NULL until fetched
    fetched_at  TEXT
);
CREATE INDEX IF NOT EXISTS replays_season ON replays(season);
CREATE INDEX IF NOT EXISTS replays_pending ON replays(detail) WHERE detail IS NULL;
CREATE TABLE IF NOT EXISTS anchors (
    season      INTEGER NOT NULL,
    player      TEXT NOT NULL,          -- '<platform>:<id>'
    rank_steps  INTEGER NOT NULL,       -- reference rank on the division scale
    rank_source TEXT NOT NULL,          -- 'ballchasing' | 'manual' | api name
    as_of       TEXT NOT NULL,          -- when the reference rank was observed
    PRIMARY KEY (season, player)
);
"""


class Store:
    def __init__(self, path: Path):
        path.parent.mkdir(parents=True, exist_ok=True)
        self.db = sqlite3.connect(path)
        self.db.executescript(SCHEMA)

    def queue(self, replay_id: str, season: int, date: str, source: str) -> bool:
        cur = self.db.execute(
            "INSERT OR IGNORE INTO replays (id, season, date, source) VALUES (?, ?, ?, ?)",
            (replay_id, season, date, source),
        )
        return cur.rowcount == 1

    def pending(self, season: int | None = None) -> list[str]:
        sql = "SELECT id FROM replays WHERE detail IS NULL"
        args: tuple = ()
        if season is not None:
            sql += " AND season = ?"
            args = (season,)
        return [r[0] for r in self.db.execute(sql + " ORDER BY date", args)]

    def save_detail(self, replay_id: str, detail: dict) -> None:
        blob = gzip.compress(json.dumps(detail, separators=(",", ":")).encode())
        self.db.execute(
            "UPDATE replays SET detail = ?, fetched_at = ? WHERE id = ?",
            (blob, datetime.now(timezone.utc).isoformat(), replay_id),
        )

    def details(self, season: int | None = None, ids: list[str] | None = None):
        sql = "SELECT id, detail FROM replays WHERE detail IS NOT NULL"
        args: list = []
        if season is not None:
            sql += " AND season = ?"
            args.append(season)
        if ids is not None:
            sql += f" AND id IN ({','.join('?' * len(ids))})"
            args.extend(ids)
        for rid, blob in self.db.execute(sql, args):
            yield rid, json.loads(gzip.decompress(blob))

    def count_by_source(self, season: int) -> dict[str, tuple[int, int]]:
        out = {}
        for source, total, done in self.db.execute(
            "SELECT source, COUNT(*), COUNT(detail) FROM replays WHERE season = ? GROUP BY source", (season,)
        ):
            out[source] = (total, done)
        return out

    def commit(self) -> None:
        self.db.commit()


# --- Row flattening ----------------------------------------------------------

@dataclass
class PlayerRow:
    replay_id: str
    season: int
    date: str
    duration: int
    overtime: bool
    team: str
    won: bool
    score_diff: int
    player: str
    name: str
    rank_id: str | None
    rank_steps: int | None
    teammate_rank_steps: int | None
    full_game: bool
    stats: dict[str, float]


def player_rows(detail: dict) -> list[PlayerRow]:
    """One row per player, with game context. Stats are flattened as group_stat."""
    rows: list[PlayerRow] = []
    duration = int(detail.get("duration") or 0)
    goals = {c: (detail.get(c, {}).get("stats", {}).get("core", {}).get("goals") or 0) for c in ("blue", "orange")}
    season = detail.get("season")
    for color, other in (("blue", "orange"), ("orange", "blue")):
        players = detail.get(color, {}).get("players", [])
        for p in players:
            pid = p.get("id") or {}
            mates = [m for m in players if m is not p]
            mate_steps = rank_steps(mates[0].get("rank")) if mates else None
            played = (p.get("end_time") or 0) - (p.get("start_time") or 0)
            flat = {}
            for group, values in (p.get("stats") or {}).items():
                for k, v in values.items():
                    if isinstance(v, (int, float)) and not isinstance(v, bool):
                        flat[f"{group}_{k}"] = v
            rank = p.get("rank")
            steps = rank_steps(rank)
            rows.append(PlayerRow(
                replay_id=detail["id"],
                season=season,
                date=detail.get("date", ""),
                duration=duration,
                overtime=bool(detail.get("overtime")),
                team=color,
                won=goals[color] > goals[other],
                score_diff=goals[color] - goals[other],
                player=f"{pid.get('platform')}:{pid.get('id')}",
                name=p.get("name", ""),
                # Normalized from the tier number: ballchasing reports GC1 as both
                # 'grand-champion' and 'grand-champion-1'.
                rank_id=TIERS[steps // 4] if steps is not None and steps // 4 < len(TIERS) else None,
                rank_steps=steps,
                teammate_rank_steps=mate_steps,
                full_game=duration > 0 and played >= 0.9 * duration,
                stats=flat,
            ))
    return rows


# --- Commands ----------------------------------------------------------------

def season_bounds(api: Ballchasing, season: str) -> tuple[datetime, datetime]:
    first = api.list_replays(playlist=PLAYLIST, season=season, count=1, sort_by="replay-date", sort_dir="asc")
    last = api.list_replays(playlist=PLAYLIST, season=season, count=1, sort_by="replay-date", sort_dir="desc")
    if not first.get("list") or not last.get("list"):
        sys.exit(f"No {PLAYLIST} replays found for {season}")
    parse = lambda s: datetime.fromisoformat(s.replace("Z", "+00:00"))
    return parse(first["list"][0]["date"]), parse(last["list"][0]["date"])


def cmd_status(args, api: Ballchasing | None, store: Store) -> None:
    if api:
        latest = api.list_replays(playlist=PLAYLIST, count=1, sort_by="replay-date", sort_dir="desc")
        current = latest["list"][0]["season"] if latest.get("list") else None
        log(f"ballchasing tier: {api.tier} | current season: f{current}")
    seasons = [r[0] for r in store.db.execute("SELECT DISTINCT season FROM replays ORDER BY season")]
    for s in seasons:
        counts = store.count_by_source(s)
        total = sum(t for t, _ in counts.values())
        done = sum(d for _, d in counts.values())
        log(f"f{s}: {done}/{total} replays fetched")
        for source, (t, d) in sorted(counts.items(), key=lambda kv: kv[0]):
            if source.startswith("tier:"):
                print(f"    {source:26} {d:4}/{t}")
        anchors = sum(t for src, (t, _) in counts.items() if src.startswith("anchor:"))
        if anchors:
            print(f"    anchor games              {anchors}")


def cmd_collect(args, api: Ballchasing, store: Store) -> None:
    season = args.season
    snum = season_number(season)
    tiers = args.tiers.split(",") if args.tiers else TIERS
    for t in tiers:
        if t not in TIER_INDEX:
            sys.exit(f"Unknown tier {t}")

    start, end = season_bounds(api, season)
    windows = args.windows
    span = (end - start) / windows
    log(f"{season}: {start:%Y-%m-%d} to {end:%Y-%m-%d}, {windows} windows, target {args.per_tier}/tier, tier {api.tier}")

    for tier in tiers:
        source = f"tier:{tier}"
        have = store.count_by_source(snum).get(source, (0, 0))[0]
        if have >= args.per_tier:
            log(f"{tier}: already queued {have}")
            continue
        per_window = -(-(args.per_tier - have) // windows)
        queued = 0
        for w in range(windows):
            w_start, w_end = start + span * w, start + span * (w + 1)
            page = api.list_replays(
                playlist=PLAYLIST, season=season, min_rank=tier, max_rank=tier, count=min(200, per_window * 2),
                sort_by="replay-date", sort_dir="asc",
                replay_date_after=w_start.isoformat(), replay_date_before=w_end.isoformat(),
            )
            items = page.get("list", [])
            random.Random(f"{season}{tier}{w}").shuffle(items)
            taken = 0
            for item in items:
                if taken >= per_window or have + queued >= args.per_tier:
                    break
                if store.queue(item["id"], snum, item["date"], source):
                    taken += 1
                    queued += 1
        store.commit()
        log(f"{tier}: queued {queued} (now {have + queued})")

    fetch_pending(api, store, snum, args.limit)


def fetch_pending(api: Ballchasing, store: Store, season: int, limit: int | None) -> None:
    ids = store.pending(season)
    if limit:
        ids = ids[:limit]
    log(f"fetching {len(ids)} replay details")
    for i, rid in enumerate(ids, 1):
        try:
            detail = api.get_replay(rid)
        except urllib.error.HTTPError as e:
            if e.code == 404:
                store.db.execute("DELETE FROM replays WHERE id = ?", (rid,))
                continue
            raise
        if detail.get("status") != "ok":
            continue  # still processing on ballchasing; retried next run
        store.save_detail(rid, detail)
        if i % 25 == 0 or i == len(ids):
            store.commit()
            log(f"  {i}/{len(ids)}")
    store.commit()


def cmd_export_rows(args, api, store: Store) -> None:
    snum = season_number(args.season)
    out = Path(args.out)
    fieldnames: list[str] | None = None
    n = 0
    with out.open("w", newline="") as fh:
        writer = None
        for _, detail in store.details(snum):
            for row in player_rows(detail):
                if args.ranked_only and row.rank_steps is None:
                    continue
                record = {k: v for k, v in row.__dict__.items() if k != "stats"} | row.stats
                if writer is None:
                    fieldnames = list(record.keys())
                    writer = csv.DictWriter(fh, fieldnames=fieldnames, extrasaction="ignore", restval="")
                    writer.writeheader()
                writer.writerow(record)
                n += 1
    log(f"wrote {n} rows to {out}")


def cmd_anchor_test(args, api: Ballchasing, store: Store) -> None:
    """Validate anchor labeling on a season that has true per-player ranks.

    For each tier, pick anchors (players seen in collected tier games with a
    known rank), fetch their other games in the season within +/- window days of
    the reference game, and compare lobby-mates' true ranks to the anchor's
    reference rank.
    """
    season = args.season
    snum = season_number(season)
    rng = random.Random(args.seed)

    # Candidate anchors from already-collected tier games: (player -> list of (date, steps))
    seen: dict[str, list[tuple[str, int]]] = collections.defaultdict(list)
    for _, detail in store.details(snum):
        for row in player_rows(detail):
            if row.rank_steps is not None and row.full_game and not row.player.endswith(":None"):
                seen[row.player].append((row.date, row.rank_steps))

    by_tier: dict[int, list[str]] = collections.defaultdict(list)
    for player, obs in seen.items():
        by_tier[obs[0][1] // 4].append(player)

    reference: dict[str, tuple[str, int]] = {}
    for tier_idx in range(len(TIERS)):
        candidates = sorted(by_tier.get(tier_idx, []))
        rng.shuffle(candidates)
        for player in candidates[: args.anchors_per_tier]:
            date, steps = seen[player][0]
            reference[player] = (date, steps)
            store.db.execute(
                "INSERT OR REPLACE INTO anchors VALUES (?, ?, ?, 'ballchasing', ?)", (snum, player, steps, date)
            )
    store.commit()
    log(f"{len(reference)} anchors across {len(by_tier)} tiers")

    # Queue each anchor's nearby games
    for player, (date, _) in reference.items():
        source = f"anchor:{player}"
        if store.count_by_source(snum).get(source):
            continue
        ref = datetime.fromisoformat(date.replace("Z", "+00:00"))
        page = api.list_replays(
            playlist=PLAYLIST, season=season, player_id=player, count=200,
            replay_date_after=(ref - timedelta(days=args.window_days)).isoformat(),
            replay_date_before=(ref + timedelta(days=args.window_days)).isoformat(),
        )
        items = page.get("list", [])
        rng.shuffle(items)
        for item in items[: args.games_per_anchor]:
            store.queue(item["id"], snum, item["date"], source)
        store.commit()
    fetch_pending(api, store, snum, None)

    report_anchor_test(store, snum, reference)


def report_anchor_test(store: Store, snum: int, reference: dict[str, tuple[str, int]]) -> None:
    errors = {"teammate": [], "opponent": []}
    drift: list[int] = []
    per_tier: dict[int, list[int]] = collections.defaultdict(list)
    sources = {
        src[len("anchor:"):]: [r[0] for r in store.db.execute("SELECT id FROM replays WHERE source = ? AND detail IS NOT NULL", (src,))]
        for (src,) in store.db.execute("SELECT DISTINCT source FROM replays WHERE season = ? AND source LIKE 'anchor:%'", (snum,))
    }
    for player, ids in sources.items():
        if player not in reference or not ids:
            continue
        ref_steps = reference[player][1]
        for _, detail in store.details(ids=ids):
            rows = player_rows(detail)
            me = next((r for r in rows if r.player == player), None)
            if not me:
                continue
            if me.rank_steps is not None:
                drift.append(me.rank_steps - ref_steps)
            for r in rows:
                if r is me or r.rank_steps is None:
                    continue
                err = r.rank_steps - ref_steps
                errors["teammate" if r.team == me.team else "opponent"].append(err)
                per_tier[ref_steps // 4].append(err)

    def summary(errs: list[int]) -> str:
        if not errs:
            return "no data"
        a = [abs(e) for e in errs]
        within = lambda k: 100 * sum(x <= k for x in a) / len(a)
        return (f"n={len(errs):5}  median |err| {statistics.median(a):.0f} div  "
                f"within 1 tier: {within(4):5.1f}%  within 2 tiers: {within(8):5.1f}%  bias {statistics.mean(errs):+.1f} div")

    print("\nAnchor test: lobby-mate true rank vs anchor reference rank (1 tier = 4 divisions)")
    print(f"  anchor's own rank drift  {summary(drift)}")
    print(f"  teammates                {summary(errors['teammate'])}")
    print(f"  opponents                {summary(errors['opponent'])}")
    print("  by anchor tier:")
    for t in sorted(per_tier):
        print(f"    {TIERS[t]:18} {summary(per_tier[t])}")


def cmd_prune(args, api: Ballchasing | None, store: Store) -> None:
    current = season_number(args.current) if args.current else None
    if current is None:
        latest = api.list_replays(playlist=PLAYLIST, count=1, sort_by="replay-date", sort_dir="desc")
        current = latest["list"][0]["season"]
    labeled = []
    for (s,) in store.db.execute("SELECT DISTINCT season FROM replays WHERE detail IS NOT NULL"):
        sample = [d for _, d in zip(range(20), store.details(s))]
        if any(r.rank_steps is not None for _, d in sample for r in player_rows(d)):
            labeled.append(s)
    keep = retained_seasons(current, labeled)
    drop = [s for (s,) in store.db.execute("SELECT DISTINCT season FROM replays") if s not in keep]
    log(f"current f{current}; keeping {sorted(f'f{s}' for s in keep)}; dropping {sorted(f'f{s}' for s in drop)}")
    if drop and not args.dry_run:
        q = ",".join("?" * len(drop))
        store.db.execute(f"DELETE FROM replays WHERE season IN ({q})", drop)
        store.db.execute(f"DELETE FROM anchors WHERE season IN ({q})", drop)
        store.commit()
        store.db.execute("VACUUM")


def main(argv: list[str] | None = None) -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--db", default=str(DEFAULT_DB))
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("status", help="Show API tier, current season and what is stored")

    c = sub.add_parser("collect", help="Queue and fetch tier-filtered games for a season")
    c.add_argument("--season", required=True, help="e.g. f22")
    c.add_argument("--per-tier", type=int, default=200)
    c.add_argument("--tiers", help="comma-separated subset, e.g. gold-2,diamond-2")
    c.add_argument("--windows", type=int, default=4, help="spread sampling across this many date windows")
    c.add_argument("--limit", type=int, help="fetch at most this many details this run")

    a = sub.add_parser("anchor-test", help="Validate anchor lobby labeling on a season with true ranks")
    a.add_argument("--season", required=True)
    a.add_argument("--anchors-per-tier", type=int, default=3)
    a.add_argument("--games-per-anchor", type=int, default=20)
    a.add_argument("--window-days", type=int, default=14)
    a.add_argument("--seed", type=int, default=7)

    e = sub.add_parser("export-rows", help="Write one CSV row per player-game")
    e.add_argument("--season", required=True)
    e.add_argument("--out", required=True)
    e.add_argument("--ranked-only", action="store_true")

    r = sub.add_parser("prune", help="Drop seasons outside n, n-1 and the newest labeled season")
    r.add_argument("--current", help="current season, e.g. f24 (default: detect)")
    r.add_argument("--dry-run", action="store_true")

    args = p.parse_args(argv)
    store = Store(Path(args.db))
    offline = args.cmd == "export-rows" or (args.cmd == "prune" and args.current)
    api = None if offline else Ballchasing(load_api_key())
    {
        "status": cmd_status,
        "collect": cmd_collect,
        "anchor-test": cmd_anchor_test,
        "export-rows": cmd_export_rows,
        "prune": cmd_prune,
    }[args.cmd](args, api, store)


if __name__ == "__main__":
    main()
