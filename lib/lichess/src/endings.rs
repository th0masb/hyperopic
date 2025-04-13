use std::time::{Duration, Instant};

use reqwest::blocking::{Client, Response};
use serde_derive::Deserialize;

use anyhow::{Result, anyhow};
use hyperopic::moves::Move;
use hyperopic::position::Position;
use hyperopic::{LookupMoveService, union_boards};

const TIMEOUT_MS: u64 = 1000;
const MAX_PIECE_COUNT: u32 = 7;
const TABLE_ENDPOINT: &'static str = "http://tablebase.lichess.ovh/standard";

#[derive(Default)]
pub struct LichessEndgameClient {
    client: Client,
}

impl LookupMoveService for LichessEndgameClient {
    fn lookup(&mut self, position: Position) -> Result<Option<Move>> {
        let query = position.to_string().replace(" ", "_");
        let piece_count = union_boards(&position.side_boards).count_ones();
        if piece_count > MAX_PIECE_COUNT {
            log::info!("Too many pieces to use endgame tables for {}", query);
            Ok(None)
        } else {
            let start = Instant::now();
            let response_result = self.execute_query(query.as_str());
            let query_duration = start.elapsed();
            log::info!("Endgame table query took {}ms", query_duration.as_millis());
            let raw_move = self.process_response(response_result?)?;
            position
                .clone()
                .play(&raw_move)?
                .first()
                .cloned()
                .ok_or(anyhow!("{} not parsed correctly on {}", raw_move, position))
                .map(|m| Some(m))
        }
    }
}

impl LichessEndgameClient {
    fn execute_query(&self, query: &str) -> Result<Response> {
        Ok(self
            .client
            .get(TABLE_ENDPOINT)
            .query(&[("fen", query)])
            .timeout(Duration::from_millis(TIMEOUT_MS))
            .send()?)
    }

    fn process_response(&self, resp: Response) -> Result<String> {
        let response_data = resp.json::<EndgameTableResponse>()?;
        response_data
            .moves
            .get(0)
            .map(|mv| mv.uci.clone())
            .ok_or(anyhow!("No suggested moves for given position"))
    }
}

#[derive(Deserialize)]
struct EndgameTableResponse {
    moves: Vec<SuggestedMove>,
}

#[derive(Deserialize)]
struct SuggestedMove {
    uci: String,
}
