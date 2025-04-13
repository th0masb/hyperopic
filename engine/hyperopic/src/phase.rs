use crate::ClassMap;
use crate::constants::{class, piece_class};
use crate::moves::Move;
use crate::node::Evaluation;
use crate::position::Position;

const MAX_PHASE: i32 = 256;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Phase {
    phase_values: ClassMap<i32>,
    phase_counter: i32,
    total_phase: i32,
    phase: i32,
}

impl Default for Phase {
    fn default() -> Self {
        let phase_values = [0i32, 1i32, 1i32, 2i32, 6i32, 0i32];
        Phase {
            phase_counter: 0,
            phase: 0,
            total_phase: 16 * phase_values[class::P]
                + 4 * (phase_values[class::N] + phase_values[class::B] + phase_values[class::R])
                + 2 * phase_values[class::Q],
            phase_values,
        }
    }
}

impl<'a> From<&'a Position> for Phase {
    fn from(value: &Position) -> Self {
        let mut phase = Phase::default();
        phase.phase_counter = phase.total_phase
            - (0..64)
                .flat_map(|sq| value.piece_locs[sq])
                .map(|p| phase.phase_values[piece_class(p)])
                .sum::<i32>();
        phase.update_phase();
        phase
    }
}

impl Phase {
    pub fn unwrap(&self, eval: Evaluation) -> i32 {
        match eval {
            Evaluation::Single(eval) => eval,
            Evaluation::Phased { mid, end } => self.interpolate(mid, end),
        }
    }

    pub fn phase_progression(&self) -> f32 {
        (self.phase as f32) / (MAX_PHASE as f32)
    }

    pub fn interpolate(&self, mid: i32, end: i32) -> i32 {
        ((mid * (MAX_PHASE - self.phase)) + end * self.phase) / MAX_PHASE
    }

    fn update_phase(&mut self) {
        self.phase = (self.phase_counter * MAX_PHASE + self.total_phase / 2i32) / self.total_phase;
    }

    pub fn make(&mut self, mv: &Move) {
        let counter_start = self.phase_counter;
        match mv {
            Move::Null | Move::Castle { .. } => {}
            Move::Enpassant { .. } => self.phase_counter += self.phase_values[class::P],
            Move::Normal { capture, .. } => {
                if let Some(piece) = capture {
                    self.phase_counter += self.phase_values[piece_class(*piece)]
                }
            }
            Move::Promote { promoted, .. } => {
                self.phase_counter += self.phase_values[class::P];
                self.phase_counter -= self.phase_values[piece_class(*promoted)];
            }
        }
        if self.phase_counter != counter_start {
            self.update_phase()
        }
    }

    pub fn unmake(&mut self, mv: &Move) {
        let counter_start = self.phase_counter;
        match mv {
            Move::Null | Move::Castle { .. } => {}
            Move::Enpassant { .. } => self.phase_counter -= self.phase_values[class::P],
            Move::Normal { capture, .. } => {
                if let Some(piece) = capture {
                    self.phase_counter -= self.phase_values[piece_class(*piece)]
                }
            }
            Move::Promote { promoted, .. } => {
                self.phase_counter -= self.phase_values[class::P];
                self.phase_counter += self.phase_values[piece_class(*promoted)];
            }
        }
        if self.phase_counter != counter_start {
            self.update_phase()
        }
    }
}
