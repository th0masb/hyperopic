use crate::position::{Position, TerminalState};

fn execute_test(expected: Option<TerminalState>, input: &str) {
    let board = input.parse::<Position>().unwrap();
    assert_eq!(expected, board.compute_terminal_state());
    //assert_eq!(expected, board.reflect().compute_terminal_state());
}

#[test]
fn checkmate() {
    execute_test(Some(TerminalState::Loss), "5R1k/pp2R2p/8/1b2r3/3p3q/8/PPB3P1/6K1 b - - 0 36");
}

#[test]
fn not_terminal() {
    execute_test(None, "r1b1qrk1/pp5p/1np2b2/3nNP2/3P2p1/1BN5/PP1BQ1P1/4RRK1 b - - 0 18");
}

#[test]
fn not_terminal2() {
    execute_test(None, "4R3/1p4rk/6p1/2p1BpP1/p1P1pP2/P7/1P6/K2Q4 b - - 0 2");
}

#[test]
fn not_terminal3() {
    execute_test(None, "2R2bk1/5p1p/5p1P/3N4/3K2P1/8/8/3r4 w - - 51 100");
}

#[test]
fn not_terminal4() {
    execute_test(None, "8/1p3B2/1n6/p3Pkp1/3P1pPp/1K3P1P/8/8 b - g3 0 41");
}

#[test]
fn stalemate() {
    execute_test(Some(TerminalState::Draw), "6k1/6p1/7p/8/1p6/p1qp4/8/3K4 w - - 0 45");
}

#[test]
fn fifty_moves_1() {
    execute_test(Some(TerminalState::Draw), "8/8/8/8/3B4/7K/2k1Q3/1q6 b - - 100 120")
}

#[test]
fn repetition_1() {
    execute_test(
        None,
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Nxe4 5. Re1 Nd6 6. Nxe5 Be7 \
        7. Bf1 Nxe5 8. Rxe5 O-O 9. d4 Ne8 10. d5",
    )
}

#[test]
fn repetition_2() {
    execute_test(
        None,
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Nxe4 5. Re1 Nd6 6. Nxe5 Be7 \
        7. Bf1 Nxe5 8. Rxe5 O-O 9. d4 Ne8 10. d5 Bc5",
    )
}

#[test]
fn repetition_3() {
    execute_test(
        None,
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Nxe4 5. Re1 Nd6 6. Nxe5 Be7 \
        7. Bf1 Nxe5 8. Rxe5 O-O 9. d4 Ne8 10. d5 Bc5 11. Be3",
    )
}

#[test]
fn repetition_4() {
    execute_test(
        None,
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Nxe4 5. Re1 Nd6 6. Nxe5 Be7 \
        7. Bf1 Nxe5 8. Rxe5 O-O 9. d4 Ne8 10. d5 Bc5 11. Be3 Be7",
    )
}

#[test]
fn repetition_5() {
    execute_test(
        None,
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Nxe4 5. Re1 Nd6 6. Nxe5 Be7 \
        7. Bf1 Nxe5 8. Rxe5 O-O 9. d4 Ne8 10. d5 Bc5 11. Be3 Be7 12. Bd2",
    )
}

#[test]
fn repetition_6() {
    execute_test(
        None,
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Nxe4 5. Re1 Nd6 6. Nxe5 Be7 \
        7. Bf1 Nxe5 8. Rxe5 O-O 9. d4 Ne8 10. d5 Bc5 11. Be3 Be7 12. Bd2 Bc5",
    )
}

#[test]
fn repetition_7() {
    execute_test(
        None,
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Nxe4 5. Re1 Nd6 6. Nxe5 Be7 \
        7. Bf1 Nxe5 8. Rxe5 O-O 9. d4 Ne8 10. d5 Bc5 11. Be3 Be7 12. Bd2 Bc5 12. Be3",
    )
}

#[test]
fn repetition_8() {
    execute_test(
        Some(TerminalState::Draw),
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Nxe4 \
        5. Re1 Nd6 6. Nxe5 Be7 7. Bf1 Nxe5 8. Rxe5 O-O 9. d4 Ne8 10. d5 Bc5 11. Be3 Be7 \
        12. Bd2 Bc5 13. Be3 Bb4 14. Bd2 Bc5 15. Be3",
    )
}

#[test]
fn repetition_9() {
    execute_test(
        Some(TerminalState::Draw),
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Nxe4 \
        5. Re1 Nd6 6. Nxe5 Be7 7. Bf1 Nxe5 8. Rxe5 O-O 9. d4 Ne8 10. d5 Bc5 11. Be3 Be7 \
        12. Bd2 Bc5 13. Be3 Bb4 14. Bd2 Bc5 15. Be3",
    )
}

#[test]
fn repetition_10() {
    execute_test(
        Some(TerminalState::Draw),
        "1. Nf3 Nf6 2. d4 g6 3. Bg5 Bg7 4. Nbd2 O-O 5. e4 d5 6. e5 Ne4 7. Nxe4 dxe4 \
        8. Nd2 Qxd4 9. Bxe7 Re8 10. Bf6 Bxf6 11. exf6 Qxb2 12. Bc4 e3 13. fxe3 Bg4 14. Qxg4 Qxa1+ \
        15. Qd1 Qxd1+ 16. Kxd1 Rxe3 17. Re1 Rxe1+ 18. Kxe1 Nd7 19. Ne4 Re8 20. Bd3 Nxf6 \
        21. Kf2 Nxe4+ 22. Bxe4 Rxe4 23. Kf3 Rc4 24. Ke3 Rxc2 25. Kd3 Rxg2 26. Kc4 Rxa2 \
        27. Kb3 Rxh2 28. Kc4 Kg7 29. Kd5 Kf6 30. Ke4 a5 31. Kd5 a4 32. Kc4 a3 33. Kb3 a2 \
        34. Ka3 Rc2 35. Kb3 Rh2 36. Ka3 Rc2 37. Kb3 Rh2",
    )
}

#[test]
fn repetition_11() {
    execute_test(
        Some(TerminalState::Draw),
        "1. e3 e6 2. Qf3 Nf6 3. Kd1 Nc6 4. d4 d5 5. Bb5 e5 6. Qg3 exd4 7. exd4 Ne4 8. Qe3 Be7 \
        9. f3 Nd6 10. Bxc6+ bxc6 11. h4 Nf5 12. Qc3 Bd7 13. h5 Ng3 14. Rh2 Nf1 15. Rh1 Ng3 16. Rh2 Nf1 17. Rh1 Ng3",
    )
}
