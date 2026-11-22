use poise::CreateReply;
use rand::seq::IndexedRandom;

use crate::{commands::embeds::embed, Context, Error};

#[derive(Debug, Clone, Copy, PartialEq, poise::ChoiceParameter)]
pub enum Hand {
    #[name = "rock"]
    Rock,
    #[name = "paper"]
    Paper,
    #[name = "scissors"]
    Scissors,
}

impl Hand {
    fn emoji(self) -> &'static str {
        match self {
            Hand::Rock => "🪨",
            Hand::Paper => "📄",
            Hand::Scissors => "✂️",
        }
    }

    fn beats(self, other: Hand) -> bool {
        matches!(
            (self, other),
            (Hand::Rock, Hand::Scissors)
                | (Hand::Paper, Hand::Rock)
                | (Hand::Scissors, Hand::Paper)
        )
    }
}

/// Plays rock, paper, scissors against the bot
#[poise::command(prefix_command, slash_command, category = "Fun")]
pub async fn rps(ctx: Context<'_>, #[description = "Your hand"] hand: Hand) -> Result<(), Error> {
    let mine = *[Hand::Rock, Hand::Paper, Hand::Scissors]
        .choose(&mut rand::rng())
        .unwrap_or(&Hand::Rock);
    let outcome = if hand.beats(mine) {
        "You win!"
    } else if mine.beats(hand) {
        "I win!"
    } else {
        "It's a draw!"
    };
    let description = format!("You: {} · Me: {}", hand.emoji(), mine.emoji());
    ctx.send(
        CreateReply::default()
            .embed(embed(ctx, "Rock, paper, scissors", &description, outcome).await?),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Hand::*;

    #[test]
    fn each_hand_beats_exactly_one_other() {
        for hand in [Rock, Paper, Scissors] {
            let wins = [Rock, Paper, Scissors]
                .iter()
                .filter(|o| hand.beats(**o))
                .count();
            assert_eq!(wins, 1, "{hand:?}");
            assert!(!hand.beats(hand));
        }
        assert!(Paper.beats(Rock));
    }
}
