#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    Black,
    White,
}

impl Turn {
    pub fn opponent(self) -> Self {
        match self {
            Turn::Black => Turn::White,
            Turn::White => Turn::Black,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl Direction {
    fn mask(self) -> u64 {
        match self {
            Direction::Up | Direction::Down => 0x00ff_ffff_ffff_ff00,
            Direction::Left | Direction::Right => 0x7e7e_7e7e_7e7e_7e7e,
            Direction::UpLeft | Direction::UpRight | Direction::DownLeft | Direction::DownRight => {
                0x007e_7e7e_7e7e_7e00
            }
        }
    }

    fn shift(self, x: u64) -> u64 {
        match self {
            Direction::Up => x << 8,
            Direction::Down => x >> 8,
            Direction::Left => x << 1,
            Direction::Right => x >> 1,
            Direction::UpLeft => x << 7,
            Direction::UpRight => x << 9,
            Direction::DownLeft => x >> 9,
            Direction::DownRight => x >> 7,
        }
    }
}

const DIRECTIONS: [Direction; 8] = [
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
    Direction::UpLeft,
    Direction::UpRight,
    Direction::DownLeft,
    Direction::DownRight,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bitboard {
    pub black: u64,
    pub white: u64,
    pub legal: u64,
    pub turn: Turn,
    pub passed: bool,
    pub ended: bool,
}

impl Default for Bitboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Bitboard {
    pub fn new() -> Self {
        let mut board = Self {
            black: 0x0000_0008_1000_0000,
            white: 0x0000_0010_0800_0000,
            legal: 0,
            turn: Turn::Black,
            passed: false,
            ended: false,
        };

        board.update_legal();
        board
    }

    /// Additive constructor: rebuild a board from its persisted parts.
    ///
    /// `legal` and (potentially) `ended` are derived deterministically from the
    /// piece positions and turn, so callers only supply the persisted fields.
    /// This makes the private `update_legal`/end-detection reachable when a
    /// board is round-tripped through JSON without changing the game logic.
    pub fn from_parts(black: u64, white: u64, turn: Turn, passed: bool, ended: bool) -> Self {
        let mut board = Self {
            black,
            white,
            legal: 0,
            turn,
            passed,
            ended,
        };
        board.update_legal();
        board
    }

    pub fn play(&self, index: usize) -> Self {
        if self.ended || index >= 64 {
            return *self;
        }

        let pos = 1u64 << (63 - index);

        if self.legal & pos == 0 {
            return *self;
        }

        let mut next = *self;
        next.place(pos);
        next.advance_turn();
        next
    }

    pub fn black_count(self) -> u32 {
        self.black.count_ones()
    }

    pub fn white_count(self) -> u32 {
        self.white.count_ones()
    }

    fn own_and_opponent(self) -> (u64, u64) {
        match self.turn {
            Turn::Black => (self.black, self.white),
            Turn::White => (self.white, self.black),
        }
    }

    fn lookup(own: u64, opponent: u64, dir: Direction) -> u64 {
        let mask = opponent & dir.mask();

        let mut captured = mask & dir.shift(own);
        captured |= mask & dir.shift(captured);
        captured |= mask & dir.shift(captured);
        captured |= mask & dir.shift(captured);
        captured |= mask & dir.shift(captured);
        captured |= mask & dir.shift(captured);

        captured
    }

    fn flips_for(self, pos: u64) -> u64 {
        let (own, opponent) = self.own_and_opponent();

        DIRECTIONS.iter().copied().fold(0, |flips, dir| {
            let line = Self::lookup(pos, opponent, dir);

            if own & dir.shift(line) != 0 {
                flips | line
            } else {
                flips
            }
        })
    }

    fn place(&mut self, pos: u64) {
        let flips = self.flips_for(pos);

        match self.turn {
            Turn::Black => {
                self.black |= pos | flips;
                self.white ^= flips;
            }
            Turn::White => {
                self.white |= pos | flips;
                self.black ^= flips;
            }
        }

        self.passed = false;
    }

    fn update_legal(&mut self) {
        let blank = !(self.black | self.white);
        let (own, opponent) = self.own_and_opponent();

        self.legal = DIRECTIONS.iter().copied().fold(0, |legal, dir| {
            let line = Self::lookup(own, opponent, dir);
            legal | (blank & dir.shift(line))
        });
    }

    fn advance_turn(&mut self) {
        self.turn = self.turn.opponent();
        self.update_legal();

        if self.legal != 0 {
            return;
        }

        if self.passed {
            self.ended = true;
            return;
        }

        self.passed = true;
        self.turn = self.turn.opponent();
        self.update_legal();

        if self.legal == 0 {
            self.ended = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_has_four_legal_moves_for_black() {
        let board = Bitboard::new();
        assert_eq!(board.turn, Turn::Black);
        assert_eq!(board.legal.count_ones(), 4);
        assert_eq!(board.black_count(), 2);
        assert_eq!(board.white_count(), 2);
    }

    #[test]
    fn playing_d3_flips_one_disc() {
        // Index 19 = row 2, col 3 (d3) is one of the four opening moves.
        let board = Bitboard::new();
        let pos = 1u64 << (63 - 19);
        assert_ne!(board.legal & pos, 0, "index 19 must be legal");

        let next = board.play(19);
        // Black placed one disc and flipped exactly one white disc.
        assert_eq!(next.black_count(), 4);
        assert_eq!(next.white_count(), 1);
        assert_eq!(next.turn, Turn::White);
    }

    #[test]
    fn illegal_play_is_a_no_op() {
        let board = Bitboard::new();
        // Index 0 (a1 corner) is not legal from the opening.
        let next = board.play(0);
        assert_eq!(next, board);
    }

    #[test]
    fn from_parts_recomputes_legal() {
        let board = Bitboard::new();
        let rebuilt = Bitboard::from_parts(
            board.black,
            board.white,
            board.turn,
            board.passed,
            board.ended,
        );
        assert_eq!(rebuilt.legal, board.legal);
    }
}
