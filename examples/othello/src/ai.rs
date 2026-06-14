// CPU AI for Othello: minimax with alpha-beta pruning over the bitboard.
//
// `Bitboard::play` already auto-handles passes, so a child node's `turn` may
// equal the parent's (when the opponent had to pass). We therefore decide
// max/min by comparing the node's turn against the CPU colour rather than
// alternating strictly by depth.

use crate::board::{Bitboard, Turn};

/// CPU difficulty levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Easy,
    Normal,
    Hard,
}

impl std::str::FromStr for Level {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "easy" => Ok(Level::Easy),
            "normal" => Ok(Level::Normal),
            "hard" => Ok(Level::Hard),
            _ => Err(format!("unknown level: {s}")),
        }
    }
}

impl Level {
    fn depth(self) -> u32 {
        match self {
            Level::Easy => 1,
            Level::Normal => 4,
            Level::Hard => 7,
        }
    }

    /// Empty-square count at or below which the *root* position switches to an
    /// exact endgame search (disc-difference eval, searched to the end). The
    /// decision is made once per `choose_move`, never inside the tree —
    /// extending mid-search would bolt a full endgame solve onto every
    /// depth-limited leaf, exploding the effective horizon to
    /// `depth + threshold` empties (minutes-to-hours of search).
    fn endgame_threshold(self) -> u32 {
        match self {
            Level::Easy => 0,
            Level::Normal => 8,
            Level::Hard => 12,
        }
    }
}

/// Classic positional weights, indexed by cell index (0 = top-left, row-major).
/// Corners +120, X-squares -40, C-squares -20, edges positive.
#[rustfmt::skip]
const WEIGHTS: [i32; 64] = [
    120, -20,  20,   5,   5,  20, -20, 120,
    -20, -40,  -5,  -5,  -5,  -5, -40, -20,
     20,  -5,  15,   3,   3,  15,  -5,  20,
      5,  -5,   3,   3,   3,   3,  -5,   5,
      5,  -5,   3,   3,   3,   3,  -5,   5,
     20,  -5,  15,   3,   3,  15,  -5,  20,
    -20, -40,  -5,  -5,  -5,  -5, -40, -20,
    120, -20,  20,   5,   5,  20, -20, 120,
];

/// Per-row positional-weight sums, precomputed at compile time:
/// `ROW_WEIGHTS[r][pattern]` is the sum of `WEIGHTS[r*8 + c]` over every column
/// `c` whose bit is set in `pattern`. Bit 7 of the pattern is column 0,
/// matching the board's `1u64 << (63 - i)` convention, so row `r` of a board
/// mask is exactly `mask.to_be_bytes()[r]`.
const ROW_WEIGHTS: [[i32; 256]; 8] = {
    let mut table = [[0i32; 256]; 8];
    let mut r = 0;
    while r < 8 {
        let mut pattern = 0;
        while pattern < 256 {
            let mut sum = 0;
            let mut c = 0;
            while c < 8 {
                if pattern & (0x80 >> c) != 0 {
                    sum += WEIGHTS[r * 8 + c];
                }
                c += 1;
            }
            table[r][pattern] = sum;
            pattern += 1;
        }
        r += 1;
    }
    table
};

/// Sum of `WEIGHTS` over the set bits of `mask`: one table lookup per row
/// instead of iterating set bits (no allocation, branch-free).
fn positional_sum(mask: u64) -> i32 {
    mask.to_be_bytes()
        .into_iter()
        .enumerate()
        .map(|(r, row)| ROW_WEIGHTS[r][row as usize])
        .sum()
}

/// Convert a single-bit mask to its cell index (0..64), matching the
/// `1u64 << (63 - i)` convention used by `Bitboard`.
#[inline]
fn bit_to_index(bit: u64) -> usize {
    (63 - bit.trailing_zeros()) as usize
}

/// Collect the set-bit indices of `mask`.
fn indices(mut mask: u64) -> Vec<usize> {
    let mut out = Vec::with_capacity(mask.count_ones() as usize);
    while mask != 0 {
        let bit = mask & mask.wrapping_neg();
        out.push(bit_to_index(bit));
        mask ^= bit;
    }
    out
}

fn empty_count(board: &Bitboard) -> u32 {
    64 - (board.black | board.white).count_ones()
}

fn disc_diff_for(board: &Bitboard, cpu: Turn) -> i32 {
    let black = board.black_count() as i32;
    let white = board.white_count() as i32;
    match cpu {
        Turn::Black => black - white,
        Turn::White => white - black,
    }
}

/// Legal-move count for `who` on this board, regardless of whose turn it is.
fn mobility_for(board: &Bitboard, who: Turn) -> i32 {
    if board.turn == who {
        board.legal.count_ones() as i32
    } else {
        let other = Bitboard::from_parts(board.black, board.white, who, false, false);
        other.legal.count_ones() as i32
    }
}

/// Static evaluation from `cpu`'s perspective. Higher = better for the CPU.
fn evaluate(board: &Bitboard, cpu: Turn, endgame: bool) -> i32 {
    if board.ended {
        let diff = disc_diff_for(board, cpu);
        return diff.signum() * 10_000 + diff;
    }

    if endgame {
        // Exact endgame: maximise the final disc difference.
        return disc_diff_for(board, cpu);
    }

    // Positional weight: own discs add, opponent discs subtract.
    let (own, opp) = match cpu {
        Turn::Black => (board.black, board.white),
        Turn::White => (board.white, board.black),
    };
    let positional = positional_sum(own) - positional_sum(opp);

    // Mobility: CPU legal-move count minus opponent legal-move count.
    let mobility = mobility_for(board, cpu) - mobility_for(board, cpu.opponent());

    positional + mobility * 15
}

/// Legal move indices, ordered by positional weight (best first) to improve
/// alpha-beta pruning efficiency.
fn ordered_moves(board: &Bitboard) -> Vec<usize> {
    let mut moves = indices(board.legal);
    moves.sort_by(|&a, &b| WEIGHTS[b].cmp(&WEIGHTS[a]));
    moves
}

/// Alpha-beta search. Returns the value of `board` from `cpu`'s perspective.
///
/// `endgame` is decided once at the root (see `choose_move`): when true, depth
/// is ignored and the search runs to the end of the game (recursion is bounded
/// by the remaining empty squares, ≤ the level's endgame threshold).
fn ab(
    board: &Bitboard,
    depth: u32,
    mut alpha: i32,
    mut beta: i32,
    cpu: Turn,
    endgame: bool,
) -> i32 {
    if board.ended {
        return evaluate(board, cpu, false);
    }
    if !endgame && depth == 0 {
        return evaluate(board, cpu, false);
    }

    let moves = ordered_moves(board);
    if moves.is_empty() {
        // Not ended but no moves: `play` already auto-passes, so treat as a
        // static node.
        return evaluate(board, cpu, endgame);
    }

    let next_depth = if endgame { depth } else { depth - 1 };
    let maximizing = board.turn == cpu;

    if maximizing {
        let mut best = i32::MIN;
        for m in moves {
            let child = board.play(m);
            let v = ab(&child, next_depth, alpha, beta, cpu, endgame);
            best = best.max(v);
            alpha = alpha.max(best);
            if beta <= alpha {
                break;
            }
        }
        best
    } else {
        let mut best = i32::MAX;
        for m in moves {
            let child = board.play(m);
            let v = ab(&child, next_depth, alpha, beta, cpu, endgame);
            best = best.min(v);
            beta = beta.min(best);
            if beta <= alpha {
                break;
            }
        }
        best
    }
}

/// Tiny xorshift RNG seeded from the wall clock — avoids a `rand` dependency.
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15)
            | 1;
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

/// Choose the CPU's move. Returns `None` if there are no legal moves.
pub fn choose_move(board: &Bitboard, level: Level) -> Option<usize> {
    let cpu = board.turn;
    let moves = ordered_moves(board);
    if moves.is_empty() {
        return None;
    }

    // Root-level mode decision: exact endgame solve vs depth-limited search.
    let endgame = empty_count(board) <= level.endgame_threshold();
    let depth = level.depth();

    // Score every root move with one ply already applied (the child's value
    // from the CPU's perspective).
    let mut scored: Vec<(usize, i32)> = moves
        .iter()
        .map(|&m| {
            let child = board.play(m);
            let v = ab(&child, depth, i32::MIN, i32::MAX, cpu, endgame);
            (m, v)
        })
        .collect();

    // Best-first.
    scored.sort_by_key(|&(_, v)| std::cmp::Reverse(v));

    match level {
        Level::Easy => {
            // Pick randomly among the top-2 moves for a bit of variety.
            let top = scored.len().min(2);
            let mut rng = Rng::new();
            Some(scored[rng.below(top)].0)
        }
        _ => Some(scored[0].0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Bitboard;

    #[test]
    fn each_level_returns_a_legal_opening_move() {
        let board = Bitboard::new();
        let legal = indices(board.legal);
        for level in [Level::Easy, Level::Normal, Level::Hard] {
            let mv = choose_move(&board, level).expect("a move must exist");
            assert!(
                legal.contains(&mv),
                "level {:?} returned illegal move {mv}",
                level
            );
        }
    }

    /// Fixed RNG seed so a failing playout is reproducible.
    const TEST_SEED: u64 = 0x9e37_79b9_7f4a_7c15;

    #[test]
    fn row_lut_matches_naive_weight_sum() {
        let mut rng = Rng(TEST_SEED);
        let masks = std::iter::once(0)
            .chain(std::iter::once(u64::MAX))
            .chain((0..1000).map(|_| rng.next_u64() & rng.next_u64()));
        for mask in masks {
            let naive: i32 = indices(mask).into_iter().map(|i| WEIGHTS[i]).sum();
            assert_eq!(positional_sum(mask), naive, "mismatch for mask {mask:#x}");
        }
    }

    #[test]
    fn random_vs_ai_playout_terminates() {
        let mut board = Bitboard::new();
        let mut rng = Rng(TEST_SEED);
        let mut guard = 0;
        while !board.ended {
            let mv = if board.turn == Turn::Black {
                let moves = indices(board.legal);
                moves[rng.below(moves.len())]
            } else {
                choose_move(&board, Level::Normal).unwrap()
            };
            let next = board.play(mv);
            assert_ne!(next, board, "play must make progress on a legal move");
            board = next;
            guard += 1;
            assert!(guard < 200, "playout did not terminate");
        }
        assert!(board.ended);
        assert!(board.black_count() + board.white_count() <= 64);
    }

    /// Regression guard for the endgame-extension explosion: with the old
    /// node-level endgame switch, Hard at ~20 empties effectively became an
    /// exact 19-empty solve and ran for minutes. Root-level switching keeps
    /// every Hard move in the tens of milliseconds.
    #[test]
    fn hard_is_fast_through_the_midgame_endgame_boundary() {
        for target_empties in [21u32, 20, 19, 18, 14, 13, 12] {
            let mut board = Bitboard::new();
            while !board.ended && empty_count(&board) > target_empties {
                let moves = ordered_moves(&board);
                board = board.play(moves[0]);
            }
            if board.ended {
                continue;
            }
            let t = std::time::Instant::now();
            let mv = choose_move(&board, Level::Hard);
            assert!(mv.is_some());
            assert!(
                t.elapsed() < std::time::Duration::from_secs(20),
                "hard search at {} empties took {:?}",
                empty_count(&board),
                t.elapsed()
            );
        }
    }

    #[test]
    fn hard_handles_sparse_endgame() {
        let mut board = Bitboard::new();
        let mut rng = Rng(TEST_SEED);
        let mut guard = 0;
        while !board.ended && empty_count(&board) > 10 {
            let moves = indices(board.legal);
            board = board.play(moves[rng.below(moves.len())]);
            guard += 1;
            if guard > 200 {
                break;
            }
        }
        if !board.ended {
            let mv = choose_move(&board, Level::Hard).expect("a move must exist");
            assert!(indices(board.legal).contains(&mv));
        }
    }
}
