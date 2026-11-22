use poise::CreateReply;
use rand::seq::IndexedRandom;

use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};

/// Picks one of the options you give it
#[poise::command(prefix_command, slash_command, aliases("pick"), category = "Fun")]
pub async fn choose(
    ctx: Context<'_>,
    #[description = "Options separated by | or commas, e.g. pizza | sushi | tacos"]
    #[rest]
    options: String,
) -> Result<(), Error> {
    let options = split_options(&options);
    // Picked before any await: the thread-local RNG is not Send.
    let choice = options.choose(&mut rand::rng()).copied();
    let embed = match choice {
        Some(choice) if options.len() >= 2 => {
            embed(
                ctx,
                "I choose…",
                &options.join(" · "),
                &format!("👉 {choice}"),
            )
            .await?
        }
        _ => error_embed(ctx, "Give me at least two options, e.g. `pizza | sushi`!").await?,
    };
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Splits on `|`, or on commas if there is no `|`. Empty options are dropped.
fn split_options(input: &str) -> Vec<&str> {
    let separator = if input.contains('|') { '|' } else { ',' };
    input
        .split(separator)
        .map(str::trim)
        .filter(|option| !option.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::split_options;

    #[test]
    fn options_split_on_pipes_then_commas() {
        assert_eq!(
            split_options("pizza | sushi, rolls | "),
            ["pizza", "sushi, rolls"]
        );
        assert_eq!(split_options("a, b ,c"), ["a", "b", "c"]);
        assert_eq!(split_options("just one"), ["just one"]);
        assert!(split_options(" | , ").len() <= 1);
    }
}
