use songbird::SerenityInit;

use poise::serenity_prelude::{self as serenity, ActivityData};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error};

mod commands;

// commands: music
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

// commands: tools
use crate::commands::tools::ping::*;

// commands: kashi
use crate::commands::kashi::kashi::*;

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

// this is for debug only
#[poise::command(prefix_command)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    // logger and dotenv initialization
    tracing_subscriber::fmt::init();
    dotenv::dotenv().expect("Failed to load .env file.");

    let token = std::env::var("DISCORD_TOKEN").expect("Environment variable `DISCORD_TOKEN` not found!");
    let prefix = std::env::var("PREFIX").expect("Environment variable `PREFIX` not found!");

    let commands = vec![
        // commands: music
        deafen(),
        join(),
        leave(),
        loopcurrent(),
        mute(),
        pause(),
        play(),
        queue(),
        resume(),
        skip(),
        stop(),
        // commands: tools
        ping(),
        // commands: kashi
        kashi(),
        // commands: debug
        register(),
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
                info!("Got an event in event handler: {:?}", event.snake_case_name());
                Ok(())
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, _framework| {
            Box::pin(async move {
                info!("{} [{}] connected successfully!", ready.user.name, ready.user.id);
                ctx.set_activity(Some(ActivityData::listening(prefix + "help")));
                // poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                Ok(Data {})
            })
        })
        .options(options)
        .build();

    let intents = serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .register_songbird()
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
