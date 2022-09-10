use std::net::SocketAddr;

use cache::{CachingLayer, RedisCache};
use clap::Parser;
use hyper::Uri;
use proxy::ProxyService;
use tower::{ServiceBuilder, make::Shared};
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, EnvFilter, util::SubscriberInitExt};

mod cache;
mod proxy;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
   upstream: Uri,
   redis: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv::dotenv()?;

    let args = Args::parse();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .with(ErrorLayer::default())
        .init();

    color_eyre::install()?;

    let cache =
        RedisCache::new(
            args.redis.as_str()
        )
        .await?
        .with_prefix("http-cache".into());

    let proxy = ProxyService::new(args.upstream);

    let service = ServiceBuilder::new()
        .layer(CachingLayer::new(cache))
        .service(proxy);

    let make_service = Shared::new(service);

    let listen_addr: SocketAddr = "0.0.0.0:3200".parse()?;

    tracing::info!("Listening on {listen_addr}");

    hyper::Server::bind(&listen_addr)
        .serve(make_service)
        .await?;
    
    Ok(())
}