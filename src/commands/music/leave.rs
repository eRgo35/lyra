use crate::{Context, Error};

#[poise::command(prefix_command, slash_command)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if !manager.get(guild_id).is_some() {
        ctx.say("Not in a voice channel").await?;
        return Ok(())
    }

    if let Err(err) = manager.remove(guild_id).await { 
        ctx.say(format!("Failed: {:?}", err)).await?;
    }

    ctx.say("Left voice channel").await?;

    Ok(())
}
