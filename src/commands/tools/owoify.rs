use poise::CreateReply;
use rand::RngExt;
use regex::Regex;
use std::str::FromStr;

use crate::{commands::embeds::embed, Context, Error};

pub trait OwOifiable {
    /// The owoification method
    fn owoify(&self) -> Self;
}

impl OwOifiable for String {
    /// Owoifies a String
    fn owoify(&self) -> Self {
        let mut rng = rand::rng();
        let faces = ["(・`ω´・)", "OwO", "owo", "oωo", "òωó", "°ω°", "UwU", ">w<", "^w^"];
        let face = &format!(" {} ", faces[rng.random_range(0..faces.len())]).to_owned();
        let pats: Vec<(&str, &str)> = vec![
            ("(?:r|l)", "w"),
            ("(?:R|L)", "W"),
            ("n([aeiou])", "ny$1"),
            ("N([aeiou])", "Ny$1"),
            ("N([AEIOU])", "NY$1"),
            ("th", "d"),
            ("ove", "uv"),
            ("!+", face),
        ];

        let mut owoified = String::from_str(self).unwrap();

        for &(f, t) in &pats {
            let re = Regex::new(f).unwrap();
            owoified = re.replace_all(&owoified, t).to_string();
        }

        owoified
    }
}

/// Owoifies whatever you want uwu
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn owoify(
    ctx: Context<'_>,
    #[description = "Text to owoify w-woify OwO"]
    #[rest]
    text: String,
) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "OwO", &text.owoify(), "").await.unwrap()))
        .await?;

    Ok(())
}