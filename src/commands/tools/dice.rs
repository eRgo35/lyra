use rand::Rng;

use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Rolls a dice
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn dice(ctx: Context<'_>) -> Result<(), Error> {
    let dice = {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..=6)
    };

    ctx.send(
        CreateReply::default().embed(
            embed(
                ctx,
                "Let's roll the dice!",
                "",
                &format!("Your number is: {}", dice),
            )
            .await
            .unwrap(),
        ),
    )
    .await?;

    Ok(())
}
