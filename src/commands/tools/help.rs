use crate::{commands::embeds::embed, Context, Error};
use poise::CreateReply;
use std::fmt::Write as _;

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
    let extra_text_at_bottom = "\
Use /help command for more info on a command.
You can edit you message to the bot and the bot will edit its response.";

    let commands = &ctx.framework().options().commands;

    let mut found = command.as_ref().and_then(|name| {
        commands
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    });

    // If not a top-level command, try to find a subcommand by traversing qualified_name
    if found.is_none() {
        if let Some(name) = command.as_ref() {
            for parent in commands.iter() {
                for sub in &parent.subcommands {
                    if sub.name.eq_ignore_ascii_case(name)
                        || sub.qualified_name.eq_ignore_ascii_case(name)
                    {
                        found = Some(sub);
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }
        }
    }

    let reply = if let Some(cmd) = found {
        let prefix_str = ctx
            .framework()
            .options()
            .prefix_options
            .prefix
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_default();

        let mut invocations = String::new();
        if cmd.slash_action.is_some() {
            writeln!(invocations, "`/{}`", cmd.name).unwrap();
        }
        if cmd.prefix_action.is_some() {
            writeln!(invocations, "`{}{}`", prefix_str, cmd.name).unwrap();
        }
        if invocations.is_empty() {
            writeln!(invocations, "`{}`", cmd.name).unwrap();
        }
        let invocations = invocations.trim_end();

        let text = match (&cmd.description, &cmd.help_text) {
            (Some(d), Some(h)) => format!("{}\n\n{}", d, h),
            (Some(d), None) => d.to_string(),
            (None, Some(h)) => h.to_string(),
            (None, None) => "No help available".to_string(),
        };

        let mut params_text = String::new();
        if !cmd.parameters.is_empty() {
            params_text.push_str("\n\n```\nParameters:\n");
            let longest = cmd.parameters.iter().map(|p| p.name.len()).max().unwrap_or(0);
            for parameter in &cmd.parameters {
                let req = if parameter.required { "required" } else { "optional" };
                let desc = parameter.description.as_deref().unwrap_or("");
                let padding = " ".repeat(longest.saturating_sub(parameter.name.len()) + 3);
                writeln!(
                    params_text,
                    "{}{}({}) {}",
                    parameter.name, padding, req, desc
                )
                .unwrap();
            }
            params_text.push_str("```");
        }

        let mut subs_text = String::new();
        if !cmd.subcommands.is_empty() {
            subs_text.push_str("\n\n```\nSubcommands:\n");
            let longest = cmd.subcommands.iter().map(|s| s.name.len()).max().unwrap_or(0);
            for sub in &cmd.subcommands {
                let desc = sub.description.as_deref().unwrap_or("");
                let padding = " ".repeat(longest.saturating_sub(sub.name.len()) + 3);
                writeln!(subs_text, "{}{}{}", sub.name, padding, desc).unwrap();
            }
            subs_text.push_str("```");
        }

        format!("**{}**\n\n{}{}{}", invocations, text, params_text, subs_text)
    } else if let Some(name) = command {
        format!("No such command `{}`", name)
    } else {
        // List all top-level commands grouped by category
        let prefix_str = ctx
            .framework()
            .options()
            .prefix_options
            .prefix
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_default();

        let mut by_category: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for cmd in commands.iter() {
            let invocation = if cmd.slash_action.is_some() {
                format!("`/{}`", cmd.name)
            } else if cmd.prefix_action.is_some() {
                format!("`{}{}`", prefix_str, cmd.name)
            } else {
                continue;
            };
            let description = cmd.description.as_deref().unwrap_or("").to_string();
            let category = match &cmd.category {
                Some(cat) => cat.to_string(),
                None => "Uncategorized".to_string(),
            };
            by_category.entry(category).or_default().push(format!("{}  - {}", invocation, description));
        }

        let mut text = String::new();
        for (category, cmds) in &by_category {
            writeln!(text, "{}:", category).unwrap();
            for line in cmds {
                writeln!(text, "{}", line).unwrap();
            }
            text.push('\n');
        }
        text.push_str(extra_text_at_bottom);
        text
    };

    ctx.send(
        CreateReply::default().embed(embed(ctx, "Lyra", &reply, "").await.unwrap()),
    )
    .await?;
    Ok(())
}
