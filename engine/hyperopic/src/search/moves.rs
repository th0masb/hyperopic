use crate::board::{control, cord, iter, union_boards};
use crate::constants::boards::{ADJACENT_FILES, FILES};
use crate::constants::{
    class, create_piece, in_board, lift, piece_class, piece_side, reflect_piece, reflect_side,
    side, side_parity, square_file, square_rank,
};
use crate::eval::tables::PositionTables;
use crate::moves::Move::{Castle, Enpassant, Normal, Null, Promote};
use crate::moves::{Move, Moves};
use crate::node::TreeNode;
use crate::position::{CASTLING_DETAILS, ConstrainedPieces, Position};
use crate::{Board, Class, Piece, Square};

#[derive(Default)]
pub struct MoveGenerator {
    estimator: MaterialAndPositioningHeuristic,
}

pub struct SearchMove {
    pub m: Move,
    pub is_attack: bool,
    pub is_check: bool,
    pub is_promoting: bool,
    pub is_passed_pawn: bool,
    pub is_positional_xray: bool,
}

impl SearchMove {
    pub fn is_tactical(&self) -> bool {
        self.is_attack
            || self.is_check
            || self.is_promoting
            || self.is_passed_pawn
            || self.is_positional_xray
    }
}

impl MoveGenerator {
    pub fn generate(&self, node: &TreeNode) -> Vec<SearchMove> {
        let pos = node.position();
        let enemy_king = create_piece(reflect_side(pos.active), class::K);
        let enemy_king_loc = pos.piece_boards[enemy_king].trailing_zeros() as usize;
        let discoveries = pos.compute_discoveries_on(enemy_king_loc).unwrap();
        let mut moves = node.position().moves(&Moves::All);
        moves.sort_by_cached_key(|m| self.estimator.estimate(node, m));
        let occupied = union_boards(&pos.side_boards);
        moves
            .into_iter()
            .map(|m| SearchMove {
                is_attack: is_attack(&m),
                is_check: is_checking(&m, &discoveries, enemy_king_loc, occupied),
                is_promoting: matches!(m, Promote { .. }),
                is_passed_pawn: is_passed_pawn(&m, pos),
                is_positional_xray: is_positional_xray(&m, pos),
                m,
            })
            .collect()
    }
}

fn is_positional_xray(m: &Move, pos: &Position) -> bool {
    match m {
        Null | Enpassant { .. } | Promote { .. } | Castle { .. } => false,
        Normal { moving, from, dest, capture, .. } => {
            let class = piece_class(*moving);
            let side = piece_side(*moving);
            capture.is_none()
                && (2..5).contains(&class)
                && !in_board(pos.passive_control, *dest)
                && {
                    let occupied = (union_boards(&pos.side_boards) | lift(*dest)) & !lift(*from);
                    let empty_control = control(*moving, *dest, 0);
                    let enemy_side = reflect_side(side);
                    let higher_value_locs = get_positional_xray_targets(class)
                        .iter()
                        .map(|&c| create_piece(enemy_side, c))
                        .fold(0u64, |a, n| a | pos.piece_boards[n])
                        & empty_control;
                    iter(higher_value_locs).any(|sq| (occupied & cord(*dest, sq)).count_ones() < 4)
                }
        }
    }
}

fn get_positional_xray_targets<'a>(class: Class) -> &'a [Class] {
    match class {
        class::B => &[class::R, class::Q, class::K],
        class::R => &[class::Q, class::K],
        class::Q => &[class::K],
        _ => panic!("{} not a valid piece class", class),
    }
}

#[cfg(test)]
mod test {
    use crate::Symmetric;
    use crate::constants::piece;
    use crate::constants::square::*;
    use crate::moves::Move;
    use crate::moves::Move::Normal;
    use crate::position::Position;
    use crate::search::moves::{is_passed_pawn, is_positional_xray};

    fn execute_test(pos: Position, m: Move, p: fn(&Move, &Position) -> bool, expected: bool) {
        let ref_p = pos.reflect();
        let ref_m = m.reflect();
        assert_eq!(p(&m, &pos), expected);
        assert_eq!(p(&ref_m, &ref_p), expected);
    }

    #[test]
    fn is_positional_xray_case_0() {
        execute_test(
            "rnbqk2r/pp3pp1/2n2b1p/8/3PN3/1Q3N2/PP3PPP/R3KB1R w KQkq - 1 11".parse().unwrap(),
            Normal { moving: piece::WB, from: F1, dest: B5, capture: None },
            is_positional_xray,
            true,
        )
    }

    #[test]
    fn is_positional_xray_case_1() {
        execute_test(
            "r1bqk2r/pp1n1pp1/2n2b1p/8/3PN3/1Q3N2/PP3PPP/R3KB1R w KQkq - 1 11".parse().unwrap(),
            Normal { moving: piece::WB, from: F1, dest: B5, capture: None },
            is_positional_xray,
            false,
        )
    }

    #[test]
    fn is_positional_xray_case_2() {
        execute_test(
            "rnbqk2r/pp3pp1/2p2b1p/8/3PN3/1Q3N2/PP3PPP/R3KB1R w KQkq - 1 11".parse().unwrap(),
            Normal { moving: piece::WB, from: F1, dest: B5, capture: None },
            is_positional_xray,
            false,
        )
    }

    #[test]
    fn is_passed_pawn_case_0() {
        execute_test(
            "rnbqk2r/pp3pp1/2p2b1p/8/2BPN3/1Q3N2/PP3PPP/R3K2R w KQkq - 2 11".parse().unwrap(),
            Normal { moving: piece::WP, from: D4, dest: D5, capture: None },
            is_passed_pawn,
            false,
        )
    }

    #[test]
    fn is_passed_pawn_case_1() {
        execute_test(
            "rnbqk2r/pp3pp1/5b1p/2p5/2BPN3/1Q3N2/PP3PPP/R3K2R w KQkq - 0 12".parse().unwrap(),
            Normal { moving: piece::WP, from: D4, dest: D5, capture: None },
            is_passed_pawn,
            true,
        )
    }

    #[test]
    fn is_passed_pawn_case_2() {
        execute_test(
            "rnbqk2r/pp3pp1/3p1b1p/2p5/2BPN3/1Q3N2/PP3PPP/R3K2R w KQkq - 0 12".parse().unwrap(),
            Normal { moving: piece::WP, from: D4, dest: D5, capture: None },
            is_passed_pawn,
            false,
        )
    }
}

fn is_passed_pawn(m: &Move, pos: &Position) -> bool {
    match m {
        Null | Promote { .. } => true,
        Castle { .. } | Enpassant { .. } => false,
        Normal { moving, dest, .. } => {
            piece_class(*moving) == class::P && {
                let file_index = square_file(*dest);
                let file = FILES[file_index];
                let adjacents = ADJACENT_FILES[file_index] | file;
                let enemy_pawns = pos.piece_boards[reflect_piece(*moving)] & adjacents;
                let rank_index = square_rank(*dest) as i32;
                let piece_side_parity = side_parity(piece_side(*moving));
                !iter(enemy_pawns)
                    .any(|sq| piece_side_parity * (square_rank(sq) as i32 - rank_index) > 0)
            }
        }
    }
}

fn is_attack(m: &Move) -> bool {
    match m {
        Null | Castle { .. } => false,
        Enpassant { .. } => true,
        Normal { capture, .. } => capture.is_some(),
        Promote { capture, .. } => capture.is_some(),
    }
}

fn is_checking(
    m: &Move,
    discoveries: &ConstrainedPieces,
    enemy_king: Square,
    occupied: Board,
) -> bool {
    match m {
        Null | Enpassant { .. } => false,
        Normal { moving, from, dest, .. } => {
            in_board(discoveries.1[*from], *dest)
                || in_board(control(*moving, *dest, occupied & !lift(*from)), enemy_king)
        }
        Promote { from, dest, promoted, .. } => {
            in_board(discoveries.1[*from], *dest)
                || in_board(control(*promoted, *dest, occupied & !lift(*from)), enemy_king)
        }
        Castle { corner } => {
            let details = &CASTLING_DETAILS[*corner];
            in_board(
                control(create_piece(side::W, class::R), details.rook_line.1, occupied),
                enemy_king,
            )
        }
    }
}

/// Main private of the heuristic move estimator trait,
/// it categorises moves into one of four subcategories from
/// best (good exchanges) to worst (bad exchanges) and then
/// also orders within those subcategories.
#[derive(Default)]
struct MaterialAndPositioningHeuristic {
    tables: PositionTables,
}

impl MaterialAndPositioningHeuristic {
    fn estimate(&self, board: &TreeNode, mv: &Move) -> i32 {
        match self.get_category(board, mv) {
            MoveCategory::GoodExchange(n) => 30_000 + n,
            MoveCategory::Special => 20_000,
            MoveCategory::Positional(n) => 10_000 + n,
            MoveCategory::BadExchange(n) => n,
        }
    }

    fn get_category(&self, eval: &TreeNode, mv: &Move) -> MoveCategory {
        match mv {
            Null | Enpassant { .. } | Castle { .. } | Promote { .. } => MoveCategory::Special,
            &Normal { moving, from, dest, capture } => {
                if capture.is_some() {
                    let exchange_value = eval.see(from, dest);
                    if exchange_value > 0 {
                        MoveCategory::GoodExchange(exchange_value)
                    } else {
                        MoveCategory::BadExchange(exchange_value)
                    }
                } else {
                    get_lower_value_delta(eval, moving, dest)
                        .map(|n| MoveCategory::BadExchange(n))
                        .unwrap_or_else(|| {
                            let side = piece_side(moving);
                            let from_value = self.tables.midgame(moving, from);
                            let dest_value = self.tables.midgame(moving, dest);
                            MoveCategory::Positional(side_parity(side) * (dest_value - from_value))
                        })
                }
            }
        }
    }
}

enum MoveCategory {
    // Wraps the see exchange value, > 0
    GoodExchange(i32),
    Special,
    // Wraps the position table value
    Positional(i32),
    // Wraps the see exchange value <= 0
    BadExchange(i32),
}

fn get_lower_value_delta(eval: &TreeNode, piece: Piece, dst: Square) -> Option<i32> {
    let piece_values = eval.piece_values();
    let p_class = piece_class(piece);
    let moving_value = piece_values[p_class];
    get_lower_value_pieces(p_class)
        .into_iter()
        .map(|&class| create_piece(reflect_side(piece_side(piece)), class))
        .filter(|p| in_board(compute_control(eval.position(), *p), dst))
        .map(|p| piece_values[piece_class(p)] - moving_value)
        .min()
}

fn get_lower_value_pieces<'a>(class: Class) -> &'a [Class] {
    match class {
        class::P => &[],
        class::N | class::B => &[class::P],
        class::R => &[class::P, class::N, class::B],
        class::Q => &[class::P, class::N, class::B, class::R],
        class::K => &[class::P, class::N, class::B, class::R, class::Q],
        _ => panic!("{} not a valid piece class", class),
    }
}

fn compute_control(board: &Position, piece: Piece) -> Board {
    let occupied = union_boards(&board.side_boards);
    iter(board.piece_boards[piece]).fold(0u64, |a, n| a | control(piece, n, occupied))
}
