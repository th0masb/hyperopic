use crate::TranspositionsImpl;
use crate::position::Position;
use crate::search::SearchParameters;
use crate::search::end::EmptyEndSignal;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::Instant;

#[rustfmt::skip]
/// Run on system76
/// ------------------------------------------------------------------------------------------------
/// 15/12/20 | 4(8)(2) | 100   | 0      | 737,690            | This is a control run on master to test
///          |         |       |        |                    | the addition of first attempt at
///          |         |       |        |                    | heuristically ordering all moves
///          |         |       |        |                    | according to their quality during the
///          |         |       |        |                    | negamax search.
/// ------------------------------------------------------------------------------------------------
/// 15/12/20 | 4(8)(2) | 100   | 0      | 182,397!!          | Massive difference in adding the move
///          |         |       |        |                    | ordering :)
/// ------------------------------------------------------------------------------------------------
///
/// Again on System76, I think the positions changed since last run but not too much difference
/// in the control run.
/// ------------------------------------------------------------------------------------------------
/// 21/12/20 | 4(8)(2) | 100   | 0      | 179,573            | This is a control run on master to test
///          |         |       |        |                    | the switch to negascout and the
///          |         |       |        |                    | accompanying shallow eval move ordering
/// ------------------------------------------------------------------------------------------------
/// 21/12/20 | 4(8)(2) | 100   | 0      | 17,769             | Order of magnitude quicker!
/// ------------------------------------------------------------------------------------------------
///
/// ------------------------------------------------------------------------------------------------
/// 31/12/20 | 4(8)(2) | 100   | 0      | 19,806             | This is a control run on master to test
///          |         |       |        |                    | the addition of transposition tables.
///          |         |       |        |                    | Slower likely to board API changes,
///          |         |       |        |                    | beefing up of Move enum and addition
///          |         |       |        |                    | of opening eval component.
/// ------------------------------------------------------------------------------------------------
/// 31/12/20 | 4(8)(2) | 100   | 0      | 15,544             | With 100,000 table entries
/// ------------------------------------------------------------------------------------------------
/// 31/12/20 | 4(8)(2) | 100   | 0      | 15,444             | With 1,000,000 table entries
/// ------------------------------------------------------------------------------------------------
/// 31/12/20 | 4(8)(2) | 500   | 0      | 80,161             | With 50,000 table entries
/// ------------------------------------------------------------------------------------------------
/// 31/12/20 | 4(8)(2) | 500   | 0      | 80,834             | With 200,000 table entries
/// ------------------------------------------------------------------------------------------------
/// 31/12/20 | 4(8)(2) | 500   | 0      | 80,410             | With 10,000 table entries
/// ------------------------------------------------------------------------------------------------
/// 19/07/23 | 4(*)(1) | 500   | 0      |  3,367             | 100,000 table entries
/// ------------------------------------------------------------------------------------------------
#[test]
#[ignore]
fn benchmark() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    let data_path = format!(
        "{}/{}",
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        std::env::var("MIDDLEGAME_INPUT_DATA").unwrap(),
    );
    let max_positions = std::env::var("MIDDLEGAME_MAX_CASES")?.parse::<usize>()?;
    let depth = std::env::var("MIDDLEGAME_DEPTH")?.parse::<usize>()?;
    let table_size = std::env::var("MIDDLEGAME_TABLE_SIZE")?.parse::<usize>()?;

    let positions = BufReader::new(File::open(&data_path)?)
        .lines()
        .take(max_positions)
        .map(|l| l.unwrap())
        .map(|l| match l.as_str().parse::<Position>() {
            Err(message) => panic!("{}", message),
            Ok(position) => position,
        })
        .collect::<Vec<_>>();

    let start = Instant::now();
    let mut best_moves = vec![];
    for (i, position) in positions.into_iter().enumerate() {
        if i % 5 == 0 {
            println!("[Position {}, Duration {}ms]", i, start.elapsed().as_millis());
        }
        best_moves.push(crate::search::search(position.into(), SearchParameters {
            end_signal: EmptyEndSignal,
            table: Arc::new(TranspositionsImpl::new(table_size)),
            max_depth: Some(depth as u8),
        })?)
    }
    println!("Successfully computed {} moves at depth {} in {}ms", best_moves.len(), depth, start.elapsed().as_millis());
    Ok(())
}
