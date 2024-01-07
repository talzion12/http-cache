use std::net::IpAddr;

use hyper::Uri;
use url::Url;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Options {
    #[clap(long, env = "UPSTREAM_URL")]
    pub upstream: Option<Uri>,

    #[clap(long, env = "CACHE_URL")]
    pub cache_url: Url,

    #[clap(long, env = "CACHE_URL_2")]
    pub cache_url_2: Option<Url>,

    #[clap(long, env = "HOST", default_value = "0.0.0.0")]
    pub host: IpAddr,

    #[clap(long, env = "PORT", default_value = "8080")]
    pub port: u16,

    #[clap(long, env = "LOG_FORMAT", default_value = "text")]
    pub log_format: LogFormat,

    #[clap(long, env = "HTTP2_DISABLED")]
    pub http2_disabled: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LogFormat {
    Json,
    Text,
    Pretty,
}
