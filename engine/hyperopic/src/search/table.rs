use crate::moves::Move;
use crate::position::Position;
use std::sync::{Arc, Mutex};
use NodeType::Pv;

pub trait Transpositions {
    fn get(&self, pos: &Position) -> Option<Arc<TableEntry>>;
    fn put(&self, pos: &Position, root_index: u16, depth: u8, eval: i32, node_type: NodeType);
    fn reset(&self);
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TableEntry {
    pub root_index: u16,
    pub key: u64,
    pub depth: u8,
    pub eval: i32,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NodeType {
    Pv(Vec<Move>),
    Cut(Move),
    All(Move),
}

pub struct ConcurrentTT {
    inner: Vec<Mutex<Option<Arc<TableEntry>>>>,
}

impl Transpositions for ConcurrentTT {
    fn get(&self, pos: &Position) -> Option<Arc<TableEntry>> {
        let index = self.index(pos.key);
        self.inner[index].lock().unwrap().as_ref().filter(|&e| e.key == pos.key).cloned()
    }

    fn put(&self, pos: &Position, root_index: u16, depth: u8, eval: i32, node_type: NodeType) {
        let index = self.index(pos.key);
        let mut curr_guard = self.inner[index].lock().unwrap();
        *curr_guard = Some(Arc::new(TableEntry { root_index, depth, eval, key: pos.key, node_type }));
    }

    fn reset(&self) {
        for row in self.inner.iter() {
            let mut p = row.lock().unwrap();
            *p = None;
        }
    }
}

impl ConcurrentTT {
    pub fn new(n_entries: usize) -> ConcurrentTT {
        let mut inner = Vec::with_capacity(n_entries);
        for _ in 0..n_entries {
            inner.push(Mutex::new(None));
        }
        ConcurrentTT { inner }
    }

    fn index(&self, k: u64) -> usize {
        (k % self.inner.len() as u64) as usize
    }
}
