use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// rEpEaTs YoUr TeXt LiKe ThIs
#[poise::command(prefix_command, slash_command, category = "Fun")]
pub async fn mock(
    ctx: Context<'_>,
    #[description = "Text to mock"]
    #[rest]
    text: String,
) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "Mock", &mocking(&text), "").await?))
        .await?;
    Ok(())
}

/// Alternates the case of letters, starting lower-case; other characters are kept and
/// don't count towards the alternation.
fn mocking(text: &str) -> String {
    let mut upper = false;
    text.chars()
        .flat_map(|c| {
            let changed: Vec<char> = if !c.is_alphabetic() {
                vec![c]
            } else if upper {
                c.to_uppercase().collect()
            } else {
                c.to_lowercase().collect()
            };
            if c.is_alphabetic() {
                upper = !upper;
            }
            changed
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::mocking;

    #[test]
    fn letters_alternate_and_the_rest_is_kept() {
        assert_eq!(mocking("I love Rust!"), "i LoVe RuSt!");
        assert_eq!(mocking("zażółć 123"), "zAżÓłĆ 123");
    }
}
