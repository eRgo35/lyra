use serenity::framework::standard::Args;
use serenity::framework::standard::{
    macros::command,
    CommandResult,
};

use reqwest::Client as HttpClient;

use serenity::client::Context;

use serenity::prelude::TypeMapKey;

use serenity::model::prelude::*;

use songbird::input::YoutubeDl;

use crate::commands::misc::check_msg;

pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

#[command]
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

    let guild_id = msg.guild_id.unwrap();

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let src = if do_search {
            YoutubeDl::new(http_client, url)
        } else {
            YoutubeDl::new(http_client, url)
        };

        let _ = handler.play_input(src.clone().into());

        check_msg(msg.channel_id.say(&ctx.http, "Playing song").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel!")
                .await,
        );
    }

    Ok(())
}
