use std::cell::RefCell;

use itertools::Itertools;
use std::hash::Hasher;

use rustc_hash::FxHasher;

use crate::Board;
use crate::board::iter;
use crate::constants::boards::{ADJACENT_FILES, EMPTY, FILES, RANKS};
use crate::constants::{class, create_piece, lift, side, square_rank};
use crate::moves::Move;
use crate::node::{EvalFacet, Evaluation};
use crate::position::Position;

const WHITE_HALF: Board = RANKS[0] | RANKS[1] | RANKS[2] | RANKS[3];
const BLACK_HALF: Board = RANKS[4] | RANKS[5] | RANKS[6] | RANKS[7];

type Score = (i32, i32);

#[derive(Clone, PartialEq)]
struct CachedEval {
    whites: Board,
    blacks: Board,
    mid: i32,
    end: i32,
}

pub struct PawnStructureFacet {
    doubled_pawn_penalty: Score,
    isolated_pawn_penalty: Score,
    connected_passer_bonus: Score,
    passer_rank_bonuses: [Score; 6],
    cache: RefCell<Vec<Option<CachedEval>>>,
}

impl Default for PawnStructureFacet {
    fn default() -> Self {
        PawnStructureFacet {
            cache: RefCell::new(vec![None; 10000]),
            doubled_pawn_penalty: (-15, -25),
            isolated_pawn_penalty: (-10, -5),
            connected_passer_bonus: (60, 110),
            passer_rank_bonuses: [
                // Starting rank
                (0, 0),
                (10, 10),
                (20, 20),
                (40, 60),
                (80, 100),
                // Last rank before promotion
                (160, 200),
            ],
        }
    }
}

impl EvalFacet for PawnStructureFacet {
    fn static_eval(&self, board: &Position) -> Evaluation {
        let whites = board.piece_boards[create_piece(side::W, class::P)];
        let blacks = board.piece_boards[create_piece(side::B, class::P)];
        let mut cache_ref = self.cache.borrow_mut();
        let mut hasher = FxHasher::default();
        hasher.write_u64(whites);
        hasher.write_u64(blacks);
        let hash = hasher.finish();
        let index = (hash % cache_ref.len() as u64) as usize;
        let existing = cache_ref[index].as_ref();
        if let Some(entry) = existing {
            if entry.whites == whites && entry.blacks == blacks {
                return Evaluation::Phased { mid: entry.mid, end: entry.end };
            }
        }

        let (mid, end) = *&[
            self.evaluate_passed_pawns(whites, blacks),
            self.evaluate_doubled_pawns(whites, blacks),
            self.evaluate_isolated_pawns(whites, blacks),
        ]
        .iter()
        .fold((0, 0), |(am, ae), &(nm, ne)| (am + nm, ae + ne));

        cache_ref[index] = Some(CachedEval { whites, blacks, mid, end });
        Evaluation::Phased { mid, end }
    }

    fn make(&mut self, _: &Move, _: &Position) {}

    fn unmake(&mut self, _: &Move) {}
}

impl PawnStructureFacet {
    fn evaluate_passed_pawns(&self, whites: Board, blacks: Board) -> Score {
        let (w_passers, b_passers) = find_passed_pawns(whites, blacks);
        let (mut mid, mut end) = (0i32, 0i32);
        // Evaluate the rank rewards for advancing
        for i in 1..7 {
            let rank = RANKS[i];
            let w_count = (w_passers & rank).count_ones() as i32;
            let (w_mid, w_end) = self.passer_rank_bonuses[i - 1];
            let b_count = (b_passers & rank).count_ones() as i32;
            let (b_mid, b_end) = self.passer_rank_bonuses[6 - i];
            mid += w_count * w_mid - b_count * b_mid;
            end += w_count * w_end - b_count * b_end;
        }
        // Evaluate the connection rewards, only count connections if we are in the opponents half
        let (con_mid, con_end) = self.connected_passer_bonus;
        for i in 0..7 {
            let this_file = FILES[i];
            let next_file = FILES[i + 1];
            let w_count =
                count_connections(this_file & w_passers, next_file & w_passers & BLACK_HALF);
            let b_count =
                count_connections(this_file & b_passers, next_file & b_passers & WHITE_HALF);
            mid += (w_count - b_count) * con_mid;
            end += (w_count - b_count) * con_end;
        }
        (mid, end)
    }

    fn evaluate_doubled_pawns(&self, whites: Board, blacks: Board) -> Score {
        let doubled_count = count_doubled_pawns(whites, blacks);
        let (mid_pen, end_pen) = self.doubled_pawn_penalty;
        (mid_pen * doubled_count, end_pen * doubled_count)
    }

    fn evaluate_isolated_pawns(&self, whites: Board, blacks: Board) -> Score {
        let isolated_count = count_isolated_pawns(whites, blacks);
        let (mid_pen, end_pen) = self.isolated_pawn_penalty;
        (mid_pen * isolated_count, end_pen * isolated_count)
    }
}

fn count_connections(a: Board, b: Board) -> i32 {
    let mut count = 0;
    for sq_a in iter(a) {
        for sq_b in iter(b) {
            if (square_rank(sq_a) as i32 - square_rank(sq_b) as i32).abs() < 2 {
                count += 1;
            }
        }
    }
    count
}

fn find_passed_pawns(whites: Board, blacks: Board) -> (Board, Board) {
    let (mut passed_w, mut passed_b) = (EMPTY, EMPTY);
    for file_index in 0..8 {
        let file = FILES[file_index];
        let block_files = ADJACENT_FILES[file_index] | file;

        let last_black_def = iter(block_files & blacks).last().map(|s| square_rank(s)).unwrap_or(0);
        iter(file & whites)
            .filter(|s| square_rank(*s) >= last_black_def)
            .for_each(|sq| passed_w |= lift(sq));

        let last_white_def =
            iter(block_files & whites).next().map(|s| square_rank(s)).unwrap_or(10);
        iter(file & blacks)
            .filter(|s| square_rank(*s) <= last_white_def)
            .for_each(|sq| passed_b |= lift(sq));
    }
    (passed_w, passed_b)
}

#[cfg(test)]
mod test_passed {

    use super::*;
    use crate::board;
    use crate::constants::square::*;
    use crate::test::reflect_board;

    #[test]
    fn eval_1() {
        test_eval((160, 200), board!(B7), EMPTY)
    }

    #[test]
    fn eval_2() {
        test_eval((2 * 160 + 70, 2 * 200 + 120), board!(B7, C7), EMPTY)
    }

    #[test]
    fn eval_3() {
        test_eval((2 * 160 + 70 - 40, 2 * 200 + 120 - 60), board!(B7, C7, F4), board!(F5, G4))
    }

    fn test_eval(expected: Score, whites: Board, blacks: Board) {
        let mut f = PawnStructureFacet::default();
        f.connected_passer_bonus = (70, 120);
        let (mid, end) = expected;
        assert_eq!(expected, f.evaluate_passed_pawns(whites, blacks));
        assert_eq!(
            (-mid, -end),
            f.evaluate_passed_pawns(reflect_board(blacks), reflect_board(whites))
        );
    }

    #[test]
    fn count_connections_1() {
        assert_eq!(2, count_connections(board!(C2, C5), board!(B4, B5, B7)))
    }

    #[test]
    fn find_passers_1() {
        test_find_passers(
            board!(A4, E3, E5, G4),
            board!(B3, B6, D3, D4, E6, F7),
            EMPTY,
            board!(B3, D3),
        )
    }

    #[test]
    fn find_passers_2() {
        test_find_passers(board!(C4, C5, C6), board!(B5, D5), board!(C5, C6), EMPTY)
    }

    #[test]
    fn find_passers_3() {
        test_find_passers(
            board!(G2, E5, D4, B6, B4, B3),
            board!(H6, F5, E6, C6, A6),
            board!(B6),
            EMPTY,
        )
    }

    fn test_find_passers(
        whites: Board,
        blacks: Board,
        expected_white_passers: Board,
        expected_black_passers: Board,
    ) {
        test_find_passers_impl(whites, blacks, expected_white_passers, expected_black_passers);
        test_find_passers_impl(
            reflect_board(blacks),
            reflect_board(whites),
            reflect_board(expected_black_passers),
            reflect_board(expected_white_passers),
        )
    }

    fn test_find_passers_impl(
        whites: Board,
        blacks: Board,
        expected_white_passers: Board,
        expected_black_passers: Board,
    ) {
        let (actual_w, actual_b) = find_passed_pawns(whites, blacks);
        assert_eq!(expected_white_passers, actual_w);
        assert_eq!(expected_black_passers, actual_b);
    }
}

fn count_doubled_pawns(whites: Board, blacks: Board) -> i32 {
    let mut count = 0i32;
    for file_index in 0..8 {
        let file = FILES[file_index];
        count += count_doubling(file & whites);
        count -= count_doubling(file & blacks);
    }
    count
}

fn count_doubling(board: Board) -> i32 {
    iter(board)
        .tuple_windows::<(_, _)>()
        .filter(|(a, b)| square_rank(*b) == square_rank(*a) + 1)
        .count() as i32
}

fn count_isolated_pawns(whites: Board, blacks: Board) -> i32 {
    let mut count = 0i32;
    for file_index in 0..8 {
        let file = FILES[file_index];
        let adj_files = ADJACENT_FILES[file_index];
        if (adj_files & whites) == 0 {
            count += (file & whites).count_ones() as i32
        }
        if (adj_files & blacks) == 0 {
            count -= (file & blacks).count_ones() as i32
        }
    }
    count
}

#[cfg(test)]
mod simple_test {
    use crate::constants::boards::EMPTY;
    use crate::constants::square::*;
    use crate::eval::pawns::{count_doubled_pawns, count_isolated_pawns};
    use crate::test::reflect_board;
    use crate::{Board, board};

    fn execute_test(
        under_test: fn(Board, Board) -> i32,
        whites: Board,
        blacks: Board,
        expected_count: i32,
    ) {
        assert_eq!(under_test(whites, blacks), expected_count);
        assert_eq!(under_test(reflect_board(blacks), reflect_board(whites)), -expected_count);
    }

    #[test]
    fn doubled_case_0() {
        execute_test(count_doubled_pawns, EMPTY, EMPTY, 0)
    }

    #[test]
    fn doubled_case_1() {
        execute_test(count_doubled_pawns, board!(A4, A5, A7, B3, C2, C3), EMPTY, 2)
    }

    #[test]
    fn doubled_case_2() {
        execute_test(count_doubled_pawns, board!(A4, A5, A7, B3, C2, C3, D5, D6, D7), EMPTY, 4)
    }

    #[test]
    fn doubled_case_3() {
        execute_test(
            count_doubled_pawns,
            board!(A4, A5, A7, B3, C2, C3, D5, D6, D7),
            board!(C5, D6, H6, H7),
            3,
        )
    }

    #[test]
    fn isolated_case_0() {
        execute_test(count_isolated_pawns, EMPTY, EMPTY, 0)
    }

    #[test]
    fn isolated_case_1() {
        execute_test(count_isolated_pawns, board!(A5), EMPTY, 1)
    }

    #[test]
    fn isolated_case_2() {
        execute_test(count_isolated_pawns, board!(H6), EMPTY, 1)
    }

    #[test]
    fn isolated_case_3() {
        execute_test(count_isolated_pawns, board!(A3, B2, C4, D4), EMPTY, 0)
    }

    #[test]
    fn isolated_case_4() {
        execute_test(count_isolated_pawns, board!(A3, B2, C4, D4, F2, G2, H3), EMPTY, 0)
    }

    #[test]
    fn isolated_case_5() {
        execute_test(count_isolated_pawns, board!(A3, B2, C4, E4, G2, H3), EMPTY, 1)
    }

    #[test]
    fn isolated_case_6() {
        execute_test(count_isolated_pawns, board!(A3, B2, C4, E4, E5, G2, H3), EMPTY, 2)
    }

    #[test]
    fn isolated_case_7() {
        execute_test(count_isolated_pawns, board!(A3, B2, C4, E4, E5, G2, H3), board!(C7), 1)
    }
}

//
//fn count_backward_pawns(whites: BitBoard, blacks: BitBoard) -> i32 {
//    let mut count = 0i32;
//    for file_index in 1..7 {
//        let file = BitBoard::FILES[file_index];
//        let adj_files = ADJACENT_FILES[file_index];
//        if let Some(candidate) = (file & whites).first() {
//            let rank = candidate.rank_index();
//            let adj_rank = (adj_files & whites).first()
//                .map(|s| s.rank_index()).unwrap_or(10);
//            if adj_rank > rank {
//                count += 1
//            }
//        }
//        if let Some(candidate) = (file & blacks).iter().last() {
//            let rank = candidate.rank_index();
//            let adj_rank = (adj_files & blacks).iter().last()
//                .map(|s| s.rank_index()).unwrap_or(0);
//            if adj_rank < rank {
//                count -= 1
//            }
//        }
//    }
//    count
//}
//
//#[cfg(test)]
//mod backward_test {
//    use crate::Reflectable;
//    use super::*;
//    use crate::Square::*;
//
//    fn execute_test(whites: BitBoard, blacks: BitBoard, expected: i32) {
//        assert_eq!(count_backward_pawns(whites, blacks), expected);
//        assert_eq!(count_backward_pawns(blacks.reflect(), whites.reflect()), -expected);
//    }
//
//    #[test]
//    fn case_0() {
//        execute_test(
//            A2 | B2 | C2 | D2 | E2 | F2 | G2 | H2,
//            A7 | B7 | C7 | D7 | E7 | F7 | G7 | H7,
//            0
//        );
//    }
//
//    #[test]
//    fn case_1() {
//        execute_test(
//            C3 | D2 | E3 | F2 | G2 | H2,
//            A7 | B7 | C7 | D7 | E7 | F7 | G7 | H7,
//            1
//        );
//    }
//
//    #[test]
//    fn case_2() {
//        execute_test(
//            C3 | D2 | F2 | G2 | H2,
//            A7 | B7 | C7 | D7 | E7 | F7 | G7 | H7,
//            1
//        );
//    }
//
//    #[test]
//    fn case_3() {
//        execute_test(
//            A2 | C3 | D2 | F4 | G2,
//            C7 | D6 | E7 | F7 | G6 | H7,
//            1
//        );
//    }
//}
//
