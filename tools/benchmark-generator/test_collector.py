"""Unit tests for pure collector logic. Run: python3 -m unittest test_collector.py"""

import unittest

from collector import TIERS, player_rows, rank_steps, retained_seasons, season_number


def detail(blue_goals=2, orange_goals=1, duration=300, ranks=("diamond-2", 14, 1)):
    rank = {"id": ranks[0], "tier": ranks[1], "division": ranks[2]}

    def player(n, start=0, end=duration):
        return {
            "id": {"platform": "epic", "id": f"p{n}"}, "name": f"P{n}", "rank": rank,
            "start_time": start, "end_time": end,
            "stats": {"core": {"goals": 1, "mvp": True}, "boost": {"bpm": 400.5}},
        }

    return {
        "id": "r1", "season": 22, "date": "2026-05-01T00:00:00Z", "duration": duration, "overtime": False,
        "blue": {"stats": {"core": {"goals": blue_goals}}, "players": [player(1), player(2)]},
        "orange": {"stats": {"core": {"goals": orange_goals}}, "players": [player(3), player(4, start=200)]},
    }


class RankSteps(unittest.TestCase):
    def test_division_scale(self):
        self.assertEqual(rank_steps({"tier": 1, "division": 1}), 0)
        self.assertEqual(rank_steps({"tier": 14, "division": 3}), 13 * 4 + 2)

    def test_missing_or_unranked(self):
        self.assertIsNone(rank_steps(None))
        self.assertIsNone(rank_steps({"tier": 0, "division": 1}))

    def test_bad_division_defaults_to_first(self):
        self.assertEqual(rank_steps({"tier": 2}), 4)

    def test_tier_list_matches_ballchasing_tier_numbers(self):
        self.assertEqual(TIERS[13], "diamond-2")  # tier 14
        self.assertEqual(len(TIERS), 22)


class Seasons(unittest.TestCase):
    def test_season_number(self):
        self.assertEqual(season_number("f22"), 22)
        with self.assertRaises(ValueError):
            season_number("14")

    def test_retention_keeps_n_n1_and_newest_labeled(self):
        self.assertEqual(retained_seasons(24, [18, 21, 22]), {24, 23, 22})

    def test_retention_when_current_is_labeled(self):
        self.assertEqual(retained_seasons(24, [24]), {24, 23})


class PlayerRows(unittest.TestCase):
    def test_context_and_flattening(self):
        rows = player_rows(detail())
        self.assertEqual(len(rows), 4)
        blue = rows[0]
        self.assertTrue(blue.won)
        self.assertEqual(blue.score_diff, 1)
        self.assertEqual(blue.player, "epic:p1")
        self.assertEqual(blue.rank_steps, 13 * 4)
        self.assertEqual(blue.teammate_rank_steps, 13 * 4)
        self.assertEqual(blue.stats, {"core_goals": 1, "boost_bpm": 400.5})  # bool mvp excluded

    def test_rank_id_normalized_from_tier(self):
        row = player_rows(detail(ranks=("grand-champion", 19, 2)))[0]
        self.assertEqual(row.rank_id, "grand-champion-1")

    def test_late_joiner_not_full_game(self):
        rows = player_rows(detail())
        self.assertFalse(rows[3].full_game)
        self.assertTrue(rows[2].full_game)

    def test_loss_side(self):
        orange = player_rows(detail())[2]
        self.assertFalse(orange.won)
        self.assertEqual(orange.score_diff, -1)


if __name__ == "__main__":
    unittest.main()


class Benchmarks(unittest.TestCase):
    def test_units(self):
        from benchmarks import stat_unit
        self.assertEqual(stat_unit("core_goals"), "per_minute")
        self.assertEqual(stat_unit("positioning_percent_behind_ball"), "as_is")
        self.assertEqual(stat_unit("positioning_avg_distance_to_ball"), "as_is")
        self.assertEqual(stat_unit("boost_bpm"), "as_is")
        self.assertEqual(stat_unit("core_shooting_percentage"), "as_is")

    def test_time_stats_dropped_only_with_percent_counterpart(self):
        from benchmarks import redundant
        names = {"movement_time_ground", "movement_percent_ground", "movement_time_powerslide"}
        self.assertTrue(redundant("movement_time_ground", names))
        self.assertFalse(redundant("movement_time_powerslide", names))

    def test_distribution_quantiles(self):
        from benchmarks import distribution
        d = distribution([float(x) for x in range(1, 102)])  # 1..101
        self.assertEqual(d["p50"], 51)
        self.assertEqual(d["p5"], 6)
        self.assertEqual(d["p95"], 96)
        self.assertEqual(d["n"], 101)
        self.assertIsNone(distribution([1.0]))

    def test_rank_signal_direction(self):
        from benchmarks import rank_signal
        pairs = [(steps, {"s": float(steps) + (steps % 3)}) for steps in range(60)]
        sig = rank_signal(pairs, "s")
        self.assertEqual(sig["direction"], "higher")
        self.assertGreater(sig["weight"], 0.9)
        self.assertIsNone(rank_signal(pairs[:10], "s"))


class Anchors(unittest.TestCase):
    def test_parse_rank_forms(self):
        from anchors import parse_rank
        d2 = TIERS.index("diamond-2")
        for text in ["diamond-2", "Diamond 2", "Diamond II", "  diamond  ii ", "D2", "d2"]:
            self.assertEqual(parse_rank(text), d2, text)
        self.assertEqual(parse_rank("GC1"), TIERS.index("grand-champion-1"))
        self.assertEqual(parse_rank("Grand Champion III"), TIERS.index("grand-champion-3"))
        self.assertEqual(parse_rank("C3"), TIERS.index("champion-3"))
        self.assertEqual(parse_rank("SSL"), TIERS.index("supersonic-legend"))
        self.assertEqual(parse_rank("Supersonic Legend"), TIERS.index("supersonic-legend"))
        for bad in ["", "diamond", "diamond 4", "platinum-9", "legend"]:
            self.assertIsNone(parse_rank(bad), bad)

    def test_tier_position_interpolates_and_clamps(self):
        from anchors import _tier_position
        rising = [(10, 100.0), (11, 200.0), (12, 300.0)]
        self.assertEqual(_tier_position(150.0, rising), 10.5)
        self.assertEqual(_tier_position(50.0, rising), 10.0)
        self.assertEqual(_tier_position(999.0, rising), 12.0)
        falling = [(10, 30.0), (11, 20.0), (12, 10.0)]
        self.assertEqual(_tier_position(15.0, falling), 11.5)

    def test_tracker_urls(self):
        from anchors import tracker_url
        self.assertTrue(tracker_url("steam", "7656119", "x").endswith("/steam/7656119/overview"))
        self.assertTrue(tracker_url("epic", "abc", "Some Name").endswith("/epic/Some%20Name/overview"))
        self.assertIn("/psn/", tracker_url("ps4", "1", "n"))
        self.assertIn("/xbl/", tracker_url("xbox", "1", "n"))
        self.assertEqual(tracker_url("psynet", "1", "n"), "")
