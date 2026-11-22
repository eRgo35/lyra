use poise::CreateReply;
use rand::RngExt;

use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};

/// Rolls dice: a six-sided die, or dice notation like 2d20+3
#[poise::command(prefix_command, slash_command, aliases("roll"), category = "Fun")]
pub async fn dice(
    ctx: Context<'_>,
    #[description = "Dice notation, e.g. d20, 2d6 or 3d8+2 (default 1d6)"]
    #[rest]
    notation: Option<String>,
) -> Result<(), Error> {
    let notation = notation.unwrap_or_else(|| "1d6".to_string());
    let embed = match Roll::parse(&notation) {
        Ok(roll) => {
            let rolls: Vec<u32> = {
                let mut rng = rand::rng();
                (0..roll.count)
                    .map(|_| rng.random_range(1..=roll.sides))
                    .collect()
            };
            let (title, description) = roll.describe(&rolls);
            embed(ctx, "Let's roll the dice!", &description, &title).await?
        }
        Err(message) => error_embed(ctx, &message).await?,
    };
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[derive(Debug, PartialEq)]
struct Roll {
    count: u32,
    sides: u32,
    modifier: i64,
}

impl Roll {
    const MAX_DICE: u32 = 100;
    const MAX_SIDES: u32 = 1000;

    /// `[N]dM[+K|-K]`, case-insensitive, spaces ignored.
    fn parse(input: &str) -> Result<Roll, String> {
        let invalid = || {
            format!(
                "\"{}\" is not dice notation (e.g. d20, 2d6 or 3d8+2)!",
                input.trim()
            )
        };
        let text: String = input
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();
        let (count, rest) = text.split_once('d').ok_or_else(invalid)?;
        let (sides, modifier) = match rest.find(['+', '-']) {
            Some(at) => (&rest[..at], &rest[at..]),
            None => (rest, ""),
        };

        let count = if count.is_empty() {
            1
        } else {
            count.parse().map_err(|_| invalid())?
        };
        let sides: u32 = sides.parse().map_err(|_| invalid())?;
        let modifier: i64 = if modifier.is_empty() {
            0
        } else {
            modifier.parse().map_err(|_| invalid())?
        };

        if !(1..=Self::MAX_DICE).contains(&count) {
            return Err(format!("Roll between 1 and {} dice!", Self::MAX_DICE));
        }
        if !(2..=Self::MAX_SIDES).contains(&sides) {
            return Err(format!("Dice need 2 to {} sides!", Self::MAX_SIDES));
        }
        if modifier.abs() > 1_000_000 {
            return Err("That modifier is a bit much!".to_string());
        }
        Ok(Roll {
            count,
            sides,
            modifier,
        })
    }

    /// Embed title (the total) and description (the notation and each die).
    fn describe(&self, rolls: &[u32]) -> (String, String) {
        let total = rolls.iter().map(|&r| i64::from(r)).sum::<i64>() + self.modifier;
        let modifier = match self.modifier {
            0 => String::new(),
            m if m > 0 => format!("+{m}"),
            m => m.to_string(),
        };
        let notation = format!("{}d{}{}", self.count, self.sides, modifier);
        let description = if rolls.len() == 1 && self.modifier == 0 {
            format!("🎲 {notation}")
        } else {
            let each = rolls
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("🎲 {notation}: [{each}]{modifier}")
        };
        (format!("You rolled {total}"), description)
    }
}

#[cfg(test)]
mod tests {
    use super::Roll;

    fn roll(count: u32, sides: u32, modifier: i64) -> Roll {
        Roll {
            count,
            sides,
            modifier,
        }
    }

    #[test]
    fn notation_is_parsed() {
        assert_eq!(Roll::parse("d20"), Ok(roll(1, 20, 0)));
        assert_eq!(Roll::parse("2D6"), Ok(roll(2, 6, 0)));
        assert_eq!(Roll::parse(" 3d8 + 2 "), Ok(roll(3, 8, 2)));
        assert_eq!(Roll::parse("4d10-3"), Ok(roll(4, 10, -3)));
    }

    #[test]
    fn bad_or_huge_notation_is_rejected() {
        for input in [
            "", "20", "d", "2d", "xd6", "2d6+", "1d1", "101d6", "1d1001", "0d6", "d6*2",
        ] {
            assert!(Roll::parse(input).is_err(), "{input:?} was accepted");
        }
    }

    #[test]
    fn totals_include_the_modifier() {
        assert_eq!(
            roll(1, 6, 0).describe(&[4]),
            ("You rolled 4".to_string(), "🎲 1d6".to_string())
        );
        assert_eq!(
            roll(3, 8, -2).describe(&[1, 8, 5]),
            (
                "You rolled 12".to_string(),
                "🎲 3d8-2: [1, 8, 5]-2".to_string()
            )
        );
    }
}
