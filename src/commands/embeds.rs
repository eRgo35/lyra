use crate::{Context, Error};

use poise::serenity_prelude::{
    Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::CreateReply;

pub async fn fail(ctx: Context<'_>, err: String) -> Result<(), Error> {
    ctx.send(
        CreateReply::default().embed(
            error_embed(ctx, &format!("Failed: {:?}", err))
                .await
                .unwrap(),
        ),
    )
    .await?;

    Ok(())
}

pub async fn error_embed(ctx: Context<'_>, msg: &str) -> Result<CreateEmbed, Error> {
    let embed = CreateEmbed::default()
        .author(
            CreateEmbedAuthor::new("Something went wrong!").icon_url(ctx.author().clone().face()),
        )
        .colour(Color::from_rgb(255, 58, 97))
        .title("Oopsie, Doopsie!")
        .description(msg)
        .timestamp(Timestamp::now())
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}

pub async fn embed(
    ctx: Context<'_>,
    author: &str,
    description: &str,
    title: &str,
) -> Result<CreateEmbed, Error> {
    let embed = CreateEmbed::default()
        .author(CreateEmbedAuthor::new(author).icon_url(ctx.author().clone().face()))
        .colour(Color::from_rgb(255, 58, 97))
        .title(title)
        .description(description)
        .timestamp(Timestamp::now())
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}
