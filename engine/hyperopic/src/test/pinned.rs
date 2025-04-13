use crate::position::{ConstrainedPieces, Position};
use crate::{Square, Symmetric, board, square_map};

use crate::constants::reflect_square;
use crate::constants::square::*;
use anyhow::Result;

fn execute_pin_test(
    fen: &str,
    under_test: fn(&Position, Square) -> Result<ConstrainedPieces>,
    square: Square,
    expected: ConstrainedPieces,
) {
    let position = fen.parse::<Position>().unwrap();
    let reflected = position.reflect();
    assert_eq!(under_test(&position, square).unwrap(), expected);
    assert_eq!(under_test(&reflected, reflect_square(square)).unwrap(), expected.reflect());
}

#[test]
fn pinned_case_1() {
    execute_pin_test(
        "K2Q4/7p/1B4n1/2bq4/2rkp1R1/4p3/5br1/6B1 b KQkq - 5 10",
        Position::compute_pinned_on,
        D4,
        ConstrainedPieces(
            board!(E4, C5, D5),
            square_map!(
                E4 => board!(D4 => G4),
                C5 => board!(B6 => D4),
                D5 => board!(D4 => D8)
            ),
        ),
    );
}

#[test]
fn pinned_case_2() {
    execute_pin_test(
        "K2Q4/7p/1B4n1/2bq4/2rkp1R1/4p3/5br1/6B1 b KQkq - 5 10",
        Position::compute_pinned_on,
        E3,
        ConstrainedPieces(board!(F2), square_map!(F2 => board!(E3 => G1))),
    );
}

#[test]
fn discovery_1() {
    execute_pin_test(
        "6r1/5p1k/4pP2/4N3/3PN3/6P1/2B3PK/7R w - - 1 10",
        Position::compute_discoveries_on,
        H7,
        ConstrainedPieces(
            board!(E4, H2),
            square_map!(E4 => !board!(C2 => H7), H2 => !board!(H1 => H7)),
        ),
    )
}
