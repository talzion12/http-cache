use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::{channel::mpsc::channel, future::BoxFuture, FutureExt, SinkExt, StreamExt};
use http::{Request, Response, StatusCode};
use hyper::{body::Bytes, Body};
use phf::phf_set;
use url::Url;

use crate::cache::metadata::CacheMetadata;

use super::{create_cache_storage_from_url, storage::Cache, GetBody};

pub struct CachingLayer<C: ?Sized> {
    cache: Arc<C>,
}

impl<C: ?Sized> CachingLayer<C> {
    pub fn new(cache: impl Into<Arc<C>>) -> Self {
        Self {
            cache: cache.into(),
        }
    }
}

impl CachingLayer<dyn Cache> {
    pub async fn from_url(url: &Url) -> color_eyre::Result<Self> {
        let storage = create_cache_storage_from_url(url).await?;
        Ok(Self::new(storage))
    }
}

impl<S: Clone, C: Cache + ?Sized> tower::Layer<S> for CachingLayer<C> {
    type Service = CachingService<S, C>;

    fn layer(&self, inner: S) -> Self::Service {
        CachingService {
            inner,
            cache: self.cache.clone(),
        }
    }
}

pub struct CachingService<S, C: ?Sized> {
    inner: S,
    cache: Arc<C>,
}

impl<S, C> CachingService<S, C>
where
    S: tower::Service<Request<Body>, Response = Response<Body>, Error = hyper::Error> + Send + Sync,
    C: Cache + 'static + ?Sized,
{
    async fn on_request(&mut self, request: Request<Body>) -> Result<Response<Body>, hyper::Error>
    where
        S: tower::Service<Request<Body>, Response = Response<Body>, Error = hyper::Error>,
        C: Cache + 'static,
    {
        let uri = request.uri();
        tracing::debug!("Received request for {uri}");
        let cache_result = self.cache.get(uri).await;

        match cache_result {
            Ok(Some((metadata, body))) => self.on_cache_hit(metadata, body).await,
            Ok(None) => self.on_cache_miss(request).await,
            Err(error) => {
                tracing::error!(%error, "Failed to read from cache");
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap())
            }
        }
    }

    async fn on_cache_hit(
        &self,
        metadata: CacheMetadata,
        body: GetBody,
    ) -> Result<Response<Body>, hyper::Error> {
        tracing::debug!("Cache hit");

        let mut builder = Response::builder().status(metadata.status);

        for (key, value) in metadata.headers {
            tracing::debug!("Setting key {key}");
            builder = builder.header(key, value);
        }

        let body = builder
            .body(Body::wrap_stream(body))
            .unwrap_or_else(|error| {
                tracing::error!(%error, "Failed to build a proxy request");
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap()
            });

        Ok(body)
    }

    async fn on_cache_miss(
        &mut self,
        request: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        tracing::debug!("Cache miss");

        let uri = request.uri().clone();
        let response = self.inner.call(request).await?;

        if !response.status().is_success() {
            tracing::debug!(
                "Not caching response because the status code is {}",
                response.status()
            );
            return Ok(response);
        }

        let (parts, body) = response.into_parts();

        let metadata = CacheMetadata {
            status: parts.status.as_u16(),
            headers: parts
                .headers
                .iter()
                .filter(|(key, _)| HEADERS_TO_KEEP.contains(key.as_str()))
                .map(|(key, value)| (key.to_string(), value.as_bytes().to_vec()))
                .collect(),
        };

        let (sender, receiver) = channel::<Bytes>(10);
        let cache_cloned = self.cache.clone();

        tokio::spawn(async move {
            match cache_cloned.set(&uri, receiver, metadata).await {
                Ok(()) => tracing::debug!("Wrote to cache"),
                Err(err) => tracing::error!("Failed to write to cache {err:?}"),
            }
        });

        let res_body = Body::wrap_stream(body.then(move |part| {
            tracing::debug!("Received part");
            send_part(part, sender.clone())
        }));

        Ok(Response::from_parts(parts, res_body))
    }
}

impl<S: Clone, C: ?Sized> Clone for CachingService<S, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl<S, C> tower::Service<Request<Body>> for CachingService<S, C>
where
    S: tower::Service<Request<Body>, Response = Response<Body>, Error = hyper::Error> + Send + Sync,
    S: Clone + 'static,
    S::Future: Send,
    C: Cache + Send + Sync + 'static + ?Sized,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<S::Response, S::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[tracing::instrument(skip(self))]
    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let mut c = self.clone();
        async move { c.on_request(request).await }.boxed()
    }
}

async fn send_part(
    part: Result<Bytes, hyper::Error>,
    mut sender: futures::channel::mpsc::Sender<Bytes>,
) -> Result<Bytes, hyper::Error> {
    if let Ok(part) = &part {
        match sender.send(part.clone()).await {
            Ok(()) => (),
            Err(error) => {
                tracing::error!("Failed to write to channel {error:?}")
            }
        };
    }

    part
}

static HEADERS_TO_KEEP: phf::Set<&'static str> = phf_set! {
    "content-encoding",
    "content-type"
};
