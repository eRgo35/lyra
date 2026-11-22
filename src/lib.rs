//! Lyra: a Discord music and utility bot. The binary in `main.rs` only reads the
//! environment and runs the client built here.

use poise::serenity_prelude::{self as serenity, ActivityData};
use reqwest::Client as HttpClient;
use songbird::SerenityInit;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub mod commands;
pub mod http;

use crate::commands::fun;
use crate::commands::music;
use crate::commands::tools;
use crate::http::HttpKey;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Where custom soundboard sounds are kept, relative to the working directory
/// (a volume in Docker, `/var/lib/lyra` under systemd).
pub const SOUNDBOARD_DIR: &str = "data/soundboard";

pub struct Data {
    pub http_client: HttpClient,
    pub repeat_all: std::sync::Arc<music::repeat_all::RepeatAll>,
    pub sounds: music::soundboard::store::SoundStore,
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

/// Every command the bot registers, prefix and slash.
pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        music::deafen(),
        music::forceplay(),
        music::join(),
        music::jump(),
        music::leave(),
        music::mute(),
        music::pause(),
        music::clear(),
        music::lyrics(),
        music::move_track(),
        music::nowplaying(),
        music::play(),
        music::queue(),
        music::remove(),
        music::repeat(),
        music::resume(),
        music::seek(),
        music::shuffle(),
        music::skip(),
        music::stop(),
        music::volume(),
        music::soundboard::effect(),
        music::soundboard::soundboard(),
        music::soundboard::stream(),
        fun::choose(),
        fun::coinflip(),
        fun::eightball(),
        fun::mock(),
        fun::rps(),
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
        tools::whois(),
    ]
}

/// Parses the optional `GUILD_ID` environment value.
pub fn parse_guild_id(raw: Option<&str>) -> Result<Option<u64>, String> {
    match raw {
        None => Ok(None),
        Some(raw) => raw.trim().parse().map(Some).map_err(|_| {
            format!("Environment variable `GUILD_ID` is not a valid guild id: {raw:?}")
        }),
    }
}

/// Builds the Discord client: framework, slash command registration (in `guild_id`, or
/// globally when `None`), songbird and the shared HTTP client.
pub async fn build_client(
    token: &str,
    prefix: String,
    guild_id: Option<u64>,
) -> Result<serenity::Client, Error> {
    let http_client = HttpClient::new();
    let data_http_client = http_client.clone();

    let options = poise::FrameworkOptions {
        commands: commands(),
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
                    repeat_all: Default::default(),
                    sounds: music::soundboard::store::SoundStore::new(SOUNDBOARD_DIR),
                })
            })
        })
        .options(options)
        .build();

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(http_client)
        .await
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::parse_guild_id;

    #[test]
    fn guild_id_is_optional_and_trimmed() {
        assert_eq!(parse_guild_id(None), Ok(None));
        assert_eq!(parse_guild_id(Some(" 1234 ")), Ok(Some(1234)));
    }

    #[test]
    fn invalid_guild_id_is_an_error_naming_the_value() {
        let err = parse_guild_id(Some("abc")).unwrap_err();
        assert!(err.contains("\"abc\""), "{err}");
        assert!(parse_guild_id(Some("")).is_err());
        assert!(parse_guild_id(Some("-1")).is_err());
    }
}
