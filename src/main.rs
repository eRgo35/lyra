use poise::serenity_prelude::{self as serenity, ActivityData};
use reqwest::Client as HttpClient;
use songbird::SerenityInit;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

mod commands;
mod http;

use crate::commands::kashi;
use crate::commands::music;
use crate::commands::tools;
use crate::http::HttpKey;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            warn!("Error in command `{}`: {:?}", ctx.command().name, error);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().expect("Failed to load .env file.");

    let token =
        std::env::var("DISCORD_TOKEN").expect("Environment variable `DISCORD_TOKEN` not found!");
    let prefix = std::env::var("PREFIX").expect("Environment variable `PREFIX` not found!");

    let commands = vec![
        kashi::kashi(),
        music::deafen(),
        music::join(),
        music::leave(),
        music::mute(),
        music::pause(),
        music::play(),
        music::queue(),
        music::repeat(),
        music::resume(),
        music::seek(),
        music::shuffle(),
        music::skip(),
        music::soundboard(),
        music::stop(),
        music::volume(),
        tools::ai(),
        tools::dice(),
        tools::dictionary(),
        tools::help(),
        tools::ip(),
        tools::metar(),
        tools::owoify(),
        tools::ping(),
        tools::posix(),
        tools::qr(),
        tools::register(),
        tools::taf(),
        tools::uptime(),
        tools::verse(),
        tools::weather(),
    ];

    let options = poise::FrameworkOptions {
        commands,
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(prefix.to_string().into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            additional_prefixes: vec![],
            ..Default::default()
        },

        on_error: |error| Box::pin(on_error(error)),

        pre_command: |ctx| {
            Box::pin(async move {
                info!("Executing command {}...", ctx.command().qualified_name);
            })
        },

        post_command: |ctx| {
            Box::pin(async move {
                info!("Executed command {}!", ctx.command().qualified_name);
            })
        },

        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.author().id == 123456789 {
                    return Ok(false);
                }
                Ok(true)
            })
        }),

        skip_checks_for_owners: false,
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                info!(
                    "Got an event in event handler: {:?}",
                    event.snake_case_name()
                );
                Ok(())
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, _framework| {
            Box::pin(async move {
                info!(
                    "{} [{}] connected successfully!",
                    ready.user.name, ready.user.id
                );
                ctx.set_activity(Some(ActivityData::listening(prefix + "help")));
                // poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                Ok(Data {})
            })
        })
        .options(options)
        .build();

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await
        .expect("Error creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| error!("Client ended: {:?}", why));
    });

    let _signal_err = tokio::signal::ctrl_c().await;
    warn!("Recieved Ctrl-C, shutting down.");
}
