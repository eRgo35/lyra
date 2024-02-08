use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use reqwest::Client as HttpClient;
use songbird::input::{Compose, YoutubeDl};
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

    let is_search = !url.starts_with("http");

    let guild_id = msg.guild_id.unwrap();
    let channel_id = msg.guild(&ctx.cache).unwrap().voice_states.get(&msg.author.id).and_then(|voice_state| voice_state.channel_id);

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
       
        // if let Err(err) = handler.deafen(true).await {println!("Failed to deafen: {:?}", err)};
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);

        let mut src = if is_search {
            println!("ytsearch:{}", url);
            YoutubeDl::new_ytdl_like("yt-dlp", http_client, format!("ytsearch:{}", args.clone().message()))
        } else {
            YoutubeDl::new_ytdl_like("yt-dlp", http_client, url)
        };
        
        let _ = handler.enqueue_input(src.clone().into()).await;
        
        let metadata = src.aux_metadata().await.unwrap();

        check_msg(msg.channel_id.say(&ctx.http, format!("Playing song: {}", metadata.title.unwrap())).await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}
