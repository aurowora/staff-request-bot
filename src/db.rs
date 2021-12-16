use mongodb::IndexModel;
use mongodb::options::{IndexOptions, ReplaceOptions};
use mongodb::bson::doc;
use serde::{Serialize, Deserialize};

pub struct MongoClient(pub mongodb::Client, pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestChannelPair {
    pub requests_channel: String,
    pub archive_channel: String,
    pub manager_role: String
}

pub async fn init_database(client: &MongoClient) {
    let col = client.0.database(&client.1).collection::<RequestChannelPair>("channels");

    let model_request = IndexModel::builder()
                .keys(doc!{
                    "requests_channel": 1
                })
                .options(
                    IndexOptions::builder()
                    .unique(true)
                    .build()
                )
                .build();

    col.create_index(model_request, None).await.expect("Failed to create unique index on requests_channel");
}

pub async fn read_channel_pair(client: &MongoClient, requests_channel: &String) -> Option<RequestChannelPair> {
    let col = client.0.database(&client.1).collection::<RequestChannelPair>("channels");

    col.find_one(doc!{
        "requests_channel": requests_channel
    }, None).await.ok()?
}

pub async fn write_channel_pair(client: &MongoClient, channel_pair: &RequestChannelPair) -> Option<mongodb::error::Error> {
    let col = client.0.database(&client.1).collection::<RequestChannelPair>("channels");

    col.replace_one(doc!{
        "requests_channel": &channel_pair.requests_channel
    }, channel_pair, ReplaceOptions::builder()
        .upsert(true)
        .build()
    ).await.err()
}

pub async fn delete_by_requests(client: &MongoClient, requests: &String) -> Option<mongodb::error::Error> {
    let col = client.0.database(&client.1).collection::<RequestChannelPair>("channels");

    col.delete_one(doc!{
        "requests_channel": requests
    }, None).await.err()
}