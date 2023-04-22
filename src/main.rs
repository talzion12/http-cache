use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use cache::{Cache, CachingLayer, FsCache, RedisCache};
use clap::Parser;
use hyper::Uri;
use proxy::ProxyService;
use tower::{make::Shared, ServiceBuilder};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use url::Url;

mod cache;
mod proxy;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    upstream: Uri,
    cache_url: Url,

    #[clap(long, default_value = "0.0.0.0")]
    host: IpAddr,

    #[clap(long, default_value = "3200")]
    port: u16,
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

    let cache = create_cache(args.cache_url).await?;

    let proxy = ProxyService::new(args.upstream);

    let service = ServiceBuilder::new()
        .layer(CachingLayer::new(cache))
        .service(proxy);

    let make_service = Shared::new(service);

    let listen_addr = SocketAddr::new(args.host, args.port);

    tracing::info!("Listening on {listen_addr}");

    hyper::Server::bind(&listen_addr)
        .serve(make_service)
        .await?;

    Ok(())
}

async fn create_cache(url: Url) -> color_eyre::Result<Box<dyn Cache>> {
    let result: Box<dyn Cache> = match url.scheme() {
        "file" => Box::new(FsCache::new(PathBuf::from(url.path()))),
        "redis" => Box::new(RedisCache::new(url.as_str()).await?),
        _ => color_eyre::eyre::bail!("Scheme must be defined"),
    };
    Ok(result)
}
