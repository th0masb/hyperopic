use crate::constants::square::*;
use crate::constants::{reflect_side, side};
use crate::position::Position;
use crate::test::{assert_boards_equal, reflect_board};
use crate::{Board, Side, Symmetric, board};

fn execute_test(fen: &str, side: Side, expected: Board) {
    let position: Position = fen.parse().unwrap();
    assert_boards_equal(expected, position.compute_control(side));
    assert_boards_equal(
        reflect_board(expected),
        position.reflect().compute_control(reflect_side(side)),
    );
}

#[test]
fn control_case_1() {
    execute_test(
        "4k3/8/8/3q4/8/8/3K4/3Q4 w - - 4 15",
        side::B,
        board!(
            ~E8 => F8, F7, E7, D7, D8;
            ~D5 => D1, A5, H5, D8, A2, H1, G8, A8
        ),
    );
}

#[test]
fn control_case_2() {
    execute_test(
        "4k3/8/8/3q4/1n6/8/3K4/3Q4 w - - 4 15",
        side::B,
        board!(
            ~E8 => F8, F7, E7, D7, D8;
            ~D5 => D1, A5, H5, D8, A2, H1, G8, A8;
            ~B4 => A6, A2, C2, D3, D5, C6
        ),
    );
}

#[test]
fn control_case_3() {
    execute_test(
        "4k3/p7/3p4/3q4/1n6/8/3K4/3Qn3 w - - 4 15",
        side::B,
        board!(
            ~E8 => F8, F7, E7, D7, D8;
            ~D5 => D1, A5, H5, D6, A2, H1, G8, A8;
            ~B4 => A6, A2, C2, D3, D5, C6;
            ~D6 => E5, C5;
            ~E8 => F8, F7, E7, D7, D8;
            ~A7 => B6
        ),
    );
}

#[test]
fn control_case_4() {
    execute_test(
        "4k3/p7/2np4/3q4/1n6/8/3K4/3Qn3 w - - 4 15",
        side::B,
        board!(
            ~E8 => F8, F7, E7, D7, D8;
            ~D5 => D1, A5, H5, D6, A2, H1, G8, C6;
            ~B4 => A6, A2, C2, D3, D5, C6;
            ~D6 => E5, C5;
            ~E8 => F8, F7, E7, D7, D8;
            ~C6 => D8, B8, A7, A5, B4, D4, E5, E7;
            ~A7 => B6
        ),
    );
}

#[test]
fn control_case_5() {
    execute_test(
        "4kb2/p7/2np4/3q4/1n6/8/3K4/3Qn3 w - - 4 15",
        side::B,
        board!(
            ~E8 => F8, F7, E7, D7, D8;
            ~D5 => D1, A5, H5, D6, A2, H1, G8, C6;
            ~B4 => A6, A2, C2, D3, D5, C6;
            ~D6 => E5, C5;
            ~E8 => F8, F7, E7, D7, D8;
            ~A7 => B6;
            ~C6 => D8, B8, A7, A5, B4, D4, E5, E7;
            ~F8 => D6, H6
        ),
    );
}

#[test]
fn control_case_6() {
    execute_test(
        "2b1kb2/p7/2np4/3q4/1n6/8/3K4/3Qn3 w - - 4 15",
        side::B,
        board!(
            ~E8 => F8, F7, E7, D7, D8;
            ~D5 => D1, A5, H5, D6, A2, H1, G8, C6;
            ~B4 => A6, A2, C2, D3, D5, C6;
            ~D6 => E5, C5;
            ~E8 => F8, F7, E7, D7, D8;
            ~A7 => B6;
            ~C6 => D8, B8, A7, A5, B4, D4, E5, E7;
            ~F8 => D6, H6;
            ~C8 => A6, H3
        ),
    );
}

#[test]
fn control_case_7() {
    execute_test("8/8/3p4/p3p2p/8/8/8/8 w - - 4 15", side::B, board!(B4, C5, E5, D4, F4, G4));
}

#[test]
fn control_case_8() {
    execute_test(
        "r1b1kb1R/p6p/2np4/1N1q4/1n5P/P1N1B1P1/2PK1P2/R2QnB1R w q - 4 15",
        side::B,
        board!(
            ~H7 => G6;
            ~F8 => D6, H6;
            ~E8 => F8, F7, E7, D7, D8;
            ~E1 => G2, F3, D3, C2;
            ~D6 => E5, C5;
            ~D5 => D1, H1, H5, G8, D6, C6, B5, A2;
            ~C8 => A6, H3;
            ~C6 => D8, B8, A7, A5, B4, D4, E5, E7;
            ~B4 => A6, A2, C2, D3, D5, C6;
            ~A8 => C8, A7;
            ~A7 => B6
        ),
    );
}
