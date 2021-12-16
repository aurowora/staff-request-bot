use serde::{Serialize, Deserialize};
use std::fs::File;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub token: String,
    pub mongo_uri: String,
    pub bot_prefix: String,
    pub mongo_database: String
}

pub fn read_config(path: &str) -> Configuration {
    let path = Path::new(&path);
    
    let file = match File::open(&path) {
        Err(why) => panic!("failed to open {}: {}", path.display(), why),
        Ok(file) => file
    };

    match serde_yaml::from_reader(file) {
        Err(why) => panic!("failed to read {}: {}", path.display(), why),
        Ok(cfg) => cfg
    }
}
