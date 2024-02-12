use crate::{Context, Error};

#[poise::command(prefix_command, check = "check")]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

async fn check(ctx: Context<'_>) -> Result<bool, Error> {
    let owner = std::env::var("OWNER_ID").expect("Environment variable `OWNER_ID` not found");
    Ok(ctx.author().id.to_string() == owner)
}
