use serde_derive::Deserialize;
use std::time::Instant;

use anyhow::{Error, Result, anyhow};
use tokio::time::Duration;

const STATUS_ENDPOINT: &'static str = "https://lichess.org/api/users/status";

pub struct StatusService {
    client: StatusClient,
    status_poll_gap: Duration,
    status_checkpoint: Instant,
    user_id: String,
}

impl StatusService {
    pub fn new(our_bot_id: &str, status_poll_frequency: Duration) -> StatusService {
        StatusService {
            client: StatusClient::default(),
            status_poll_gap: status_poll_frequency,
            status_checkpoint: Instant::now(),
            user_id: our_bot_id.to_owned(),
        }
    }

    pub async fn user_status(&mut self) -> Result<Option<UserStatus>> {
        if self.status_checkpoint.elapsed() > self.status_poll_gap {
            self.status_checkpoint = Instant::now();
            self.client.user_status(self.user_id.as_str()).await.map(|status| Some(status))
        } else {
            Ok(None)
        }
    }
}

#[derive(Default)]
struct StatusClient {
    inner: reqwest::Client,
}

impl StatusClient {
    pub async fn user_status(&self, users: &str) -> Result<UserStatus> {
        self.inner
            .get(STATUS_ENDPOINT)
            .query(&[("ids", users)])
            .send()
            .await
            .map_err(Error::from)?
            .json::<Vec<UserStatus>>()
            .await
            .map_err(Error::from)?
            .first()
            .cloned()
            .ok_or(anyhow!("No statuses for {}", users))
    }
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct UserStatus {
    pub id: String,
    #[serde(default)]
    pub online: bool,
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use crate::userstatus::UserStatus;

    #[test]
    fn deserialize_with_flag_absent() -> Result<()> {
        assert_eq!(
            vec![UserStatus { id: "id".to_string(), online: false }],
            serde_json::from_str::<Vec<UserStatus>>(r#"[{"id": "id"}]"#)?
        );
        Ok(())
    }

    #[test]
    fn deserialize_with_flag_present() -> Result<()> {
        let json = r#"[{
            "id": "id",
            "online": true
        }]"#;
        assert_eq!(
            vec![UserStatus { id: "id".to_string(), online: true }],
            serde_json::from_str::<Vec<UserStatus>>(json)?
        );
        Ok(())
    }
}
