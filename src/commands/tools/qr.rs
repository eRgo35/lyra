use poise::CreateReply;
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter},
    model::{Colour, Timestamp},
};

use crate::{Context, Error};
use url::form_urlencoded;

/// Creates a qr code from text
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn qr(
    ctx: Context<'_>,
    #[description = "Message to encode"]
    #[rest]
    message: String,
) -> Result<(), Error> {
    let response = CreateReply::default().embed(generate_embed(ctx, message).await.unwrap());
    ctx.send(response).await?;

    Ok(())
}

async fn generate_embed(ctx: Context<'_>, message: String) -> Result<CreateEmbed, Error> {
    let timestamp = Timestamp::now();
    let data: String = form_urlencoded::byte_serialize(message.as_bytes()).collect();
    let url = format!(
        "http://api.qrserver.com/v1/create-qr-code/?data={}&size=1000x1000&ecc=Q&margin=8",
        data
    );

    let embed = CreateEmbed::default()
        .author(
            CreateEmbedAuthor::new("Your message as a QR Code!")
                .icon_url(ctx.author().clone().face()),
        )
        .colour(Colour::from_rgb(255, 58, 97))
        .title("Your QR Code:")
        .url(url.clone())
        .image(url)
        .timestamp(timestamp)
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}
