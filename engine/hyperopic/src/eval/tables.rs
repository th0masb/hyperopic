use crate::constants::{
    class, corner_side, create_piece, piece_class, piece_side, reflect_piece, reflect_square,
    square_file, square_rank,
};
use crate::{ClassMap, Piece, SideMap, Square, SquareMap, Symmetric};

use crate::moves::Move;
use crate::node::{EvalFacet, Evaluation};
use crate::position::{CASTLING_DETAILS, Position};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PieceSquareTablesFacet {
    tables: PositionTables,
    mid_eval: i32,
    end_eval: i32,
}

impl Default for PieceSquareTablesFacet {
    fn default() -> Self {
        PieceSquareTablesFacet { tables: PositionTables::default(), mid_eval: 0, end_eval: 0 }
    }
}

impl<'a> From<&'a Position> for PieceSquareTablesFacet {
    fn from(value: &Position) -> Self {
        let mut facet = PieceSquareTablesFacet::default();
        facet.mid_eval = facet.compute_midgame_eval(value);
        facet.end_eval = facet.compute_endgame_eval(value);
        facet
    }
}

type UpdateFn = fn(&mut PieceSquareTablesFacet, Piece, Square) -> ();

impl PieceSquareTablesFacet {
    pub fn compute_midgame_eval(&self, board: &Position) -> i32 {
        (0..64)
            .flat_map(|square| board.piece_locs[square].map(|p| (p, square)))
            .map(|(piece, square)| self.tables.midgame(piece, square))
            .sum()
    }

    pub fn compute_endgame_eval(&self, board: &Position) -> i32 {
        (0..64)
            .flat_map(|square| board.piece_locs[square].map(|p| (p, square)))
            .map(|(piece, square)| self.tables.endgame(piece, square))
            .sum()
    }

    fn add(&mut self, piece: Piece, square: Square) {
        self.mid_eval += self.tables.midgame(piece, square);
        self.end_eval += self.tables.endgame(piece, square);
    }

    fn remove(&mut self, piece: Piece, square: Square) {
        self.mid_eval -= self.tables.midgame(piece, square);
        self.end_eval -= self.tables.endgame(piece, square);
    }

    fn make_impl(&mut self, mv: &Move, add: UpdateFn, remove: UpdateFn) {
        match mv {
            Move::Null => {}
            &Move::Castle { corner } => {
                let details = &CASTLING_DETAILS[corner];
                let side = corner_side(corner);
                let rook = create_piece(side, class::R);
                let king = create_piece(side, class::K);
                remove(self, rook, details.rook_line.0);
                add(self, rook, details.rook_line.1);
                remove(self, king, details.king_line.0);
                add(self, king, details.king_line.1);
            }
            &Move::Normal { moving, from, dest, capture } => {
                remove(self, moving, from);
                add(self, moving, dest);
                if let Some(piece) = capture {
                    remove(self, piece, dest);
                }
            }
            &Move::Enpassant { side, from, dest, capture } => {
                let pawn = create_piece(side, class::P);
                remove(self, pawn, from);
                add(self, pawn, dest);
                remove(self, reflect_piece(pawn), capture);
            }
            &Move::Promote { from, dest, promoted, capture } => {
                remove(self, create_piece(piece_side(promoted), class::P), from);
                add(self, promoted, dest);
                if let Some(captured) = capture {
                    remove(self, captured, dest);
                }
            }
        }
    }
}

impl EvalFacet for PieceSquareTablesFacet {
    fn static_eval(&self, _: &Position) -> Evaluation {
        Evaluation::Phased { mid: self.mid_eval, end: self.end_eval }
    }

    fn make(&mut self, mv: &Move, _: &Position) {
        self.make_impl(mv, PieceSquareTablesFacet::add, PieceSquareTablesFacet::remove);
    }

    fn unmake(&mut self, mv: &Move) {
        self.make_impl(mv, PieceSquareTablesFacet::remove, PieceSquareTablesFacet::add);
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct PositionTables {
    tables: SideMap<ClassMap<SquareTable>>,
}

impl PositionTables {
    /// API method for indexing the midgame table, +ve better for white, -ve better for black
    pub fn midgame(&self, piece: Piece, location: Square) -> i32 {
        self.tables[piece_side(piece)][piece_class(piece)].0[location].0
    }

    /// API method for retrieving the evaluation for a piece at a given location
    /// in the midgame.
    pub fn endgame(&self, piece: Piece, location: Square) -> i32 {
        self.tables[piece_side(piece)][piece_class(piece)].0[location].1
    }
}

impl Default for PositionTables {
    fn default() -> Self {
        PositionTables {
            tables: [
                [
                    parse_full(PAWN),
                    parse_symmetric(KNIGHT),
                    parse_symmetric(BISHOP),
                    parse_symmetric(ROOK),
                    parse_symmetric(QUEEN),
                    parse_symmetric(KING),
                ],
                [
                    parse_full(PAWN).reflect(),
                    parse_symmetric(KNIGHT).reflect(),
                    parse_symmetric(BISHOP).reflect(),
                    parse_symmetric(ROOK).reflect(),
                    parse_symmetric(QUEEN).reflect(),
                    parse_symmetric(KING).reflect(),
                ],
            ],
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
struct SquareTable(SquareMap<(i32, i32)>);

impl Symmetric for SquareTable {
    fn reflect(&self) -> Self {
        SquareTable(std::array::from_fn(|sq| {
            let (mid, end) = self.0[reflect_square(sq)];
            (-mid, -end)
        }))
    }
}

type SymmetricTable = [(i32, i32); 32];
type CompleteTable = [(i32, i32); 64];

fn parse_symmetric(raw: SymmetricTable) -> SquareTable {
    SquareTable(std::array::from_fn(|sq| {
        let (rank, file) = (square_rank(sq), square_file(sq));
        let column = if file < 4 { file } else { 7 - file };
        raw[4 * rank + column]
    }))
}

fn parse_full(raw: CompleteTable) -> SquareTable {
    SquareTable(std::array::from_fn(|sq| raw[8 * square_rank(sq) + square_file(sq)]))
}

/// Tables lifted from stockfish here: https://github.com/official-stockfish/Stockfish/blob/master/src/psqt.cpp
/// They are (mid, end) values for white side on files H - E
#[rustfmt::skip]
const KNIGHT: SymmetricTable = [
    // Rank 1
    (-169, -105), (-96, -74), (-80, -46), (-79, -18),
    ( -79,  -70), (-39, -56), (-24, -15), ( -9,   6),
    ( -64,  -38), (-20, -33), (  4,  -5), ( 19,  27),
    ( -28,  -36), (  5,   0), ( 41,  13), ( 47,  34),
    ( -29,  -41), ( 13, -20), ( 42,   4), ( 52,  35),
    ( -11,  -51), ( 28, -38), ( 63, -17), ( 55,  19),
    ( -67,  -64), (-21, -45), (  6, -37), ( 37,  16),
    (-200,  -98), (-80, -89), (-53, -53), (-32, -16),
    // Rank 8
];

#[rustfmt::skip]
const BISHOP: SymmetricTable = [
    // Rank 1
    (-44, -63), ( -4, -30), (-11, -35), (-28,  -8),
    (-18, -38), (  7, -13), ( 14, -14), (  3,   0),
    ( -8, -18), ( 24,   0), ( -3,  -7), ( 15,  13),
    (  1, -26), (  8,  -3), ( 26,   1), ( 37,  16),
    ( -7, -24), ( 30,  -6), ( 23, -10), ( 28,  17),
    (-17, -26), (  4,   2), ( -1,   1), (  8,  16),
    (-21, -34), (-19, -18), ( 10,  -7), ( -6,   9),
    (-48, -51), ( -3, -40), (-12, -39), (-25, -20),
    // Rank 8
];

#[rustfmt::skip]
const ROOK: SymmetricTable = [
    // Rank 1
    (-24,  -2), (-13, -6), (-7,  -3), ( 2, -2),
    (-18, -10), (-10, -7), (-5,   1), ( 9,  0),
    (-21,  10), ( -7, -4), ( 3,   2), (-1, -2),
    (-13,  -5), ( -5,  2), (-4,  -8), (-6,  8),
    (-24,  -8), (-12,  5), (-1,   4), ( 6, -9),
    (-24,   3), ( -4, -2), ( 4, -10), (10,  7),
    ( -8,   1), (  6,  2), (10,  17), (12, -8),
    (-22,  12), (-24, -6), (-6,  13), ( 4,  7),
    // Rank 8
];

#[rustfmt::skip]
const QUEEN: SymmetricTable = [
    // Rank 1
    ( 3, -69), (-5, -57), (-5, -47), ( 4, -26),
    (-3, -55), ( 5, -31), ( 8, -22), (12,  -4),
    (-3, -39), ( 6, -18), (13,  -9), ( 7,   3),
    ( 4, -23), ( 5,  -3), ( 9,  13), ( 8,  24),
    ( 0, -29), (14,  -6), (12,   9), ( 5,  21),
    (-4, -38), (10, -18), ( 6, -12), ( 8,   1),
    (-5, -50), ( 6, -27), (10, -24), ( 8,  -8),
    (-2, -75), (-2, -52), ( 1, -43), (-2, -36),
    // Rank 8
];

#[rustfmt::skip]
const KING: SymmetricTable = [
    // Rank 1
    (272,   0), (325,  41), (273,  80), (190,  93),
    (277,  57), (305,  98), (241, 138), (183, 131),
    (198,  86), (253, 138), (168, 165), (120, 173),
    (169, 103), (191, 152), (136, 168), (108, 169),
    (145,  98), (176, 166), (112, 197), ( 69, 194),
    (122,  87), (159, 164), ( 85, 174), ( 36, 189),
    ( 87,  40), (120,  99), ( 64, 128), ( 25, 141),
    ( 64,   5), ( 87,  60), ( 49,  75), (  0,  75),
    // Rank 8
];

#[rustfmt::skip]
const PAWN: CompleteTable = [
    // Rank 1
    (  0,   0), (  0,   0), (  0,   0), (  0,   0), (  0,   0), (  0,   0), (  0,   0), ( 0,   0),
    ( -5, -19), (  7,  -5), ( 19,   7), (-20, -20), (-20, -20), ( 10,  10), (  3,  -6), ( 3, -10),
    (-22,  -4), (  5,  -6), ( 22,   3), ( 32,   4), ( 15,   4), ( 11, -10), (-15, -10), (-9, -10),
    (-12,  -9), (  4, -10), ( 17, -12), ( 40, -13), ( 20,  -4), (  6,  -8), (-23,  -2), (-8,   6),
    (  5,   8), (-13,  13), ( -2,  -6), ( 11, -12), (  1, -12), (-13,   3), (  0,   4), (13,   9),
    (-18,  13), (-15,   6), ( -5,   7), ( -8,  30), ( 22,  28), ( -7,  21), (-12,  20), (-5,  28),
    ( -8,   7), ( 10,   4), (-16,  19), (  5,  25), (-13,  21), ( -3,  12), (  7, -11), (-7,   0),
    (  0,   0), (  0,   0), (  0,   0), (  0,   0), (  0,   0), (  0,   0), (  0,   0), ( 0,   0),
    // Rank 8
];

#[cfg(test)]
mod test {
    use crate::constants::square::*;

    use crate::constants::create_piece;
    use crate::constants::{class, side};

    use super::{PieceSquareTablesFacet, PositionTables};
    use crate::node::EvalFacet;
    use crate::position::Position;

    #[test]
    fn zero_eval_for_start() {
        let pst = PieceSquareTablesFacet::from(&Position::default());
        assert_eq!(0, pst.mid_eval);
        assert_eq!(0, pst.end_eval);
    }

    #[test]
    fn test_midgame() {
        let tables = PositionTables::default();
        assert_eq!(-7, tables.midgame(create_piece(side::W, class::P), C6));
        assert_eq!(7, tables.midgame(create_piece(side::B, class::P), C3));
        assert_eq!(69, tables.midgame(create_piece(side::W, class::K), D5));
        assert_eq!(-69, tables.midgame(create_piece(side::B, class::K), D4));
    }

    #[test]
    fn test_endgame() {
        let tables = PositionTables::default();
        assert_eq!(21, tables.endgame(create_piece(side::W, class::P), C6));
        assert_eq!(-21, tables.endgame(create_piece(side::B, class::P), C3));
        assert_eq!(194, tables.endgame(create_piece(side::W, class::K), D5));
        assert_eq!(-194, tables.endgame(create_piece(side::B, class::K), D4));
    }

    #[test]
    fn test_evolution() {
        let pgn = "1. e4 c5 2. Nc3 Nc6 3. Nf3 e6 4. Bc4 d6 5. d4 cxd4 6. Nxd4 Nxd4 \
        7. Qxd4 Ne7 8. Bg5 Nc6 9. Qd2 f6 10. Be3 Be7 11. O-O-O Ne5 12. Be2 Bd7 13. Nb5 Bxb5 \
        14. Bxb5+ Nc6 15. Bc4 Qd7 16. Qe2 O-O 17. Qg4 f5 18. exf5 Ne5 19. Bxe6+ Qxe6 20. Qxg7+ Kxg7 \
        21. fxe6 Kf6 22. Rhe1 Kxe6 23. f4 Nc6 24. g4 Kf7 25. f5 Ne5 26. g5 Rfe8 27. f6 Bf8 \
        28. Bf4 Kg6 29. h4 a6 30. Bxe5 dxe5 31. Rd5 e4 32. Rd7 b6 33. Re2 h5 34. a3 Re6 \
        35. Kb1 Bd6 36. Rg7+ Kf5 37. f7 Kg4 38. Rh7 Bg3 39. g6 Be5 40. Rxe4+ Kf5 41. Rxe5+ Rxe5 \
        42. g7 Ree8 43. fxe8=Q Rxe8 44. Rh8 Re1+ 45. Ka2 Rg1 46. g8=Q Rxg8 47. Rxg8 Kf4 \
        48. Kb3 Kf5 49. Rg5+ Ke6 50. Rxh5 Kd7";

        let final_position: Position = pgn.parse().unwrap();
        let moves: Vec<_> = final_position.history.iter().map(|(_, m)| m.clone()).collect();
        let mut board = Position::default();
        let mut pst = PieceSquareTablesFacet::default();
        for m in moves {
            pst.make(&m, &board);
            board.make(m.clone()).unwrap();
            assert_eq!(PieceSquareTablesFacet::from(&board), pst);
            pst.unmake(&m);
            board.unmake().unwrap();
            assert_eq!(PieceSquareTablesFacet::from(&board), pst);
            pst.make(&m, &board);
            board.make(m.clone()).unwrap();
        }
    }
}
