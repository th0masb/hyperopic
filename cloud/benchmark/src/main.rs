use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use hyperopic::search::{SearchParameters, TranspositionsImpl};
use itertools::Itertools;
use lambda_runtime::{Context, Error, LambdaEvent, service_fn};
use simple_logger::SimpleLogger;

use lambda_payloads::benchmark::*;

mod positions;

const LOG_GAP: usize = 2;
const RUN_LOCALLY_VAR: &str = "RUN_LOCALLY";

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().with_level(log::LevelFilter::Info).without_timestamps().init()?;
    if let Ok(_) = std::env::var(RUN_LOCALLY_VAR) {
        let output = handler(LambdaEvent::new(
            BenchStartEvent { positions: 200, depth: 8, table_size: 100_000 },
            Context::default(),
        ))
        .await?;
        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    } else {
        lambda_runtime::run(service_fn(handler)).await
    }
}

async fn handler(event: LambdaEvent<BenchStartEvent>) -> Result<BenchOutput, Error> {
    let e = &event.payload;
    let positions = positions::get(e.positions);
    let n = positions.len();
    let start = Instant::now();
    let mut moves = vec![];
    let mut hasher = DefaultHasher::new();
    for (i, position) in positions.into_iter().enumerate() {
        if i % LOG_GAP == 0 {
            let cum_hash = hasher.finish();
            hasher = DefaultHasher::new();
            hasher.write_u64(cum_hash);
            log::info!(
                "[Position {}, Elapsed {}ms, Hash {}]",
                i,
                start.elapsed().as_millis(),
                cum_hash
            );
        }
        let search_result = hyperopic::search::search(
            position.into(),
            SearchParameters { end: e.depth, table: &mut TranspositionsImpl::new(e.table_size) },
        )?;
        search_result.best_move.hash(&mut hasher);
        moves.push(search_result);
    }

    let execution_times =
        moves.iter().map(|o| o.time.as_millis() as u64).sorted().collect::<Vec<_>>();

    let output = BenchOutput {
        depth_searched: e.depth,
        positions_searched: n,
        memory_allocated_mb: event.context.env_config.memory as usize,
        min_search_time_millis: execution_times[0],
        max_search_time_millis: execution_times[n - 1],
        median_search_time_millis: execution_times[n / 2],
        average_search_time_millis: execution_times.iter().sum::<u64>() / n as u64,
        total_search_time_secs: execution_times.iter().sum::<u64>() / 1000u64,
    };

    log::info!("{}", serde_json::to_string(&output)?);
    Ok(output)
}
