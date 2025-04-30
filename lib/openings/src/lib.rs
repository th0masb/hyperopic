use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{Error, Result, anyhow};
use hyperopic::openings::{OpeningMoveFetcher, OpeningMoveRecord};
use hyperopic::position::Position;
use itertools::Itertools;
use log::info;
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, GetItemInput};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct OpeningTable {
    pub name: String,
    pub region: String,
    #[serde(rename = "positionKey")]
    pub position_key: String,
    #[serde(rename = "moveKey")]
    pub move_key: String,
    #[serde(rename = "maxDepth")]
    pub max_depth: u8,
}

pub struct DynamoOpeningClient {
    params: OpeningTable,
    client: DynamoDbClient,
}

impl TryFrom<OpeningTable> for DynamoOpeningClient {
    type Error = Error;

    fn try_from(value: OpeningTable) -> std::result::Result<Self, Self::Error> {
        let region = Region::from_str(value.region.as_str())?;
        Ok(DynamoOpeningClient { client: DynamoDbClient::new(region), params: value })
    }
}

impl OpeningMoveFetcher for DynamoOpeningClient {
    fn lookup(&self, position_key: &str) -> Result<Vec<OpeningMoveRecord>> {
        futures::executor::block_on(async {
            let index = position_key.to_string().split_whitespace().take(3).join(" ");
            info!("Querying table {} for position {}", self.params.name, index);
            self.client
                .get_item(self.create_request(index))
                .await
                .map_err(|err| anyhow!("{}", err))
                .and_then(|response| match response.item {
                    None => {
                        info!("No match found!");
                        Ok(vec![])
                    }
                    Some(attributes) => self.try_extract_move(attributes),
                })
        })
    }
}
impl DynamoOpeningClient {
    fn create_request(&self, query_position: String) -> GetItemInput {
        // Create key
        let mut av = AttributeValue::default();
        av.s = Some(query_position);
        let mut key = HashMap::new();
        key.insert(self.params.position_key.clone(), av);
        // Create request
        let mut request = GetItemInput::default();
        request.table_name = self.params.name.clone();
        request.key = key;
        request
    }

    fn try_extract_move(
        &self,
        attributes: HashMap<String, AttributeValue>,
    ) -> Result<Vec<OpeningMoveRecord>> {
        match attributes.get(&self.params.move_key) {
            None => Err(anyhow!("Position exists but missing recommended move attribute")),
            Some(attribute) => match &attribute.ss {
                None => Err(anyhow!(
                    "Position and recommended move attribute exist but not string set type"
                )),
                Some(move_set) => {
                    info!("Found matching set {:?}!", move_set);
                    Ok(move_set
                        .iter()
                        .filter_map(|m| OpeningMoveRecord::from_str(m).ok())
                        .collect::<Vec<OpeningMoveRecord>>())
                }
            },
        }
    }
}
