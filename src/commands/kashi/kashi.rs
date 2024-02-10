use crate::{Context, Error};

#[poise::command(prefix_command, slash_command)]
pub async fn kashi(ctx: Context<'_>) -> Result<(), Error> {
    
    let response = format!("Kashi platform is currently under construction!");
    ctx.say(response).await?;

    Ok(())
}
