//! A player's per-game stats across many replays, in the same units as the
//! bundled benchmarks, for coaching (#10) and playstyle (#9).

pub mod aggregate;
pub mod playstyle;

pub use aggregate::{player_game, stat_unit, GameStats, PlayerAggregate, Skipped};
pub use playstyle::{compute_playstyle, Axis, AxisScore, PlaystyleProfile};
