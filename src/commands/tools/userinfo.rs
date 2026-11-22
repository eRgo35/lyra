use poise::serenity_prelude::{Mentionable, Timestamp, User};
use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Shows information about a user (you, by default)
#[poise::command(
    prefix_command,
    slash_command,
    aliases("whoami", "user"),
    category = "Tools"
)]
pub async fn userinfo(
    ctx: Context<'_>,
    #[description = "User to show (default: you)"] user: Option<User>,
) -> Result<(), Error> {
    let user = user.unwrap_or_else(|| ctx.author().clone());

    let mut lines = vec![
        format!("**User:** {} ({})", user.tag(), user.mention()),
        format!("**ID:** `{}`", user.id),
        format!("**Account created:** {}", relative(user.created_at())),
    ];
    if user.bot {
        lines.push("**Bot:** yes".to_string());
    }

    if let Some(guild_id) = ctx.guild_id() {
        if let Ok(member) = guild_id.member(ctx, user.id).await {
            if let Some(nick) = &member.nick {
                lines.push(format!("**Nickname:** {nick}"));
            }
            if let Some(joined) = member.joined_at {
                lines.push(format!("**Joined this server:** {}", relative(joined)));
            }
            if let Some(since) = member.premium_since {
                lines.push(format!("**Boosting since:** {}", relative(since)));
            }
            if !member.roles.is_empty() {
                lines.push(format!("**Roles:** {}", roles(&member.roles)));
            }
        }
    }

    let name = user
        .global_name
        .clone()
        .unwrap_or_else(|| user.name.clone());
    let embed = embed(ctx, "User info", &lines.join("\n"), &name)
        .await?
        .thumbnail(user.face());
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// A Discord timestamp: the date, plus "3 years ago" in each viewer's locale.
pub(super) fn relative(time: Timestamp) -> String {
    let unix = time.unix_timestamp();
    format!("<t:{unix}:D> (<t:{unix}:R>)")
}

const LISTED_ROLES: usize = 20;

fn roles(roles: &[poise::serenity_prelude::RoleId]) -> String {
    let mut mentions: Vec<String> = roles
        .iter()
        .take(LISTED_ROLES)
        .map(|r| format!("<@&{r}>"))
        .collect();
    if roles.len() > LISTED_ROLES {
        mentions.push(format!("and {} more", roles.len() - LISTED_ROLES));
    }
    mentions.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use poise::serenity_prelude::RoleId;

    #[test]
    fn timestamps_use_discord_formatting() {
        let time = Timestamp::from_unix_timestamp(1_600_000_000).unwrap();
        assert_eq!(relative(time), "<t:1600000000:D> (<t:1600000000:R>)");
    }

    #[test]
    fn long_role_lists_are_cut() {
        let many: Vec<_> = (1..=25).map(RoleId::new).collect();
        let text = roles(&many);
        assert!(text.starts_with("<@&1> <@&2>"));
        assert!(text.ends_with("<@&20> and 5 more"));
    }
}
