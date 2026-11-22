use poise::serenity_prelude::ChannelType;
use poise::CreateReply;

use super::userinfo::relative;
use crate::{commands::embeds::embed, Context, Error};

/// Shows information about this server
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("guildinfo", "server"),
    category = "Tools"
)]
pub async fn serverinfo(ctx: Context<'_>) -> Result<(), Error> {
    // Read everything from the cache first: the guild reference can't be held over an await.
    let (name, icon, lines) = {
        let Some(guild) = ctx.guild() else {
            return Ok(());
        };
        let count = |kind: ChannelType| guild.channels.values().filter(|c| c.kind == kind).count();
        let mut lines = vec![
            format!("**ID:** `{}`", guild.id),
            format!("**Owner:** <@{}>", guild.owner_id),
            format!("**Created:** {}", relative(guild.id.created_at())),
            format!("**Members:** {}", guild.member_count),
            format!(
                "**Channels:** {} text · {} voice · {} categories",
                count(ChannelType::Text) + count(ChannelType::News),
                count(ChannelType::Voice) + count(ChannelType::Stage),
                count(ChannelType::Category)
            ),
            format!("**Roles:** {}", guild.roles.len().saturating_sub(1)), // minus @everyone
            format!("**Emojis:** {}", guild.emojis.len()),
        ];
        let boosts = guild.premium_subscription_count.unwrap_or(0);
        if boosts > 0 {
            lines.push(format!(
                "**Boosts:** {boosts} (level {})",
                u8::from(guild.premium_tier)
            ));
        }
        if let Some(description) = &guild.description {
            lines.insert(0, format!("{description}\n"));
        }
        (guild.name.clone(), guild.icon_url(), lines)
    };

    let mut embed = embed(ctx, "Server info", &lines.join("\n"), &name).await?;
    if let Some(icon) = icon {
        embed = embed.thumbnail(icon);
    }
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
