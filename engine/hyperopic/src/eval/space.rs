use crate::constants::{class, create_piece, reflect_side, side};
use crate::moves::Move;
use crate::node::{EvalFacet, Evaluation};
use crate::position::Position;
use crate::{Side, board};

const DEFAULT_SPACE_VALUE: i32 = 5;

#[derive(Debug, Clone)]
pub struct SpaceFacet {
    space_value: i32,
}

impl Default for SpaceFacet {
    fn default() -> Self {
        Self { space_value: DEFAULT_SPACE_VALUE }
    }
}

fn compute_space_count(position: &Position, side: Side) -> i32 {
    let enemy_side = reflect_side(side);
    let our_control = position.compute_control(side);
    let enemy_control = position.compute_control(reflect_side(side));
    let exclusive_enemy_control = enemy_control & !our_control;
    let friendly = position.side_boards[side];
    let enemies = position.side_boards[enemy_side];
    [class::N, class::B, class::R, class::Q]
        .iter()
        .map(|&class| create_piece(side, class))
        .flat_map(|piece| {
            board::iter(position.piece_boards[piece])
                .map(move |loc| {
                    board::board_moves(piece, loc, friendly, enemies) & !exclusive_enemy_control
                })
                .map(|board| board.count_ones() as i32)
        })
        .fold(0, |acc, space_count| acc + space_count)
}

impl EvalFacet for SpaceFacet {
    fn static_eval(&self, board: &Position) -> Evaluation {
        let space_diff = compute_space_count(board, side::W) - compute_space_count(board, side::B);
        let eval = self.space_value * space_diff;
        Evaluation::Single(eval)
    }

    fn make(&mut self, _mv: &Move, _board: &Position) {}

    fn unmake(&mut self, _mv: &Move) {}
}
