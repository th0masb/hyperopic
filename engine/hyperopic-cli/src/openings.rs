use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use hyperopic::openings::{OpeningMoveFetcher, OpeningMoveRecord};
use anyhow::{anyhow, Result};

pub struct OpeningsDatabase {
    contents: HashMap<String, Vec<OpeningMoveRecord>>,
}

impl OpeningsDatabase {
    pub fn new(path: std::path::PathBuf) -> Result<OpeningsDatabase> {
        let mut contents = HashMap::new();
        let path_name = path.to_string_lossy().to_string();
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let components = line.split(",").collect::<Vec<&str>>();
            let key = components.get(0).ok_or(anyhow!("Bad line in {}: {}", path_name, line))?;
            let value = components.get(1).ok_or(anyhow!("Bad line in {}: {}", path_name, line))?;
            let records = value.split(";")
                .map(|s| OpeningMoveRecord::from_str(s))
                .collect::<Result<Vec<_>>>()?;
            contents.insert(key.to_string(), records);
        }
        Ok(OpeningsDatabase { contents })
    }
}

impl OpeningMoveFetcher for OpeningsDatabase {
    fn lookup(&self, position_key: &str) -> Result<Vec<OpeningMoveRecord>> {
        Ok(self.contents.get(position_key).cloned().unwrap_or(vec![]))
    }
}