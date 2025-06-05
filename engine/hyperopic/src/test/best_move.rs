use crate::moves::Move;
use crate::node::TreeNode;
use crate::position::Position;
use crate::search::end::EmptyEndSignal;
use crate::search::{SearchParameters, TranspositionsImpl};
use crate::{Symmetric, node};
use std::sync::Arc;

const TABLE_SIZE: usize = 10_000;

fn test(position: &str, expected_move_pool: Vec<&str>, is_won: bool, depth: usize) {
    let position: Position = position.parse().unwrap();
    let parsed_moves: Vec<_> = expected_move_pool
        .into_iter()
        .map(|s| position.clone().play(s).unwrap().first().unwrap().clone())
        .collect();

    test_impl(position.clone().into(), parsed_moves.clone(), is_won, depth);
    let ref_board = position.reflect();
    let ref_move_pool = parsed_moves.into_iter().map(|m| m.reflect()).collect();
    test_impl(ref_board.into(), ref_move_pool, is_won, depth);
}

fn test_impl(board: TreeNode, expected_move_pool: Vec<Move>, is_won: bool, depth: usize) {
    let table = TranspositionsImpl::new(TABLE_SIZE);
    let params = SearchParameters {
        end_signal: EmptyEndSignal,
        table: Arc::new(table),
        max_depth: Some(depth as u8),
    };
    match crate::search::search(board, params) {
        Err(message) => panic!("{}", message),
        Ok(outcome) => {
            assert!(
                expected_move_pool.contains(&outcome.best_move),
                "{}",
                serde_json::to_string(&outcome).unwrap()
            );
            if is_won {
                assert_eq!(node::WIN_VALUE, outcome.relative_eval);
            }
        }
    }
}

#[test]
fn queen_escape_attack() {
    test(
        "r4rk1/5ppp/8/1Bn1p3/Q7/8/5PPP/1R3RK1 w Qq - 5 27",
        vec!["a4b4", "a4c4", "a4g4", "a4h4", "a4c2", "a4d1"],
        false,
        4,
    )
}

#[test]
fn mate_0() {
    test("r2r2k1/5ppp/1N2p3/1n6/3Q4/2B5/5PPP/1R3RK1 w Qq - 4 21", vec!["d4g7"], true, 4)
}

#[test]
fn mate_1() {
    test("8/8/8/4Q3/8/6R1/2n1pkBK/8 w - - 0 1", vec!["g3d3"], true, 4)
}

#[test]
fn mate_2() {
    test("8/7B/5Q2/6p1/6k1/8/5K2/8 w - - 0 1", vec!["f6h8", "f6f3"], true, 4)
}

#[test]
fn mate_3() {
    test("3qr2k/1b1p2pp/7N/3Q2b1/4P3/8/5PP1/6K1 w - - 0 1", vec!["d5g8"], true, 4)
}

#[test]
fn mate_4() {
    test("r1k2b1r/pp4pp/2p1n3/3NQ1B1/6q1/8/PPP2P1P/2KR4 w - - 4 20", vec!["e5c7"], true, 7)
}

#[test]
fn mate_5() {
    test("r1b1k1nr/p2p1ppp/n2B4/1p1NPN1P/6P1/3P1Q2/P1P1K3/q5b1 w - - 0 30", vec!["f5g7"], true, 5)
}

/// A funny one which currently depends on move ordering, at depth 3 the
/// best move has the same evaluation as another inferior move.
#[test]
fn tactic_1() {
    test("1r3k2/2R5/1p2p2p/1Q1pPp1q/1P1P2p1/2P1P1P1/6KP/8 b - - 2 31", vec!["b8a8"], false, 10)
}

#[test]
fn tactic_2() {
    test("r5k1/pb4pp/1pn1pq2/5B2/2Pr4/B7/PP3RPP/R4QK1 b - - 0 23", vec!["e6f5"], false, 4)
}

#[test]
fn prefer_castling() {
    test("1. e4 Nc6 2. Nf3 e5 3. Bb5 h6 4. a3 d6", vec!["e1g1"], false, 4)
}

#[test]
fn win_material() {
    test(
        "1. d4 d5 2. e3 Nf6 3. c4 c6 4. Nc3 e6 5. Bd3 dxc4 6. Bxc4 b5 7. Be2 Bd6 8. e4 b4 9. e5 bxc3 10. exf6 O-O 11. fxg7 cxb2",
        vec!["g7f8q", "g7f8r"],
        false,
        4,
    )
}

#[test]
fn enpassant_win_pawn() {
    test("8/6rk/p1p1p2p/1pPqPp2/1PNP4/1PQ5/5RPK/3b4 w - b6 0 49", vec!["c5b6"], false, 1)
}
