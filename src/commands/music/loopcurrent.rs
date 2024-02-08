use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::tracks::LoopState;

use crate::commands::misc::check_msg;

#[command]
#[aliases(loop)]
#[only_in(guilds)]
async fn loopcurrent(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Client placed at init")
        .clone();


    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        let track = queue.current().unwrap().get_info().await;
        let is_looped = track.unwrap().loops;


        let count = match args.single::<usize>() {
            Ok(count) => count,
            Err(_) => 100,
        };

        match is_looped {
            LoopState::Infinite => {
                let _ = queue.current().unwrap().disable_loop();

                check_msg(
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!("Song unlooped."),
                        )
                    .await,
                );
            }
            LoopState::Finite(_) => { 
                if count < 100 {
                    let _ = queue.current().unwrap().loop_for(count);
                    check_msg(
                        msg.channel_id
                            .say(
                                &ctx.http,
                                format!("Song looped forever (a very long time)."),
                            )
                        .await,
                    )
                }
                else {
                    let _ = queue.current().unwrap().enable_loop();

                    check_msg(
                        msg.channel_id
                            .say(
                                &ctx.http,
                                format!("Song looped {} times.", count),
                            )
                        .await,
                    )
                }

            }
        }
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
            .await,
        );
    }

    Ok(())
}
