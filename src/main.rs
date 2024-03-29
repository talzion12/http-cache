use std::{io::ErrorKind, net::SocketAddr};

use cache::CachingLayer;
use clap::Parser;
use eyre::Context;
use http::Request;
use hyper::{client::HttpConnector, Body, Client};
use hyper_rustls::HttpsConnector;
use proxy::ProxyService;
use tower::{make::Shared, util::option_layer, ServiceBuilder};
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

mod cache;
mod metadata;
mod options;
mod proxy;
#[cfg(test)]
mod test;

use metadata::layer::ExtractMetadataLayer;
use options::{LogFormat, Options};

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

    let cache_layer = CachingLayer::from_url(&options.cache_url)?;

    let cache_layer_2 = option_layer(if let Some(cache_url_2) = &options.cache_url_2 {
        Some(CachingLayer::from_url(cache_url_2)?)
    } else {
        None
    });

    let client = init_client(&options);
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
        .layer(ExtractMetadataLayer::new(options.upstream))
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

fn init_client(options: &Options) -> Client<HttpsConnector<HttpConnector>> {
    let connector_builder = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1();
    if options.http2_disabled {
        hyper::Client::builder().build(connector_builder.build())
    } else {
        hyper::Client::builder().build(connector_builder.enable_http2().build())
    }
}
