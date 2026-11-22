use poise::CreateReply;
use rand::seq::IndexedRandom;

use crate::{commands::embeds::embed, Context, Error};

const ANSWERS: [&str; 20] = [
    "It is certain.",
    "It is decidedly so.",
    "Without a doubt.",
    "Yes, definitely.",
    "You may rely on it.",
    "As I see it, yes.",
    "Most likely.",
    "Outlook good.",
    "Yes.",
    "Signs point to yes.",
    "Reply hazy, try again.",
    "Ask again later.",
    "Better not tell you now.",
    "Cannot predict now.",
    "Concentrate and ask again.",
    "Don't count on it.",
    "My reply is no.",
    "My sources say no.",
    "Outlook not so good.",
    "Very doubtful.",
];

/// Asks the magic 8-ball a yes-or-no question
#[poise::command(
    prefix_command,
    slash_command,
    rename = "8ball",
    aliases("eightball"),
    category = "Fun"
)]
pub async fn eightball(
    ctx: Context<'_>,
    #[description = "Your yes-or-no question"]
    #[rest]
    question: String,
) -> Result<(), Error> {
    let answer = ANSWERS.choose(&mut rand::rng()).copied().unwrap_or("Yes.");
    ctx.send(
        CreateReply::default().embed(
            embed(
                ctx,
                "Magic 8-ball",
                &format!("🎱 **{answer}**"),
                question.trim(),
            )
            .await?,
        ),
    )
    .await?;
    Ok(())
}
