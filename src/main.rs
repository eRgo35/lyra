use commands::tools::uptime::PROCESS_UPTIME;
use poise::serenity_prelude::{self as serenity, ActivityData};
use reqwest::Client as HttpClient;
use songbird::SerenityInit;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

mod commands;
mod http;

use crate::commands::music;
use crate::commands::tools;
use crate::http::HttpKey;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    pub http_client: HttpClient,
}

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
    // Optional: variables may come from the real environment instead.
    dotenvy::dotenv().ok();

    let _ = *PROCESS_UPTIME.lock().unwrap();

    let token =
        std::env::var("DISCORD_TOKEN").expect("Environment variable `DISCORD_TOKEN` not found!");
    let prefix = std::env::var("PREFIX").expect("Environment variable `PREFIX` not found!");

    let guild_id: Option<u64> = match std::env::var("GUILD_ID") {
        Ok(raw) => match raw.trim().parse() {
            Ok(id) => Some(id),
            Err(_) => {
                error!("Environment variable `GUILD_ID` is not a valid guild id: {raw:?}");
                std::process::exit(1);
            }
        },
        Err(_) => None,
    };

    let http_client = HttpClient::new();
    let data_http_client = http_client.clone();

    let commands = vec![
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
        music::skip(),
        music::stop(),
        music::volume(),
        music::soundboard::effect(),
        music::soundboard::stream(),
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
            prefix: Some(prefix.clone().into()),
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

        skip_checks_for_owners: false,
        event_handler: |_framework_ctx, event| {
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
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                info!(
                    "{} [{}] connected successfully!",
                    ready.user.name, ready.user.id
                );
                ctx.set_activity(Some(ActivityData::listening(prefix + "help")));

                let commands = &framework.options().commands;
                if let Some(guild_id) = guild_id {
                    poise::builtins::register_in_guild(ctx, commands, guild_id.into()).await?;
                } else {
                    poise::builtins::register_globally(ctx, commands).await?;
                }

                Ok(Data {
                    http_client: data_http_client,
                })
            })
        })
        .options(options)
        .build();

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(http_client)
        .await
        .expect("Error creating client");

    tokio::select! {
        result = client.start() => {
            if let Err(why) = result {
                error!("Client ended: {:?}", why);
                std::process::exit(1);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            warn!("Received Ctrl-C, shutting down.");
        }
    }
}
