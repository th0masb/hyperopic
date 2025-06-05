use Move::Null;
use MoveFacet::{Attacking, Checking, Promoting};
use anyhow::Result;
use std::cmp::{max, min};

use crate::constants::{class, piece_class};
use crate::moves::Move::{Castle, Enpassant, Normal, Promote};
use crate::moves::{Move, MoveFacet, Moves};
use crate::node;
use crate::node::TreeNode;
use crate::position::TerminalState;

// Depth to which we also consider checking moves for quiescent search, >= 0 means disabled
const Q_CHECK_CAP: i32 = 0;
const DELTA_SKIP_MARGIN: i32 = 200;
const DELTA_SKIP_MAX_PHASE: f32 = 0.9;
const SHALLOW_MOVE_FACETS: [MoveFacet; 3] = [Attacking, Checking, Promoting];
const DEEP_MOVE_FACETS: [MoveFacet; 2] = [Attacking, Promoting];

pub fn full_search(node: &mut TreeNode) -> Result<i32> {
    search(node, -node::INFTY, node::INFTY)
}

pub fn search(node: &mut TreeNode, alpha: i32, beta: i32) -> Result<i32> {
    search_impl(node, alpha, beta, -1)
}

/// Performs a depth limited search looking to evaluate only quiet positions,
/// i.e. those with no attack moves.
fn search_impl(node: &mut TreeNode, mut alpha: i32, beta: i32, depth: i32) -> Result<i32> {
    // We know the start node not terminal otherwise wouldn't have entered the quiescent search
    if depth != -1 {
        match node.position().compute_terminal_state() {
            Some(TerminalState::Loss) => return Ok(max(alpha, min(beta, node::LOSS_VALUE))),
            Some(TerminalState::Draw) => return Ok(max(alpha, min(beta, node::DRAW_VALUE))),
            _ => {}
        }
    }
    // If we aren't in check then we can use the static eval as the initial
    // result under the sound assumption that there exists a move
    // (which might not be considered here) we can make in the position
    // which will improve our score. We cannot make this assumption if we
    // are in check because we will consider all the moves and so we
    // assume lost until proven otherwise.
    let in_check = node.position().in_check();
    let mut result = if in_check { -node::INFTY } else { node.relative_eval() };

    // Break immediately if the stand pat is greater than beta.
    if result >= beta {
        return Ok(beta);
    }
    if alpha < result {
        alpha = result;
    }

    let phase = node.phase_progression();

    for (category, m) in compute_quiescent_moves(node, in_check, depth) {
        match category {
            MoveCategory::Other | MoveCategory::Promotion { .. } => {}
            MoveCategory::BadExchange { .. } => {
                if !in_check {
                    continue;
                }
            }
            MoveCategory::GoodExchange { optimistic_delta, .. } => {
                if !in_check
                    && depth < Q_CHECK_CAP
                    && phase < DELTA_SKIP_MAX_PHASE
                    && result + optimistic_delta + DELTA_SKIP_MARGIN < alpha
                {
                    continue;
                }
            }
        };
        node.make(m)?;
        let next_result = -search_impl(node, -beta, -alpha, depth - 1)?;
        node.unmake()?;
        result = max(result, next_result);
        alpha = max(alpha, result);
        if alpha > beta {
            return Ok(beta);
        }
    }
    return Ok(result);
}

fn compute_quiescent_moves(
    node: &mut TreeNode,
    in_check: bool,
    depth: i32,
) -> Vec<(MoveCategory, Move)> {
    let moves_selector = if in_check {
        &Moves::All
    } else if depth < Q_CHECK_CAP {
        &Moves::AreAny(&DEEP_MOVE_FACETS)
    } else {
        &Moves::AreAny(&SHALLOW_MOVE_FACETS)
    };
    let mut moves: Vec<_> = node
        .position()
        .moves(moves_selector)
        .into_iter()
        .map(|mv| (categorise(node, &mv), mv))
        .collect();

    moves.sort_unstable_by_key(|(category, _)| -category.score());
    moves
}

fn categorise(state: &mut TreeNode, mv: &Move) -> MoveCategory {
    match mv {
        Null | Enpassant { .. } | Castle { .. } => MoveCategory::Other,
        Promote { promoted, capture, .. } => {
            let values = state.piece_values();
            MoveCategory::Promotion {
                optimistic_delta: values[piece_class(*promoted)] - values[class::P]
                    + capture.map(|p| values[piece_class(p)]).unwrap_or(0),
            }
        }
        Normal { from, dest, capture, .. } => match capture {
            None => MoveCategory::Other,
            Some(piece) => {
                let see = state.see(*from, *dest);
                if see <= 0 {
                    MoveCategory::BadExchange { see }
                } else {
                    MoveCategory::GoodExchange {
                        see,
                        optimistic_delta: state.piece_values()[piece_class(*piece)],
                    }
                }
            }
        },
    }
}

enum MoveCategory {
    BadExchange { see: i32 },
    Promotion { optimistic_delta: i32 },
    Other,
    GoodExchange { see: i32, optimistic_delta: i32 },
}

impl MoveCategory {
    fn score(&self) -> i32 {
        match self {
            MoveCategory::BadExchange { see } => *see,
            MoveCategory::Promotion { optimistic_delta } => 20000 + optimistic_delta,
            MoveCategory::Other => 5000,
            MoveCategory::GoodExchange { see, .. } => 20000 + see,
        }
    }
}
