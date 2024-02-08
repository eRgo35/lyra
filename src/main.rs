use serenity::gateway::ActivityData;
use serenity::model::prelude::Message;
use songbird::SerenityInit;

use reqwest::Client as HttpClient;

use serenity::client::Context;

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::{
        standard::{macros::group, macros::hook, Configuration},
        StandardFramework,
    },
    model::gateway::Ready,
    prelude::GatewayIntents,
};

use tracing::info;

mod commands;

// music management commands
use crate::commands::music::deafen::*;
use crate::commands::music::join::*;
use crate::commands::music::leave::*;
use crate::commands::music::mute::*;
use crate::commands::music::play::*;
use crate::commands::music::queue::*;
use crate::commands::music::skip::*;
use crate::commands::music::stop::*;
use crate::commands::music::loopcurrent::*;
use crate::commands::music::pause::*;
use crate::commands::music::resume::*;

// tools
use crate::commands::tools::ping::*;

// kashi
use crate::commands::kashi::kashi::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} [{}] connected successfully!", ready.user.name, ready.user.id);
        let prefix = std::env::var("PREFIX").expect("Environment variable `PREFIX` not found!");
        ctx.set_activity(Some(ActivityData::listening(prefix + "help")));
    }
}

#[hook]
async fn before(_: &Context, msg: &Message, command_name: &str) -> bool {
    info!(
        "Received command [{}] from user [{}]",
        command_name, msg.author.name
    );
    true
}

#[group]
#[commands(
    join, deafen, leave, mute, play, ping, kashi, queue, stop, skip, loopcurrent, pause, resume
)]
struct General;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file.");
    tracing_subscriber::fmt::init();

    let token =
        std::env::var("DISCORD_TOKEN").expect("Environment variable `DISCORD_TOKEN` not found!");
    let prefix = std::env::var("PREFIX").expect("Environment variable `PREFIX` not found!");

    let framework = StandardFramework::new().before(before).group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix(prefix));

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .register_songbird()
        .event_handler(Handler)
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
