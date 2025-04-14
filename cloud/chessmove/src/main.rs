use std::time::Duration;

use lambda_runtime::{Error, LambdaEvent, service_fn};
use log;
use simple_logger::SimpleLogger;

use anyhow::anyhow;
use hyperopic::position::Position;
use hyperopic::{ComputeMoveInput, Engine, LookupMoveService};
use lambda_payloads::chessmove::*;
use lichess_api::LichessEndgameClient;
use openings::{DynamoOpeningService, OpeningTable};

const TABLE_SIZE: usize = 10000;
const TABLE_ENV_KEY: &'static str = "APP_CONFIG";

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().with_level(log::LevelFilter::Info).without_timestamps().init()?;
    lambda_runtime::run(service_fn(move_handler)).await?;
    Ok(())
}

async fn move_handler(event: LambdaEvent<ChooseMoveEvent>) -> Result<ChooseMoveOutput, Error> {
    let choose_move = &event.payload;
    let position = choose_move.moves_played.parse::<Position>()?;
    let mut engine = Engine::new(TABLE_SIZE, load_lookup_services(&choose_move.features));
    let output = engine.compute_move(ComputeMoveInput {
        position,
        remaining: Duration::from_millis(choose_move.clock_millis.remaining),
        increment: Duration::from_millis(choose_move.clock_millis.increment),
    }).recv().unwrap()?;
    Ok(ChooseMoveOutput {
        best_move: output.best_move.to_string(),
        search_details: output.search_details.map(|details| SearchDetails {
            depth_searched: details.depth as usize,
            search_duration_millis: details.time.as_millis() as u64,
            eval: details.relative_eval,
        }),
    })
}

fn load_lookup_services(features: &Vec<ChooseMoveFeature>) -> Vec<Box<dyn LookupMoveService>> {
    let mut services: Vec<Box<dyn LookupMoveService>> = vec![];
    if !features.contains(&ChooseMoveFeature::DisableOpeningsLookup) {
        let table_var = std::env::var(TABLE_ENV_KEY)
            .expect(format!("No value found for env var {}", TABLE_ENV_KEY).as_str());
        let service = serde_json::from_str::<OpeningTable>(table_var.as_str())
            .map_err(|e| anyhow!(e))
            .and_then(|table| DynamoOpeningService::try_from(table))
            .expect(format!("Could not parse table config {}", table_var).as_str());
        services.push(Box::new(service));
    }
    if !features.contains(&ChooseMoveFeature::DisableEndgameLookup) {
        services.push(Box::new(LichessEndgameClient::default()));
    }
    services
}
