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
        let faces = [
            "(・`ω´・)",
            "OwO",
            "owo",
            "oωo",
            "òωó",
            "°ω°",
            "UwU",
            ">w<",
            "^w^",
        ];
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

#[cfg(test)]
mod tests {
    use super::OwOifiable;

    fn owo(s: &str) -> String {
        s.to_string().owoify()
    }

    #[test]
    fn replaces_r_and_l_preserving_case() {
        assert_eq!(owo("hello world"), "hewwo wowwd");
        assert_eq!(owo("LARGE Rain"), "WAWGE Wain");
    }

    #[test]
    fn inserts_y_after_n_before_vowel() {
        assert_eq!(owo("banana"), "banyanya");
        assert_eq!(owo("Nap"), "Nyap");
        assert_eq!(owo("NAO"), "NYAO");
        assert_eq!(owo("nope"), "nyope");
        assert_eq!(owo("string"), "stwing");
    }

    #[test]
    fn rules_apply_in_order() {
        // l -> w runs before ove -> uv and th -> d.
        assert_eq!(owo("love the"), "wuv de");
    }

    #[test]
    fn exclamation_run_becomes_one_face() {
        let out = owo("yay!!!");
        assert!(out.starts_with("yay "), "got {out:?}");
        assert!(!out.contains('!'), "got {out:?}");
        assert!(out.ends_with(' '), "got {out:?}");
    }

    #[test]
    fn text_without_patterns_is_untouched() {
        assert_eq!(owo(""), "");
        assert_eq!(owo("日本語 ñ 123"), "日本語 ñ 123");
    }
}
