use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser)]
#[command(version, about = "ZJHive's loopback ACPv2 transport adapter")]
struct Options {
    #[arg(long, default_value = "127.0.0.1:8766")]
    listen: SocketAddr,
    #[arg(long, env = "HIVE_URL", default_value = "http://127.0.0.1:8765")]
    url: String,
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let options = Options::parse();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    zjhive_bridge::serve(options.listen, options.url).await
}
