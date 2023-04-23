use std::net::{IpAddr, SocketAddr};

use cache::CachingLayer;
use clap::Parser;
use hyper::{client::HttpConnector, Body, Uri};
use hyper_rustls::HttpsConnector;
use proxy::ProxyService;
use tower::{make::Shared, ServiceBuilder};
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;

mod cache;
mod proxy;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, env = "UPSTREAM_URL")]
    upstream: Uri,

    #[clap(long, env = "CACHE_URL")]
    cache_url: Url,

    #[clap(long, env = "CACHE_URL_2")]
    cache_url_2: Option<Url>,

    #[clap(long, env = "HOST", default_value = "0.0.0.0")]
    host: IpAddr,

    #[clap(long, env = "PORT", default_value = "8080")]
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

    let cache_layer = CachingLayer::from_url(&args.cache_url).await?;

    let cache_layer_2 = tower::util::option_layer(if let Some(cache_url_2) = &args.cache_url_2 {
        Some(CachingLayer::from_url(cache_url_2).await?)
    } else {
        None
    });

    let client = hyper::Client::builder().build(
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build(),
    );
    let proxy = ProxyService::<HttpsConnector<HttpConnector>, Body>::new(args.upstream, client);

    let service = ServiceBuilder::new()
        .layer(cache_layer_2)
        .layer(cache_layer)
        .service(proxy);
    let make_service = Shared::new(service);

    let listen_addr = SocketAddr::new(args.host, args.port);

    tracing::info!("Listening on {listen_addr}");

    hyper::Server::bind(&listen_addr)
        .serve(make_service)
        .await?;

    Ok(())
}
