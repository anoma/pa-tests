use std::env;

use anyhow::Context;

pub struct E2eConfig {
    pub alchemy_api_key: String,
    pub queue_base_url: String,
    pub queue_auth_token: String,
}

impl E2eConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            alchemy_api_key: env::var("ALCHEMY_API_KEY")
                .context("failed to parse ALCHEMY_API_KEY")?,
            queue_base_url: env::var("QUEUE_BASE_URL").context("failed to parse QUEUE_BASE_URL")?,
            queue_auth_token: env::var("QUEUE_AUTH_TOKEN")
                .context("failed to parse QUEUE_AUTH_TOKEN")?,
        })
    }
}
