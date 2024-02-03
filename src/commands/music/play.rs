use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use reqwest::Client as HttpClient;
use songbird::input::YoutubeDl;
use songbird::events::TrackEvent;

use crate::commands::{misc::check_msg, music::misc::TrackErrorNotifier};

pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

#[command]
#[aliases(p)]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Must provide a URL to a video or audio")
                    .await,
            );

            return Ok(());
        }
    };

    let do_search = !url.starts_with("http");

    let (guild_id, channel_id) = {
        let guild = msg.guild(&ctx.cache).unwrap();
        let channel_id = guild
            .voice_states
            .get(&msg.author.id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild.id, channel_id)
    };
    
    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);
        
            return Ok(());
        }
    };

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice placed at init")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        let mut handler = handler_lock.lock().await;
        
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);

        let src = if do_search {
            YoutubeDl::new_ytdl_like("yt-dlp", http_client, url)
        } else {
            YoutubeDl::new(http_client, url)
        };
        
        let _ = handler.enqueue_input(src.clone().into()).await;

        // let _ = handler.play_input(src.clone().into());

        check_msg(msg.channel_id.say(&ctx.http, "Playing song").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}
