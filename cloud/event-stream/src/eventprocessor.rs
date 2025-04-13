use async_trait::async_trait;

use lichess_events::EventProcessor;
use lichess_events::events::LichessEvent;

use crate::challenge::ChallengeService;
use crate::gamestart::GameStartService;

pub struct EventProcessorImpl {
    pub challenge_service: ChallengeService,
    pub gamestart_service: GameStartService,
}

#[async_trait]
impl EventProcessor for EventProcessorImpl {
    async fn process(&mut self, event: LichessEvent) {
        match event {
            LichessEvent::Challenge { challenge } => {
                match self.challenge_service.process_challenge(challenge).await {
                    Ok(message) => log::info!("Processed challenge with message: {}", message),
                    Err(error) => log::warn!("Error processing challenge: {}", error),
                }
            }
            LichessEvent::GameStart { game } => {
                match self.gamestart_service.process_event(game).await {
                    Ok(message) => log::info!("Processed gamestart with message: {}", message),
                    Err(error) => log::warn!("Error processing gamestart: {}", error),
                }
            }
        }
    }
}
