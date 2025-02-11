use clap::Parser;
use dotenv::dotenv;
use handler::Handler;
use sea_orm::{Database, DatabaseConnection};
use serenity::all::{ChannelId, GuildId, Http, Webhook};
use serenity::{all::GatewayIntents, futures::lock::Mutex, Client};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod handler;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(env)]
    bot_token: String,

    #[arg(long, default_value = "false")]
    debug: bool,

    #[arg(env)]
    db_url: String,

    #[arg(env)]
    board_channel_id: u64,

    #[arg(env)]
    guild_id: u64,

    #[arg(env)]
    board_channel_webhook: String,

    #[arg(env, default_value = "3")]
    min_star_count: u64,
}

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let args = Args::parse();

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_file(args.debug)
                .with_line_number(args.debug),
        )
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    debug!("connecting to database");

    let db: DatabaseConnection = Database::connect(args.db_url).await?;
    let http = Http::new("");
    let webhook = Webhook::from_url(&http, &args.board_channel_webhook).await?;

    let mut client = Client::builder(
        args.bot_token,
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_MESSAGE_REACTIONS,
    )
    .event_handler(Handler {
        db: Mutex::new(db),
        board_channel_id: ChannelId::new(args.board_channel_id),
        guild_id: GuildId::new(args.guild_id),
        webhook,
        min_count: args.min_star_count,
    })
    .await
    .expect("Err creating client");

    client.start().await?;

    Ok(())
}
