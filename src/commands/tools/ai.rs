use rand::Rng;

use std::thread::sleep;
use std::time::Duration;

use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Asks AI
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn ai(
    ctx: Context<'_>,
    #[description = "prompt to ask"]
    #[rest]
    prompt: String,
) -> Result<(), Error> {
    let iamsorry = vec![
        "I'm sorry, but as an AI language model, I must follow ethical guidelines, and I cannot engage in harmful, malicious, or offensive behavior.",
        "I'm sorry, but as an AI language model, I may not always be perfect and can make mistakes or provide inaccurate information. Please verify important details from reliable sources.",
        "I'm sorry, but as an AI language model, I can't engage in real-time conversations or remember previous interactions with users.",
        "I'm sorry, but as an AI language model, I don't have personal opinions or feelings; I can only provide information based on patterns in the data I was trained on.",
        "I'm sorry, but as an AI language model, I don't have access to real-time information or updates beyond my last training data in September 2021.",
        "I'm sorry, but as an AI language model, I don't have the ability to recall specific personal data or information about individuals.",
        "I'm sorry, but as an AI language model, I don't have consciousness or self-awareness. I'm simply a program designed to process and generate human-like text."
    ];

    println!("Funny prompts: {}", prompt);

    let response;

    let _ = {
        let mut rng = rand::thread_rng();

        response = rng.gen_range(0..iamsorry.len());
    };

    sleep(Duration::from_secs(3));

    ctx.send(
        CreateReply::default().embed(
            embed(ctx, "AI Response:", "", &format!("{}", iamsorry[response]))
                .await
                .unwrap(),
        ),
    )
    .await?;

    Ok(())
}
