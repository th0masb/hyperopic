use crate::config::{ChallengeEvent, KnownUserChallenge, UserConfig};
use itertools::Itertools;
use lambda_runtime::{Error, LambdaEvent, service_fn};
use lichess_api::LichessClient;
use lichess_api::ratings::{ChallengeRequest, TimeLimits};
use rand::prelude::{IndexedRandom};
use simple_logger::SimpleLogger;
use std::iter::repeat;
use std::time::Duration;
use tokio::time::sleep;

mod config;

const APP_CONFIG_VAR: &str = "APP_CONFIG";

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().with_level(log::LevelFilter::Info).without_timestamps().init()?;
    lambda_runtime::run(service_fn(game_handler)).await
}

async fn game_handler(event: LambdaEvent<ChallengeEvent>) -> Result<(), Error> {
    let config: UserConfig = serde_json::from_str(std::env::var(APP_CONFIG_VAR)?.as_str())?;
    match event.payload {
        ChallengeEvent::Specific { challenges } => {
            specific_challenge_handler(&config, challenges).await
        }
        ChallengeEvent::Random { time_limit_options, challenge_count, rated } => {
            random_challenge_handler(&config, time_limit_options, challenge_count, rated).await
        }
    }
}

async fn specific_challenge_handler(
    config: &UserConfig,
    challenges: Vec<KnownUserChallenge>,
) -> Result<(), Error> {
    let client = LichessClient::new(config.token.clone());
    for challenge in challenges {
        let target_id = challenge.user_id.as_str();
        let request = ChallengeRequest {
            rated: challenge.rated,
            time_limit: challenge.time_limits.clone(),
            target_user_id: target_id.to_string(),
        };
        for r in repeat(request).take(challenge.repeat) {
            let (status, _) = client.create_challenge(r).await?;
            if status.is_success() {
                log::info!("Successfully created challenge for {}", target_id);
            } else {
                log::error!("Status {} for challenge creation for {}", status, target_id);
            }
            sleep(Duration::from_secs(3)).await;
        }
    }
    Ok(())
}

async fn random_challenge_handler(
    config: &UserConfig,
    time_limit_options: Vec<TimeLimits>,
    challenge_count: usize,
    rated: bool,
) -> Result<(), Error> {
    let chosen_time_limit =
        time_limit_options.choose(&mut rand::rng()).expect("No time limit options given!");

    let client = LichessClient::new(config.token.clone());

    let our_rating = client
        .fetch_rating(config.our_user_id.as_str(), chosen_time_limit.get_type())
        .await?
        .unwrap();

    let mut bots = client
        .fetch_online_bots()
        .await?
        .into_iter()
        .filter(|b| b.id != config.our_user_id)
        .collect_vec();

    bots.sort_by_key(|bot| bot.perfs.rating_for(chosen_time_limit.get_type()).unwrap().rating);

    let lower_count = (challenge_count * 3) / 4;
    let upper_count = challenge_count - lower_count;

    let mut opponents = bots
        .iter()
        .filter(|b| {
            b.perfs.rating_for(chosen_time_limit.get_type()).unwrap().rating <= our_rating.rating
        })
        .rev()
        .take(lower_count)
        .collect::<Vec<_>>();

    opponents.extend(
        bots.iter()
            .filter(|b| {
                b.perfs.rating_for(chosen_time_limit.get_type()).unwrap().rating > our_rating.rating
            })
            .take(upper_count),
    );

    for opponent in opponents {
        let (status, _) = client
            .create_challenge(ChallengeRequest {
                rated,
                time_limit: chosen_time_limit.clone(),
                target_user_id: opponent.id.clone(),
            })
            .await?;
        log::info!("Response {} for challenge to {}", status, opponent.id.as_str());
    }
    Ok(())
}
