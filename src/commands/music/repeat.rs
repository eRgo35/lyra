use crate::{Context, Error};
use songbird::tracks::LoopState;

#[poise::command(prefix_command, slash_command)]
pub async fn repeat(ctx: Context<'_>, times: usize) -> Result<(), Error> {
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

                ctx.say("Song unlooped.").await?;
            }
            LoopState::Finite(_) => { 
                if times < 100 {
                    let _ = queue.current().unwrap().loop_for(times);
                    ctx.say("Song looped forever (a very long time)").await?;
                }
                else {
                    let _ = queue.current().unwrap().enable_loop();
                    ctx.say(format!("Song looped {} times.", times)).await?;
                }
            }
        }
    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}
