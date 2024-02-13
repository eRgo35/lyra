use crate::{commands::embeds::{error_embed, embed}, Context, Error};
use poise::CreateReply;
use songbird::tracks::LoopState;

/// Loops currently playing song provided amount of times; \
/// aliases: repeat, loop, while, for
#[poise::command(
    prefix_command, 
    slash_command,
    aliases("loop", "while", "for"),
    category = "Music"
)]
pub async fn repeat(
    ctx: Context<'_>, 
    #[description = "How many times"] #[rest] times: usize
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();


    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        let track = queue.current().unwrap().get_info().await;
        let is_looped = track.unwrap().loops;

        match is_looped {
            LoopState::Infinite => {
                let _ = queue.current().unwrap().disable_loop();

                ctx.send(
                    CreateReply::default().embed(embed(ctx, "Song Unlooped!", "", "").await.unwrap())
                ).await?;
            }
            LoopState::Finite(_) => { 
                if times == 0 {
                    let _ = queue.current().unwrap().disable_loop();

                    ctx.send(
                        CreateReply::default().embed(embed(ctx, "Song Unlooped!", "", "").await.unwrap())
                    ).await?;
                }
                else if times < 100 {
                    let _ = queue.current().unwrap().loop_for(times);
                    ctx.send(
                        CreateReply::default().embed(embed(ctx, &format!("Song looped {} times!", times), "You definitelly love this song!", "").await.unwrap())
                    ).await?;
                }
                else {
                    let _ = queue.current().unwrap().enable_loop();
                    ctx.send(
                        CreateReply::default().embed(embed(ctx, "Song looped forever!", "A very long time!", "").await.unwrap())
                    ).await?;
                }
            }
        }
    } else { 
        let msg = "I am not in a voice channel!";
        ctx.send(
            CreateReply::default().embed(error_embed(ctx, msg).await.unwrap())
        ).await?;
    }

    Ok(())
}
