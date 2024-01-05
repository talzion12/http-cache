use std::{io::ErrorKind, net::SocketAddr};

use cache::CachingLayer;
use clap::Parser;
use eyre::Context;
use http::Request;
use hyper::Body;
use proxy::ProxyService;
use tower::{make::Shared, util::option_layer, ServiceBuilder};
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

mod cache;
mod options;
mod proxy;
mod upstream_uri;

use options::{LogFormat, Options};
use upstream_uri::layer::ExtractUpstreamUriLayer;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    match dotenv::dotenv() {
        Ok(path) => {
            tracing::info!("Loaded env from {path:?}")
        }
        Err(dotenv::Error::Io(error)) if error.kind() == ErrorKind::NotFound => {
            tracing::debug!("Not loading .env because it wasn't found");
        }
        Err(error) => return Err(error).context("Failed to load .env"),
    };

    let options = Options::parse();

    init_tracing(&options);

    color_eyre::install()?;

    let cache_layer = CachingLayer::from_url(&options.cache_url).await?;

    let cache_layer_2 = option_layer(if let Some(cache_url_2) = &options.cache_url_2 {
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
    let proxy = ProxyService::new(client);

    let service = ServiceBuilder::new()
        .layer(
            tower_http::trace::TraceLayer::new_for_http().make_span_with(
                |request: &Request<Body>| {
                    tracing::info_span!(
                        "http-request",
                        scheme = request.uri().scheme_str(),
                        path = request.uri().path(),
                        query = request.uri().query()
                    )
                },
            ),
        )
        .layer(ExtractUpstreamUriLayer::new(options.upstream))
        .layer(cache_layer_2)
        .layer(cache_layer)
        .service(proxy);
    let make_service = Shared::new(service);

    let listen_addr = SocketAddr::new(options.host, options.port);

    tracing::info!("Listening on {listen_addr}");

    hyper::Server::bind(&listen_addr)
        .serve(make_service)
        .await?;

    Ok(())
}

fn init_tracing(opts: &Options) {
    tracing_subscriber::registry()
        .with(match opts.log_format {
            LogFormat::Json => tracing_subscriber::fmt::layer().json().boxed(),
            LogFormat::Pretty => tracing_subscriber::fmt::layer().pretty().boxed(),
            LogFormat::Text => tracing_subscriber::fmt::layer().boxed(),
        })
        .with(ErrorLayer::default())
        .with(EnvFilter::from_default_env())
        .init()
}
