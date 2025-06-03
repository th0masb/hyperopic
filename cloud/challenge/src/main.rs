use crate::config::{ChallengeEvent, UserConfig};
use itertools::Itertools;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use log::info;
use lichess_api::ratings::{ChallengeRequest, OnlineBot, TimeLimitType, UserDetailsGamePerf};
use lichess_api::LichessClient;
use rand::prelude::{IndexedRandom, IteratorRandom};
use simple_logger::SimpleLogger;

mod config;

const APP_CONFIG_VAR: &str = "APP_CONFIG";
const DEFAULT_RATING: u32 = 1750;

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().with_level(log::LevelFilter::Info).without_timestamps().init()?;
    lambda_runtime::run(service_fn(game_handler)).await
}

async fn game_handler(event: LambdaEvent<ChallengeEvent>) -> Result<(), Error> {
    let config: UserConfig = serde_json::from_str(std::env::var(APP_CONFIG_VAR)?.as_str())?;
    random_challenge_handler(&config, event.payload).await
}

async fn random_challenge_handler(config: &UserConfig, event: ChallengeEvent) -> Result<(), Error> {
    let mut rng = rand::rng();
    let time_limit =
        event.time_limit_options.choose(&mut rng).expect("No time limit options given!");

    let client = LichessClient::new(config.token.clone());

    let our_rating = client
        .fetch_rating(config.our_user_id.as_str(), time_limit.get_type())
        .await?
        .unwrap_or(UserDetailsGamePerf { rating: DEFAULT_RATING, prov: None });

    info!("Our Rating: {}", our_rating.rating);

    let time_limit_type = time_limit.get_type();
    let mut bots = client
        .fetch_online_bots()
        .await?
        .into_iter()
        .filter(|bot| bot.id != config.our_user_id)
        .filter(|bot| is_serious_bot(bot, time_limit_type))
        .collect_vec();

    info!("Found {} online bots", bots.len());

    bots.sort_by_key(|bot| bot.perfs.rating_for(time_limit_type).unwrap().rating);

    let ChallengeSplit { easier_count, harder_count } = compute_challenge_split(&event);

    let mut opponents = bots
        .iter()
        .filter(|b| b.perfs.rating_for(time_limit_type).unwrap().rating <= our_rating.rating)
        .rev()
        .take(easier_count as usize)
        .cloned()
        .collect_vec();

    opponents.extend(
        bots.iter()
            .filter(|b| b.perfs.rating_for(time_limit_type).unwrap().rating > our_rating.rating)
            .take(harder_count as usize)
            .cloned(),
    );

    let candidate_names = opponents.iter().map(|bot| bot.id.clone()).collect_vec();
    info!("Choosing opponents from {:?}", candidate_names);
    let chosen = opponents.iter().choose_multiple(&mut rng, event.challenge_count as usize);
    let chosen_names = chosen.iter().map(|bot| bot.id.clone()).collect_vec();
    info!("Chose opponents {:?}", chosen_names);

    for opponent in chosen {
        let (status, _) = client
            .create_challenge(ChallengeRequest {
                rated: event.rated,
                time_limit: time_limit.clone(),
                target_user_id: opponent.id.clone(),
            })
            .await?;
        info!("Response {} for challenge to {}", status, opponent.id.as_str());
    }
    Ok(())
}

fn is_serious_bot(bot: &OnlineBot, time_limit_type: TimeLimitType) -> bool {
    matches!(
        bot.perfs.rating_for(time_limit_type), 
        Some(UserDetailsGamePerf { prov: None, .. })
    )
}

#[derive(Debug, PartialEq, Clone)]
struct ChallengeSplit {
    pub easier_count: u32,
    pub harder_count: u32,
}

fn compute_challenge_split(event: &ChallengeEvent) -> ChallengeSplit {
    let harder_count = (event.sample_size as f64
        * (event.challenge_harder_percentage as f64 / 100.0))
        .round() as u32;

    ChallengeSplit {
        harder_count,
        easier_count: event.sample_size - harder_count
    }
}

#[cfg(test)]
mod test {
    use crate::config::ChallengeEvent;
    use super::{compute_challenge_split, ChallengeSplit};

    #[test]
    fn challenge_split() {
        let event = ChallengeEvent {
            time_limit_options: vec![],
            challenge_count: 10,
            sample_size: 20,
            challenge_harder_percentage: 10,
            rated: false,
        };
        assert_eq!(
            ChallengeSplit { easier_count: 18, harder_count: 2 },
            compute_challenge_split(&event)
        )
    }
}
