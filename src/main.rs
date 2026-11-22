use lyra::commands::tools::uptime::PROCESS_UPTIME;
use tracing::{error, warn};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    // Optional: variables may come from the real environment instead.
    dotenvy::dotenv().ok();

    let _ = *PROCESS_UPTIME.lock().unwrap();

    let token =
        std::env::var("DISCORD_TOKEN").expect("Environment variable `DISCORD_TOKEN` not found!");
    let prefix = std::env::var("PREFIX").expect("Environment variable `PREFIX` not found!");

    let guild_id = match lyra::parse_guild_id(std::env::var("GUILD_ID").ok().as_deref()) {
        Ok(guild_id) => guild_id,
        Err(msg) => {
            error!("{msg}");
            std::process::exit(1);
        }
    };

    let mut client = lyra::build_client(&token, prefix, guild_id)
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
