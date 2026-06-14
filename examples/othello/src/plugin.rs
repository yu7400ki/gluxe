// Othello plugin: stateless game-logic commands.
//
// Board state round-trips through JS as opaque hex strings (BoardDto). Each
// command reconstructs a `Bitboard`, applies a transition, and returns a
// `StateView` JSON object that fully describes the resulting position for the
// renderer.

use serde::Deserialize;
use serde_json::{Value, json};

use gluxe::{Plugin, PluginBuilder, command, commands};

use crate::ai::{self, Level};
use crate::board::{Bitboard, Turn};

// ---------------------------------------------------------------------------
// DTO parsing
// ---------------------------------------------------------------------------

/// The `board` argument as it arrives from JS. Occupancy bitboards round-trip
/// as hex strings (serde can't decode those to `u64` directly), so the
/// hex/turn parsing is done in [`BoardDto::to_bitboard`].
#[derive(Deserialize)]
struct BoardDto {
    black: String,
    white: String,
    turn: String,
    #[serde(default)]
    passed: bool,
    #[serde(default)]
    ended: bool,
}

impl BoardDto {
    fn to_bitboard(&self) -> Result<Bitboard, String> {
        let black = parse_hex(&self.black, "board.black")?;
        let white = parse_hex(&self.white, "board.white")?;
        let turn = match self.turn.as_str() {
            "black" => Turn::Black,
            "white" => Turn::White,
            _ => return Err("`turn` must be \"black\" or \"white\"".to_string()),
        };
        Ok(Bitboard::from_parts(
            black,
            white,
            turn,
            self.passed,
            self.ended,
        ))
    }
}

fn parse_hex(s: &str, field: &str) -> Result<u64, String> {
    u64::from_str_radix(s, 16).map_err(|e| format!("`{field}` is not valid hex: {e}"))
}

// ---------------------------------------------------------------------------
// StateView construction
// ---------------------------------------------------------------------------

/// Per-cell occupancy: 0 empty, 1 black, 2 white. Index 0 = top-left.
fn cells_of(board: &Bitboard) -> [u8; 64] {
    let mut cells = [0u8; 64];
    for (i, cell) in cells.iter_mut().enumerate() {
        let bit = 1u64 << (63 - i);
        if board.black & bit != 0 {
            *cell = 1;
        } else if board.white & bit != 0 {
            *cell = 2;
        }
    }
    cells
}

fn legal_indices(board: &Bitboard) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 0..64 {
        let bit = 1u64 << (63 - i);
        if board.legal & bit != 0 {
            out.push(i);
        }
    }
    out
}

/// Build the JSON state view, optionally annotated with the move that produced
/// it (`last_move`) and the cells whose colour flipped as a result.
fn state_view(board: &Bitboard, last_move: Option<usize>, flipped: &[usize]) -> Value {
    let cells = cells_of(board);
    json!({
        "black": format!("{:016x}", board.black),
        "white": format!("{:016x}", board.white),
        "turn": match board.turn { Turn::Black => "black", Turn::White => "white" },
        "passed": board.passed,
        "ended": board.ended,
        "cells": cells.to_vec(),
        "legal": legal_indices(board),
        "blackCount": board.black_count(),
        "whiteCount": board.white_count(),
        "lastMove": last_move,
        "flipped": flipped,
    })
}

/// Compute which cells changed colour between `before` and `after`, excluding
/// the freshly placed cell at `placed`.
fn flipped_cells(before: &Bitboard, after: &Bitboard, placed: usize) -> Vec<usize> {
    let b0 = cells_of(before);
    let b1 = cells_of(after);
    let mut out = Vec::new();
    for i in 0..64 {
        if i == placed {
            continue;
        }
        // A flip turns one colour into the other (both non-empty, differing).
        if b0[i] != 0 && b1[i] != 0 && b0[i] != b1[i] {
            out.push(i);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// `new() -> StateView` for a fresh board.
#[command(name = "new")]
fn new() -> Result<Value, String> {
    Ok(state_view(&Bitboard::new(), None, &[]))
}

/// `play({ board, index }) -> StateView` after applying the move.
#[command(name = "play")]
fn play(board: BoardDto, index: usize) -> Result<Value, String> {
    let board = board.to_bitboard()?;

    let next = board.play(index);
    if next == board {
        // Move rejected (illegal / ended): report unchanged state, no last move.
        return Ok(state_view(&next, None, &[]));
    }

    let flipped = flipped_cells(&board, &next, index);
    Ok(state_view(&next, Some(index), &flipped))
}

/// `cpuMove({ board, level }) -> { move, state }` (async / background thread).
#[command(async, name = "cpuMove")]
fn cpu_move(board: BoardDto, level: String) -> Result<Value, String> {
    let board = board.to_bitboard()?;
    let level: Level = level.parse()?;

    match ai::choose_move(&board, level) {
        Some(mv) => {
            let next = board.play(mv);
            let flipped = flipped_cells(&board, &next, mv);
            Ok(json!({
                "move": mv,
                "state": state_view(&next, Some(mv), &flipped),
            }))
        }
        None => Ok(json!({
            "move": Value::Null,
            "state": state_view(&board, None, &[]),
        })),
    }
}

/// Build the Othello plugin.
pub fn plugin() -> Plugin {
    PluginBuilder::new("othello")
        .commands(commands![new, play, cpu_move])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rebuild a `BoardDto` from the `board`-shaped subset of a StateView, so
    /// tests can feed one command's output straight into the next.
    fn board_dto(view: &Value) -> BoardDto {
        serde_json::from_value(json!({
            "black": view["black"],
            "white": view["white"],
            "turn": view["turn"],
            "passed": view["passed"],
            "ended": view["ended"],
        }))
        .unwrap()
    }

    #[test]
    fn new_view_has_four_legal_moves() {
        let v = new().unwrap();
        assert_eq!(v["legal"].as_array().unwrap().len(), 4);
        assert_eq!(v["blackCount"], 2);
        assert_eq!(v["whiteCount"], 2);
        assert_eq!(v["turn"], "black");
    }

    #[test]
    fn play_d3_reports_one_flip_and_last_move() {
        let start = new().unwrap();
        let v = play(board_dto(&start), 19).unwrap();
        assert_eq!(v["lastMove"], 19);
        assert_eq!(v["flipped"].as_array().unwrap().len(), 1);
        assert_eq!(v["turn"], "white");
        assert_eq!(v["blackCount"], 4);
        assert_eq!(v["whiteCount"], 1);
    }

    #[test]
    fn illegal_play_reports_no_last_move() {
        let start = new().unwrap();
        let v = play(board_dto(&start), 0).unwrap();
        assert_eq!(v["lastMove"], Value::Null);
        assert_eq!(v["flipped"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn cpu_move_returns_legal_move() {
        let start = new().unwrap();
        let v = cpu_move(board_dto(&start), "normal".to_string()).unwrap();
        let mv = v["move"].as_u64().unwrap() as usize;
        let legal: Vec<usize> = start["legal"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect();
        assert!(legal.contains(&mv));
        assert_eq!(v["state"]["lastMove"], mv as u64);
    }
}
