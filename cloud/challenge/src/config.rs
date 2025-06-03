use lichess_api::ratings::TimeLimits;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UserConfig {
    #[serde(rename = "ourUserId")]
    pub our_user_id: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct ChallengeEvent {
    #[serde(rename = "timeLimitOptions")]
    pub time_limit_options: Vec<TimeLimits>,
    #[serde(rename = "challengeCount")]
    pub challenge_count: u32,
    #[serde(rename = "sampleSize")]
    pub sample_size: u32,
    #[serde(rename = "challengeHarderPercentage")]
    pub challenge_harder_percentage: u32,
    pub rated: bool
}
