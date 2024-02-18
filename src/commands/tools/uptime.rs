use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

// Currently unable to get information on how long the thread was running.
const PROCESS_UPTIME: i64 = 1000;

/// Checks how long the bot has been running
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn uptime(ctx: Context<'_>) -> Result<(), Error> {
    let uptime = PROCESS_UPTIME;
    let days = uptime / (24 * 60 * 60);
    let hours = (uptime % (24 * 60 * 60)) / 3600;
    let minutes = (uptime % 60 * 60) / 60;
    let seconds = uptime % 60;

    ctx.send(
        CreateReply::default().embed(
            embed(
                ctx,
                "I have been up and awake for",
                &format!("{} seconds", uptime),
                &format!(
                    "{} days, {} hours, {} minutes and {} seconds",
                    days, hours, minutes, seconds
                ),
            )
            .await
            .unwrap(),
        ),
    )
    .await?;

    Ok(())
}
