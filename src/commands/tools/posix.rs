use std::time::SystemTime;

use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Prints current time in POSIX format
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn posix(ctx: Context<'_>) -> Result<(), Error> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    ctx.send(
        CreateReply::default().embed(
            embed(
                ctx,
                "The time is",
                "since Jan 1st 1970",
                &format!("{} ms", time),
            )
            .await
            .unwrap(),
        ),
    )
    .await?;

    Ok(())
}
