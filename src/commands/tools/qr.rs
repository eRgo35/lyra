use poise::serenity_prelude::{
    Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::CreateReply;

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
    let response = CreateReply::default().embed(generate_embed(ctx, message).await?);
    ctx.send(response).await?;

    Ok(())
}

async fn generate_embed(ctx: Context<'_>, message: String) -> Result<CreateEmbed, Error> {
    let timestamp = Timestamp::now();
    let url = qr_url(&message);

    let embed = CreateEmbed::default()
        .author(
            CreateEmbedAuthor::new("Your message as a QR Code!")
                .icon_url(ctx.author().clone().face()),
        )
        .colour(Color::from_rgb(255, 58, 97))
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

/// QR code image URL for `message` (URL-encoded into the `data` parameter).
fn qr_url(message: &str) -> String {
    let data: String = form_urlencoded::byte_serialize(message.as_bytes()).collect();
    format!(
        "http://api.qrserver.com/v1/create-qr-code/?data={}&size=1000x1000&ecc=Q&margin=8",
        data
    )
}

#[cfg(test)]
mod tests {
    use super::qr_url;

    #[test]
    fn encodes_the_message_so_it_cannot_add_parameters() {
        let url = qr_url("hi there&size=1x1#x/ż");
        assert_eq!(
            url,
            "http://api.qrserver.com/v1/create-qr-code/?data=hi+there%26size%3D1x1%23x%2F%C5%BC&size=1000x1000&ecc=Q&margin=8"
        );
    }
}
