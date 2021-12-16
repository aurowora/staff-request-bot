mod bot;
mod config;
mod db;

use std::env;
use std::sync::Arc;

use mongodb::{options::ClientOptions, bson::doc};

use serenity::framework::standard::StandardFramework;
use serenity::prelude::*;
use serenity::client::bridge::gateway::GatewayIntents;

use tokio::sync::RwLock;

use db::MongoClient;


struct MongoConn;

impl TypeMapKey for MongoConn {
    type Value = Arc<RwLock<MongoClient>>;
}

#[tokio::main]
async fn main() {
    println!("Staff Request Bot by Aurora, version 0.1");
    
    // Read in the configuration
    let cfg_path = match env::var("STAFFBOT_CFG") {
        Ok(val) => val,
        Err(e) => {
            println!("environment variable STAFFBOT_CFG unset or invalid, using /usr/local/etc/staffbot.yaml, err = {}", e);
            String::from("/usr/local/etc/staffbot.yaml")
        }
    };

    let cfg = config::read_config(&cfg_path);

    // Setup MongoDB
    let mongo_client = mongodb::Client::with_options(
        ClientOptions::parse(&cfg.mongo_uri)
            .await
            .expect("Failed to parse Mongo URI")
    ).expect("Could not config Mongo client");

    mongo_client.database(&cfg.mongo_database)
        .run_command(doc! {"ping": 1}, None)
        .await
        .expect("Could not ping Mongo, check connection");

    let mongo_client = MongoClient(mongo_client, cfg.mongo_database.clone());

    db::init_database(&mongo_client).await;

    // Setup the bot

    let framework = StandardFramework::new().configure(|c| c.with_whitespace(false)
                                                            .prefix(&cfg.bot_prefix)
                                                    )
                                                    .unrecognised_command(bot::unknown_command)
                                                    .on_dispatch_error(bot::bot_error)
                                                    .help(&bot::HELP_COMMAND)
                                                    .group(&bot::GENERAL_GROUP);

    let mut client = serenity::client::Client::builder(&cfg.token)
        .intents(GatewayIntents::all())
        .event_handler(bot::Bot)
        .framework(framework)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<MongoConn>(Arc::new(RwLock::new(mongo_client)));
    }

    if let Err(why) = client.start_autosharded().await {
        panic!("Failed to start Discord client: {:?}", why);
    }

}
