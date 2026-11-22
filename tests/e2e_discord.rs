//! End-to-end smoke test against real Discord: the bot logs in, registers its slash
//! commands in a test guild, and they show up through the REST API.
//!
//! Optional: needs a bot token and a guild the bot is in (with the
//! `applications.commands` scope). Uses its own variables so it never picks up the
//! production `.env` by accident:
//!
//! ```text
//! LYRA_E2E_DISCORD_TOKEN=... LYRA_E2E_GUILD_ID=... \
//!     cargo test --test e2e_discord -- --ignored --nocapture
//! ```
//!
//! It overwrites that guild's slash commands with the current ones.

use std::time::Duration;

use poise::serenity_prelude::{GuildId, Http};

#[tokio::test]
#[ignore = "needs LYRA_E2E_DISCORD_TOKEN and LYRA_E2E_GUILD_ID"]
async fn bot_boots_and_registers_its_slash_commands() {
    let (Ok(token), Ok(guild_id)) = (
        std::env::var("LYRA_E2E_DISCORD_TOKEN"),
        std::env::var("LYRA_E2E_GUILD_ID"),
    ) else {
        eprintln!("skipped: set LYRA_E2E_DISCORD_TOKEN and LYRA_E2E_GUILD_ID to run");
        return;
    };
    let guild_id = lyra::parse_guild_id(Some(&guild_id))
        .unwrap()
        .expect("guild id");

    let expected: Vec<String> = lyra::commands()
        .iter()
        .filter_map(|command| command.create_as_slash_command())
        .map(|definition| serde_json::to_value(definition).unwrap()["name"].to_string())
        .map(|name| name.trim_matches('"').to_string())
        .collect();

    let mut client = lyra::build_client(&token, "!".to_string(), Some(guild_id))
        .await
        .expect("client builds");
    let shards = client.shard_manager.clone();
    let running = tokio::spawn(async move { client.start().await });

    let http = Http::new(&token);
    let bot = http.get_current_user().await.expect("token is valid");
    eprintln!("logged in as {}", bot.name);

    // Registration happens in the framework setup, after the gateway Ready event.
    let mut missing = expected.clone();
    for _ in 0..60 {
        let registered = http
            .get_guild_commands(GuildId::new(guild_id))
            .await
            .expect("can list guild commands");
        missing.retain(|name| !registered.iter().any(|c| &c.name == name));
        if missing.is_empty() {
            break;
        }
        missing = expected.clone();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    shards.shutdown_all().await;
    let _ = tokio::time::timeout(Duration::from_secs(10), running).await;

    assert!(
        missing.is_empty(),
        "not registered after 60s: {missing:?} (is the bot in the guild with applications.commands?)"
    );
}
