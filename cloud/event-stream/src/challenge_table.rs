use std::collections::HashMap;
use std::str::FromStr;
use std::time::SystemTime;

use crate::config::AwsResourceId;
use anyhow::{Result, anyhow};
use rusoto_core::Region;
use rusoto_dynamodb::{
    AttributeValue, AttributeValueUpdate, DynamoDb, DynamoDbClient, PutItemInput, UpdateItemInput,
};
use rusoto_dynamodb::{GetItemInput, QueryInput};

const SECS_IN_DAY: u64 = 24 * 60 * 60;
const EPOCH_DAY_INDEX_NAME: &str = "EpochDayIndex";

mod attribute_keys {
    pub const CHALLENGER: &str = "ChallengerID";
    pub const CHALLENGE: &str = "ChallengeID";
    pub const EPOCH_DAY: &str = "ChallengeDay";
    pub const EXPIRY: &str = "Expiry";
    pub const STARTED: &str = "GameStarted";
}

pub struct ChallengeTableClient {
    table_name: String,
    client: DynamoDbClient,
}

#[derive(Clone, Debug)]
pub struct ChallengeTableEntry {
    pub challenger: String,
    pub challenge_id: String,
    #[allow(dead_code)]
    pub challenge_epoch_day: u64,
    #[allow(dead_code)]
    pub game_started: bool,
}

impl ChallengeTableClient {
    pub fn new(config: &AwsResourceId) -> ChallengeTableClient {
        ChallengeTableClient {
            table_name: config.name.clone(),
            client: DynamoDbClient::new(
                Region::from_str(config.region.as_str())
                    .expect(format!("Bad region: {}", config.region).as_str()),
            ),
        }
    }

    pub async fn get_entry(
        &self,
        challenger_id: &str,
        challenge_id: &str,
    ) -> Result<Option<ChallengeTableEntry>> {
        let request = init(|r: &mut GetItemInput| {
            r.table_name = self.table_name.clone();
            r.key = create_search_key(challenger_id, challenge_id);
        });
        let response = self.client.get_item(request).await.map_err(|e| anyhow!(e))?;

        match response.item {
            None => Ok(None),
            Some(attr) => self.extract_entry(&attr).map(|e| Some(e)),
        }
    }

    pub async fn insert_challenge(&self, challenger_id: &str, challenge_id: &str) -> Result<()> {
        log::info!("Inserting challenge for {}/{}", challenger_id, challenge_id);
        let request = init(|r: &mut PutItemInput| {
            r.table_name = self.table_name.clone();
            // Don't overwite existing challenges!
            r.condition_expression =
                Some(format!("attribute_not_exists({})", attribute_keys::EPOCH_DAY));
            r.item = init(|dest: &mut HashMap<String, AttributeValue>| {
                dest.insert(
                    attribute_keys::CHALLENGER.to_owned(),
                    init(|a: &mut AttributeValue| a.s = Some(challenger_id.to_owned())),
                );
                dest.insert(
                    attribute_keys::CHALLENGE.to_owned(),
                    init(|a: &mut AttributeValue| a.s = Some(challenge_id.to_owned())),
                );
                dest.insert(
                    attribute_keys::STARTED.to_owned(),
                    init(|a: &mut AttributeValue| a.bool = Some(false)),
                );
                dest.insert(
                    attribute_keys::EPOCH_DAY.to_owned(),
                    init(|a: &mut AttributeValue| a.n = Some(epoch_day().to_string())),
                );
                dest.insert(
                    attribute_keys::EXPIRY.to_owned(),
                    init(|a: &mut AttributeValue| {
                        a.n = Some((epoch_secs() + 2 * SECS_IN_DAY).to_string())
                    }),
                );
            });
        });
        self.client.put_item(request).await.map(|_| ()).map_err(|e| anyhow!(e))
    }

    pub async fn set_started(&self, challenger_id: &str, challenge_id: &str) -> Result<bool> {
        log::info!("Toggling started for {}/{}", challenger_id, challenge_id);
        let request = init(|r: &mut UpdateItemInput| {
            r.table_name = self.table_name.clone();
            r.return_values = Some("ALL_OLD".to_owned());
            r.key = create_search_key(challenger_id, challenge_id);
            r.attribute_updates = Some(init(|dest: &mut HashMap<String, AttributeValueUpdate>| {
                dest.insert(
                    attribute_keys::STARTED.to_owned(),
                    init(|update: &mut AttributeValueUpdate| {
                        update.action = Some("PUT".to_owned());
                        update.value = Some(init(|a: &mut AttributeValue| a.bool = Some(true)))
                    }),
                );
            }))
        });
        self.client.update_item(request).await.map_err(|e| anyhow!(e)).and_then(|response| {
            response
                .attributes
                .and_then(|attr| attr.get(attribute_keys::STARTED).cloned())
                .and_then(|started| started.bool)
                .map(|started| !started)
                .ok_or(anyhow!("Unknown flag change for {}-{}", challenger_id, challenge_id))
        })
    }

    pub async fn fetch_challenges_today(&self) -> Result<Vec<ChallengeTableEntry>> {
        let mut dest = Vec::new();
        let mut last_evaluated_key = None;
        loop {
            let query = self.build_challenges_query(last_evaluated_key.clone());
            let output = self.client.query(query).await?;
            if let Some(items) = output.items {
                dest.extend(self.extract_entries(&items)?);
            }
            last_evaluated_key = output.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }
        Ok(dest)
    }

    fn build_challenges_query(
        &self,
        last_evaluated_key: Option<HashMap<String, AttributeValue>>,
    ) -> QueryInput {
        init(|query: &mut QueryInput| {
            query.table_name = self.table_name.clone();
            query.exclusive_start_key = last_evaluated_key;
            query.index_name = Some(EPOCH_DAY_INDEX_NAME.to_owned());
            query.key_condition_expression = Some(format!("{} = :x", attribute_keys::EPOCH_DAY));
            query.expression_attribute_values =
                Some(init(|dest: &mut HashMap<String, AttributeValue>| {
                    dest.insert(
                        ":x".to_owned(),
                        init(|a: &mut AttributeValue| a.n = Some(epoch_day().to_string())),
                    );
                }));
        })
    }

    fn extract_entries(
        &self,
        attributes: &Vec<HashMap<String, AttributeValue>>,
    ) -> Result<Vec<ChallengeTableEntry>> {
        let mut dest = Vec::new();
        for attr in attributes {
            dest.push(self.extract_entry(attr)?)
        }
        Ok(dest)
    }

    fn extract_entry(&self, attr: &HashMap<String, AttributeValue>) -> Result<ChallengeTableEntry> {
        Ok(ChallengeTableEntry {
            challenger: extract_attribute(attr, attribute_keys::CHALLENGER, |a| a.s.clone())?,
            challenge_id: extract_attribute(attr, attribute_keys::CHALLENGE, |a| a.s.clone())?,
            game_started: extract_attribute(attr, attribute_keys::STARTED, |a| a.bool)?,
            challenge_epoch_day: extract_attribute(attr, attribute_keys::EPOCH_DAY, |a| {
                a.n.clone()
            })
            .and_then(|v| v.parse::<u64>().map_err(|e| anyhow!(e)))?,
        })
    }
}

fn create_search_key(challenger_id: &str, challenge_id: &str) -> HashMap<String, AttributeValue> {
    init(|dest: &mut HashMap<String, AttributeValue>| {
        dest.insert(
            attribute_keys::CHALLENGER.to_owned(),
            init(|a: &mut AttributeValue| a.s = Some(challenger_id.to_owned())),
        );
        dest.insert(
            attribute_keys::CHALLENGE.to_owned(),
            init(|a: &mut AttributeValue| a.s = Some(challenge_id.to_owned())),
        );
    })
}

fn init<T, F>(configure: F) -> T
where
    T: Default,
    F: FnOnce(&mut T) -> (),
{
    let mut instance = Default::default();
    configure(&mut instance);
    instance
}

fn extract_attribute<T, F: Fn(&AttributeValue) -> Option<T>>(
    attributes: &HashMap<String, AttributeValue>,
    key: &str,
    extractor: F,
) -> Result<T> {
    extractor(&attributes[key]).ok_or(anyhow!(
        "Attribute {} could not be extracted from {:?}",
        key,
        attributes
    ))
}

fn epoch_secs() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
}

fn epoch_day() -> u64 {
    epoch_secs() / 60 / 60 / 24
}
