use crate::{Context, Error};

/// Prints this help message; aliases: help, huh, welp
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    aliases("huh", "welp"),
    category = "Help"
)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "\
Use /help command for more info on a command.
You can edit you message to the bot and the bot will edit its response.",
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}
