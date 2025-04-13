use crate::moves::Move;
use crate::node::TreeNode;
use crate::position::Position;
use crate::search::{SearchOutcome, SearchParameters, TranspositionsImpl};
use crate::timing::TimeAllocator;
use anyhow::Result;
pub use board::union_boards;
use std::time::{Duration, Instant};

mod board;
mod eval;
mod format;
mod hash;
pub mod moves;
pub mod node;
mod parse;
mod phase;
pub mod position;
pub mod search;
mod see;
#[cfg(test)]
mod test;
mod timing;
#[rustfmt::skip]
pub mod constants;
#[cfg(test)]
mod bench;

pub type Side = usize;
// H1 -> .. -> A1 -> H2 ... -> A8
pub type Square = usize;
pub type Rank = usize;
pub type File = usize;
pub type Board = u64;
pub type Class = usize;
pub type Piece = usize;
pub type Corner = usize;
pub type Dir = (isize, isize);

pub type SquareMap<T> = [T; 64];
pub type SquareMatrix<T> = SquareMap<SquareMap<T>>;
pub type SideMap<T> = [T; 2];
pub type ClassMap<T> = [T; 6];
pub type PieceMap<T> = [T; 12];
pub type CornerMap<T> = [T; 4];

#[macro_export]
macro_rules! board {
    // Individual squares
    ($( $x:expr_2021 ),*) => {
        {
            use crate::constants::lift;
            let mut board = 0u64;
            $(board |= lift($x);)*
            board
        }
    };
    // Cords inclusive of source
    ($( $x:expr_2021 => $($y:expr_2021),+ );+) => {
        {
            use crate::board::compute_cord;
            let mut board = 0u64;
            $($(board |= compute_cord($x as usize, $y as usize);)+)+
            board
        }
    };
    // Cords exclusive of source
    ($( ~$x:expr_2021 => $($y:expr_2021),+ );+) => {
        {
            use crate::board::compute_cord;
            use crate::constants::lift;
            let mut board = 0u64;
            $($(board |= compute_cord($x as usize, $y as usize) & !lift($x);)+)+
            board
        }
    };
}

#[macro_export]
macro_rules! square_map {
    ($( $($x:expr_2021),+ => $y:expr_2021),+) => {
        {
            use std::default::Default;
            let mut result = [Default::default(); 64];
            $($(result[$x as usize] = $y;)+)+
            result
        }
    };
}

pub trait Symmetric {
    fn reflect(&self) -> Self;
}

pub trait LookupMoveService: Send + Sync {
    fn lookup(&mut self, position: Position) -> Result<Option<Move>>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputeMoveInput {
    pub position: Position,
    pub remaining: Duration,
    pub increment: Duration,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ComputeMoveOutput {
    pub best_move: Move,
    pub search_details: Option<SearchOutcome>,
}

pub struct Engine {
    transpositions: TranspositionsImpl,
    lookups: Vec<Box<dyn LookupMoveService>>,
    timing: TimeAllocator,
}

impl Engine {
    pub fn new(table_size: usize, lookups: Vec<Box<dyn LookupMoveService>>) -> Engine {
        Engine {
            transpositions: TranspositionsImpl::new(table_size),
            lookups,
            timing: TimeAllocator::default(),
        }
    }

    pub fn compute_move(&mut self, input: ComputeMoveInput) -> Result<ComputeMoveOutput> {
        let start = Instant::now();
        let node: TreeNode = input.position.into();
        match self.perform_lookups(node.position().clone()) {
            Some(mv) => Ok(ComputeMoveOutput { best_move: mv, search_details: None }),
            None => {
                let position_count = node.position().history.len();
                search::search(
                    node,
                    SearchParameters {
                        table: &mut self.transpositions,
                        end: self.timing.allocate(
                            position_count,
                            input.remaining - start.elapsed(),
                            input.increment,
                        ),
                    },
                )
                .map(|outcome| ComputeMoveOutput {
                    best_move: outcome.best_move.clone(),
                    search_details: Some(outcome),
                })
            }
        }
    }

    fn perform_lookups(&mut self, position: Position) -> Option<Move> {
        for service in self.lookups.iter_mut() {
            if let Ok(Some(m)) = service.lookup(position.clone()) {
                return Some(m);
            }
        }
        None
    }
}

#[cfg(test)]
mod macro_test {
    use crate::constants::lift;

    use crate::constants::piece;
    use crate::constants::square::*;
    use crate::{Piece, SquareMap, board};

    #[test]
    fn board_macro() {
        assert_eq!(lift(A1) | lift(A2) | lift(B5), board!(A1, A2, B5));
        assert_eq!(lift(A1) | lift(A2) | lift(A3), board!(A1 => A3));
        assert_eq!(board!(C3, C2, C1, A3, B3), board!(C3 => A3, C1));
        assert_eq!(
            board!(C3, C2, C1, A3, B3, F2, E3, D4, C5, B6, G4, H6),
            board!(C3 => A3, C1; F2 => B6, H6),
        );
        assert_eq!(
            board!(C2, C1, A3, B3, E3, D4, C5, B6, G4, H6),
            board!(~C3 => A3, C1; ~F2 => B6, H6),
        );
    }

    #[test]
    fn square_map_macro() {
        let mut expected: SquareMap<Option<Piece>> = [None; 64];
        expected[F5] = Some(piece::WB);
        expected[A8] = Some(piece::WB);
        expected[D2] = Some(piece::BR);
        assert_eq!(expected, square_map!(F5, A8 => Some(piece::WB), D2 => Some(piece::BR)));
    }
}
