use std::sync::Arc;
use std::time::{Duration, Instant};

use lambda_runtime::{Error, LambdaEvent, service_fn};
use log;
use simple_logger::SimpleLogger;

use anyhow::anyhow;
use log::info;
use hyperopic::openings::OpeningService;
use hyperopic::position::Position;
use hyperopic::timing::TimeAllocator;
use hyperopic::{ComputeMoveInput, Engine, LookupMoveService};
use lambda_payloads::chessmove::*;
use lichess_api::LichessEndgameClient;
use openings::{DynamoOpeningClient, OpeningTable};

const DEFAULT_TABLE_SIZE: usize = 2_500_000;
const LATENCY_MILLIS: u64 = 200;
const TABLE_ENV_KEY: &'static str = "APP_CONFIG";

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().with_level(log::LevelFilter::Info).without_timestamps().init()?;
    lambda_runtime::run(service_fn(move_handler)).await?;
    Ok(())
}

async fn move_handler(event: LambdaEvent<ChooseMoveEvent>) -> Result<ChooseMoveOutput, Error> {
    let setup_start = Instant::now();
    let choose_move = &event.payload;
    let position = choose_move.moves_played.parse::<Position>()?;
    let table_size = choose_move.table_size.unwrap_or(DEFAULT_TABLE_SIZE);
    let engine = Engine::new(table_size, load_lookup_services(&choose_move.features));
    let input = ComputeMoveInput::new(
        position,
        Duration::from_millis(choose_move.clock_millis.remaining),
        Duration::from_millis(choose_move.clock_millis.increment),
        TimeAllocator::with_latency(Duration::from_millis(LATENCY_MILLIS)),
    );
    let setup_duration = setup_start.elapsed();
    info!("Setup time: {}ms", setup_duration.as_millis());
    let output = engine.compute_move(input)?;
    Ok(ChooseMoveOutput {
        best_move: output.best_move.to_string(),
        search_details: output.search_details.map(|details| SearchDetails {
            depth_searched: details.depth as usize,
            search_duration_millis: details.time.as_millis() as u64,
            eval: details.relative_eval,
        }),
    })
}

fn load_lookup_services(
    features: &Vec<ChooseMoveFeature>,
) -> Vec<Arc<dyn LookupMoveService + Send + Sync>> {
    let mut services: Vec<Arc<dyn LookupMoveService + Send + Sync>> = vec![];
    if !features.contains(&ChooseMoveFeature::DisableOpeningsLookup) {
        let table_var = std::env::var(TABLE_ENV_KEY)
            .expect(format!("No value found for env var {}", TABLE_ENV_KEY).as_str());
        let service = serde_json::from_str::<OpeningTable>(table_var.as_str())
            .map_err(|e| anyhow!(e))
            .and_then(|table| DynamoOpeningClient::try_from(table))
            .expect(format!("Could not parse table config {}", table_var).as_str());
        services.push(Arc::new(OpeningService::new(service)));
    }
    if !features.contains(&ChooseMoveFeature::DisableEndgameLookup) {
        services.push(Arc::new(LichessEndgameClient::default()));
    }
    services
}
