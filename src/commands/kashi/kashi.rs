use crate::{Context, Error};

/// Kashi integration platform (WIP)
#[poise::command(
    prefix_command, 
    slash_command,
    category = "Kashi"
)]
pub async fn kashi(
    ctx: Context<'_>
) -> Result<(), Error> {
    
    let response = format!("Kashi platform is currently under construction!");
    ctx.say(response).await?;

    Ok(())
}
