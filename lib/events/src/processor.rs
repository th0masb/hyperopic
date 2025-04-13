use crate::EventProcessor;
use crate::events::LichessEvent;
use crate::userstatus::StatusService;
use response_stream::LoopAction;

pub struct StreamLineProcessor<E: EventProcessor> {
    pub status_service: StatusService,
    pub event_processor: E,
}

impl<E: EventProcessor> StreamLineProcessor<E> {
    pub async fn handle_stream_read(&mut self, line: &str) -> LoopAction<()> {
        if line.is_empty() {
            self.user_status().await
        } else {
            match serde_json::from_str::<LichessEvent>(line) {
                Err(e) => log::warn!("Parse error: {} for \"{}\"", e, line),
                Ok(event) => {
                    log::info!("Received event: {}", line);
                    self.event_processor.process(event).await
                }
            };
            LoopAction::Continue
        }
    }

    async fn user_status(&mut self) -> LoopAction<()> {
        match self.status_service.user_status().await {
            Err(e) => {
                log::warn!("Error fetching user status: {}", e);
                LoopAction::Continue
            }
            Ok(None) => LoopAction::Continue,
            Ok(Some(status)) => {
                if status.online {
                    LoopAction::Continue
                } else {
                    LoopAction::Break(())
                }
            }
        }
    }
}
