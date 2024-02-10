use crate::{Context, Error};

use songbird::input::{Compose, YoutubeDl};
use songbird::events::TrackEvent;

use crate::commands::music::misc::TrackErrorNotifier;
use crate::http::HttpKey;

#[poise::command(prefix_command, slash_command)]
pub async fn play(ctx: Context<'_>, url: String) -> Result<(), Error> {
    let is_search = !url.starts_with("http");

    let guild_id = ctx.guild_id().unwrap();
    let channel_id = ctx.guild().unwrap().voice_states.get(&ctx.author().id).and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.say("Not in a voice channel").await?;
        
            return Ok(());
        }
    };

    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird Voice placed at init")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        let mut handler = handler_lock.lock().await;
       
        // if let Err(err) = handler.deafen(true).await {println!("Failed to deafen: {:?}", err)};
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);

        let mut src = if is_search {
            println!("ytsearch:{}", url);
            YoutubeDl::new_ytdl_like("yt-dlp", http_client, format!("ytsearch:{}", url))
        } else {
            YoutubeDl::new_ytdl_like("yt-dlp", http_client, url)
        };
        
        let _ = handler.enqueue_input(src.clone().into()).await;
        
        let _metadata = src.aux_metadata().await.unwrap();

        // ctx.say(format!("Playing song: {}", metadata.title.unwrap())).await?;
    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}
