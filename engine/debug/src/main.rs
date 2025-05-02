use clap::{Parser, Subcommand};
use itertools::Itertools;
use std::sync::Arc;

use hyperopic::moves::Moves;
use hyperopic::node::TreeNode;
use hyperopic::position::Position;
use hyperopic::search::end::EmptyEndSignal;
use hyperopic::search::{NodeType, SearchParameters, TableEntry, Transpositions};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    SearchPgn {
        #[arg(long)]
        pgn: String,
        #[arg(long)]
        depth: usize,
        #[arg(long, default_value_t = 100000)]
        table_size: usize,
    },
    SearchFen {
        #[arg(long)]
        fen: String,
        #[arg(long)]
        depth: usize,
        #[arg(long, default_value_t = 100000)]
        table_size: usize,
    },
    Moves {
        #[arg(long)]
        fen: String,
    },
}

fn main() {
    match Cli::parse().command {
        Commands::SearchPgn { pgn, depth, table_size } => {
            run_search(pgn.parse::<Position>().unwrap().into(), depth, table_size);
        }
        Commands::SearchFen { fen, depth, table_size } => {
            run_search(fen.parse::<Position>().unwrap().into(), depth, table_size);
        }
        Commands::Moves { fen } => {
            let board = fen.as_str().parse::<Position>().unwrap();
            let moves: Vec<_> =
                board.moves(&Moves::All).into_iter().map(|m| m.to_string()).collect();
            println!("{}", serde_json::to_string_pretty(&moves).unwrap());
        }
    }
}

struct DebugTranspositions {
    store: Vec<Option<(String, TableEntry)>>,
}

impl DebugTranspositions {
    pub fn new(size: usize) -> DebugTranspositions {
        DebugTranspositions { store: vec![None; size] }
    }
}

impl Transpositions for DebugTranspositions {
    fn get(&self, pos: &Position) -> Option<Arc<TableEntry>> {
        let index = (pos.key % self.store.len() as u64) as usize;
        if let Some((existing, n)) = self.store[index].as_ref() {
            if n.key == pos.key {
                let new_pos = to_table_id(&pos);
                if existing.as_str() != new_pos.as_str() {
                    panic!("Collision: {} <-> {}", existing, new_pos)
                }
                Some(Arc::new(n.clone()))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn put(&self, pos: &Position, root_index: u16, depth: u8, eval: i32, node_type: NodeType) {
        let index = (pos.key % self.store.len() as u64) as usize;
        let m = match &node_type {
            NodeType::Pv(path) => path.first().unwrap(),
            NodeType::Cut(m) => m,
            NodeType::All(m) => m,
        };
        if !pos.moves(&Moves::All).contains(m) {
            panic!("Bad node {} <-> {:?}", pos.to_string(), node_type)
        }
        let entry = TableEntry { key: pos.key, root_index, depth, eval, node_type };
        //self.store[index] = Some((to_table_id(&pos), entry))
        panic!("Do me")
    }

    fn reset(&self) {
        todo!()
    }
}

fn to_table_id(pos: &Position) -> String {
    pos.to_string().split_whitespace().take(4).join(" ")
}

fn run_search(mut state: TreeNode, depth: usize, table_size: usize) {
    if depth == 0 {
        println!("Static: {}", state.relative_eval());
        println!("Quiescent: {}", hyperopic::search::quiescent::full_search(&mut state).unwrap());
    } else {
        let outcome = hyperopic::search::search(
            state,
            SearchParameters {
                end_signal: EmptyEndSignal,
                table: Arc::new(DebugTranspositions::new(table_size)),
                max_depth: Some(depth as u8),
            },
        );
        println!("{}", serde_json::to_string_pretty(&outcome.unwrap()).unwrap());
    }
}
