use crate::moves::Move;
use crate::node::TreeNode;
use crate::position::Position;
use crate::search::end::SearchEndSignal;
use crate::search::{SearchOutcome, SearchParameters, Transpositions, TranspositionsImpl};
use crate::timing::TimeAllocator;
use Ordering::SeqCst;
use anyhow::{Result, anyhow};
pub use board::union_boards;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use threadpool::ThreadPool;

mod board;
mod eval;
mod format;
mod hash;
pub mod moves;
pub mod node;
pub mod openings;
mod parse;
mod phase;
pub mod position;
pub mod search;
mod see;
#[cfg(test)]
mod test;
pub mod timing;
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
    ($( $x:expr ),*) => {
        {
            use crate::constants::lift;
            let mut board = 0u64;
            $(board |= lift($x);)*
            board
        }
    };
    // Cords inclusive of source
    ($( $x:expr => $($y:expr),+ );+) => {
        {
            use crate::board::compute_cord;
            let mut board = 0u64;
            $($(board |= compute_cord($x as usize, $y as usize);)+)+
            board
        }
    };
    // Cords exclusive of source
    ($( ~$x:expr => $($y:expr),+ );+) => {
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
    ($( $($x:expr),+ => $y:expr),+) => {
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

pub trait LookupMoveService {
    fn lookup(&self, position: Position) -> Result<Option<Move>>;
}

#[derive(Debug, Clone)]
pub struct ComputeMoveInput<E: SearchEndSignal + Clone> {
    /// The root position we want to search
    pub position: Position,
    /// The end signal for stopping the search
    pub search_end: E,
    /// The max depth on the search
    pub max_depth: Option<u8>,
    /// Flag which when set disables early return, i.e. in the case
    /// of a forced checkmate we wait for the end signal instead of
    /// returning the result immediately
    pub wait_for_end: bool,
}

impl ComputeMoveInput<Instant> {
    pub fn new(position: Position, remaining: Duration, inc: Duration, timing: TimeAllocator) -> Self {
        let position_count = position.history.len();
        ComputeMoveInput {
            position,
            search_end: Instant::now() + timing.allocate(position_count, remaining, inc),
            max_depth: None,
            wait_for_end: false,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ComputeMoveOutput {
    pub best_move: Move,
    pub search_details: Option<SearchOutcome>,
}

pub struct Engine {
    transpositions: Arc<TranspositionsImpl>,
    lookups: Vec<Arc<dyn LookupMoveService + Send + Sync>>,
    threads: ThreadPool,
    /// Flag ensuring at most one operation runs at any time
    available: Arc<AtomicBool>,
}

impl Engine {
    pub fn new(
        table_size: usize,
        lookups: Vec<Arc<dyn LookupMoveService + Send + Sync>>,
    ) -> Engine {
        Engine {
            transpositions: Arc::new(TranspositionsImpl::new(table_size)),
            lookups,
            threads: ThreadPool::new(1),
            available: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn reset(&self) -> bool {
        if self.available.compare_exchange(true, false, SeqCst, SeqCst).is_ok() {
            self.transpositions.reset();
            self.available.store(true, SeqCst);
            true
        } else {
            false
        }
    }

    pub fn compute_move<E>(&self, input: ComputeMoveInput<E>) -> Result<ComputeMoveOutput>
    where
        E: SearchEndSignal + Clone + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        if self.compute_move_async(input, move |r| tx.send(r).unwrap()) {
            rx.recv()?
        } else {
            Err(anyhow!("Engine unavailable, operation already running"))
        }
    }

    pub fn compute_move_async<E, F>(&self, input: ComputeMoveInput<E>, on_complete: F) -> bool
    where
        E: SearchEndSignal + Clone + Send + 'static,
        F: FnOnce(Result<ComputeMoveOutput>) -> () + Send + 'static,
    {
        if self.available.compare_exchange(true, false, SeqCst, SeqCst).is_err() {
            return false;
        }
        let lookups = self.lookups.clone();
        let transpositions = self.transpositions.clone();
        let available = self.available.clone();
        let search_end = input.search_end.clone();
        let max_depth = input.max_depth;
        let wait_for_end = input.wait_for_end;
        self.threads.execute(move || {
            let node: TreeNode = input.position.into();
            let output = match perform_lookups(lookups, node.position().clone()) {
                Some(mv) => Ok(ComputeMoveOutput { best_move: mv, search_details: None }),
                None => search::search(
                    node,
                    SearchParameters {
                        table: transpositions,
                        end_signal: search_end.clone(),
                        max_depth,
                    },
                )
                .map(|outcome| ComputeMoveOutput {
                    best_move: outcome.best_move.clone(),
                    search_details: Some(outcome),
                }),
            };
            if wait_for_end {
                // Wait until the search is meant to end, i.e. in case we have forced ending
                // and an infinite search has been requested.
                search_end.join();
            }
            // Make sure the engine is available again
            available.store(true, SeqCst);
            on_complete(output);
        });
        true
    }
}

fn perform_lookups(
    lookups: Vec<Arc<dyn LookupMoveService + Send + Sync>>,
    position: Position,
) -> Option<Move> {
    for service in lookups.iter() {
        if let Ok(Some(m)) = service.lookup(position.clone()) {
            return Some(m);
        }
    }
    None
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
