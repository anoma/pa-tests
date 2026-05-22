use clap::Parser;

#[derive(Parser, Clone, Debug)]
pub struct E2eConfig {
    #[arg(long, env = "ALCHEMY_API_KEY")]
    pub alchemy_api_key: String,

    #[arg(long, env = "QUEUE_BASE_URL")]
    pub queue_base_url: String,

    #[arg(long, env = "QUEUE_AUTH_TOKEN")]
    pub queue_auth_token: Option<String>,
}
