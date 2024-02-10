use reqwest::Client as HttpClient;
use poise::serenity_prelude::prelude::TypeMapKey;

pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}
