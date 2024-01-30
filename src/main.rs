use songbird::SerenityInit;

use reqwest::Client as HttpClient;

use serenity::client::Context;

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::{
        standard::{macros::group, Configuration},
        StandardFramework,
    },
    model::gateway::Ready,
    prelude::{GatewayIntents, TypeMapKey},
};

mod commands;

// music management commands
use crate::commands::music::deafen::*;
use crate::commands::music::join::*;
use crate::commands::music::leave::*;
use crate::commands::music::mute::*;

// tools
use crate::commands::tools::ping::*;

// kashi
use crate::commands::kashi::kashi::*;

struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(
    join, deafen, leave, mute,
    ping,
    kashi
)]
struct General;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file.");
    tracing_subscriber::fmt::init();

    let token =
        std::env::var("DISCORD_TOKEN").expect("Environment variable `DISCORD_TOKEN` not found!");
    let prefix = std::env::var("PREFIX").expect("Environment variable `PREFIX` not found!");

    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix(prefix));

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await
        .expect("Error creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    let _signal_err = tokio::signal::ctrl_c().await;
    println!("Recieved Ctrl-C, shutting down.");
}
