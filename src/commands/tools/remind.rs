use poise::CreateReply;

use crate::reminders::{parse_delay, Reminder};
use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};

/// Sets a reminder: remind 2h30m take the pizza out (add `dm` to get it privately)
#[poise::command(
    prefix_command,
    slash_command,
    category = "Tools",
    subcommands("set", "list", "cancel")
)]
pub async fn remind(
    ctx: Context<'_>,
    #[description = "Send it as a direct message"]
    #[flag]
    dm: bool,
    #[description = "When, e.g. 10m, 2h30m, 1d"] when: String,
    #[description = "What to remind you about"]
    #[rest]
    text: String,
) -> Result<(), Error> {
    // Prefix only (`!remind 2h text`); the slash form is `/remind set`.
    set_reminder(ctx, dm, &when, &text).await
}

/// Sets a reminder
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn set(
    ctx: Context<'_>,
    #[description = "Send it as a direct message instead of here"]
    #[flag]
    dm: bool,
    #[description = "When, e.g. 10m, 2h30m, 1d or 1w"] when: String,
    #[description = "What to remind you about"]
    #[rest]
    text: String,
) -> Result<(), Error> {
    set_reminder(ctx, dm, &when, &text).await
}

/// Lists your pending reminders
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let reminders = match ctx.data().reminders.list(ctx.author().id.get()).await {
        Ok(reminders) => reminders,
        Err(msg) => return reply_error(ctx, &msg).await,
    };
    let description = if reminders.is_empty() {
        "You have no reminders. Set one with `/remind set 10m stretch`.".to_string()
    } else {
        reminders.iter().map(line).collect::<Vec<_>>().join("\n")
    };
    ctx.send(
        CreateReply::default()
            .embed(embed(ctx, "Reminders", &description, "Your reminders").await?)
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

/// Cancels one of your reminders by its number
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn cancel(
    ctx: Context<'_>,
    #[description = "Reminder number, as shown by /remind list"] id: u64,
) -> Result<(), Error> {
    match ctx.data().reminders.cancel(ctx.author().id.get(), id).await {
        Ok(reminder) => {
            let title = format!("Cancelled #{}", reminder.id);
            ctx.send(
                CreateReply::default()
                    .embed(embed(ctx, "Reminders", &clip(&reminder.text), &title).await?)
                    .ephemeral(true),
            )
            .await?;
            Ok(())
        }
        Err(msg) => reply_error(ctx, &msg).await,
    }
}

async fn set_reminder(ctx: Context<'_>, dm: bool, when: &str, text: &str) -> Result<(), Error> {
    let delay = match parse_delay(when) {
        Ok(delay) => delay,
        Err(msg) => return reply_error(ctx, &msg).await,
    };
    let added = ctx
        .data()
        .reminders
        .add(
            ctx.author().id.get(),
            ctx.channel_id().get(),
            dm,
            delay,
            text,
        )
        .await;
    let reminder = match added {
        Ok(reminder) => reminder,
        Err(msg) => return reply_error(ctx, &msg).await,
    };

    let place = if dm { "by DM" } else { "here" };
    let description = format!(
        "{}\n\nI'll remind you {place} <t:{}:R> (<t:{}:f>).",
        clip(&reminder.text),
        reminder.due,
        reminder.due
    );
    let title = format!("Reminder #{} set", reminder.id);
    ctx.send(
        CreateReply::default()
            .embed(embed(ctx, "Reminders", &description, &title).await?)
            .ephemeral(dm),
    )
    .await?;
    Ok(())
}

async fn reply_error(ctx: Context<'_>, msg: &str) -> Result<(), Error> {
    ctx.send(
        CreateReply::default()
            .embed(error_embed(ctx, msg).await?)
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

fn line(reminder: &Reminder) -> String {
    let dm = if reminder.dm { " · DM" } else { "" };
    format!(
        "`#{}` <t:{}:R>{dm}: {}",
        reminder.id,
        reminder.due,
        clip(&reminder.text)
    )
}

/// At most 80 characters, for lists and confirmations.
fn clip(text: &str) -> String {
    if text.chars().count() > 80 {
        format!("{}…", text.chars().take(79).collect::<String>())
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_lines_show_id_time_and_clipped_text() {
        let reminder = Reminder {
            id: 3,
            user: 1,
            channel: 2,
            dm: true,
            created: 0,
            due: 1700000000,
            text: "y".repeat(100),
        };
        assert_eq!(
            line(&reminder),
            format!("`#3` <t:1700000000:R> · DM: {}…", "y".repeat(79))
        );
    }
}
