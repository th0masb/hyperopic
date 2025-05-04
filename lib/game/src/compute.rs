use anyhow::Result;
use async_trait::async_trait;
use hyperopic::moves::Move;
use hyperopic::timing::TimeAllocator;
use hyperopic::{ComputeMoveInput, Engine};
use std::time::Duration;

#[async_trait]
pub trait MoveChooser {
    async fn choose(
        &self,
        moves_played: &str,
        remaining: Duration,
        increment: Duration,
    ) -> Result<Move>;
}

#[async_trait]
impl MoveChooser for Engine {
    async fn choose(
        &self,
        moves_played: &str,
        remaining: Duration,
        increment: Duration,
    ) -> Result<Move> {
        let position = moves_played.parse()?;
        tokio::task::block_in_place(|| {
            self.compute_move(ComputeMoveInput::new(
                position,
                remaining,
                increment,
                TimeAllocator::default(),
            ))
        })
        .map(|output| {
            match output.search_details {
                None => log::info!("Used move from lookup"),
                Some(details) => {
                    let formatted = serde_json::to_string(&details).unwrap_or("error".to_string());
                    log::info!("Computed: {}", formatted);
                }
            };
            output.best_move
        })
    }
}
