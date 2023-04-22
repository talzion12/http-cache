use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::{channel::mpsc::channel, future::BoxFuture, FutureExt, SinkExt, StreamExt};
use http::{Request, Response, StatusCode};
use hyper::{body::Bytes, Body};

use super::storage::Cache;

pub struct CachingLayer<C> {
    cache: Arc<C>,
}

impl<C> CachingLayer<C> {
    pub fn new(cache: C) -> Self {
        Self {
            cache: Arc::from(cache),
        }
    }
}

impl<S: Clone, C: Cache> tower::Layer<S> for CachingLayer<C> {
    type Service = CachingService<S, C>;

    fn layer(&self, inner: S) -> Self::Service {
        CachingService {
            inner,
            cache: self.cache.clone(),
        }
    }
}

pub struct CachingService<S, C> {
    inner: S,
    cache: Arc<C>,
}

impl<S: Clone, C> Clone for CachingService<S, C> {
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
    C: Cache + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<S::Response, S::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[tracing::instrument(skip(self))]
    fn call(&mut self, request: Request<Body>) -> Self::Future {
        call_impl(self.cache.clone(), self.inner.clone(), request).boxed()
    }
}

async fn call_impl<S, C>(
    cache: Arc<C>,
    mut inner: S,
    request: Request<Body>,
) -> Result<Response<Body>, hyper::Error>
where
    S: tower::Service<Request<Body>, Response = Response<Body>, Error = hyper::Error>,
    C: Cache + 'static,
{
    let key = request.uri().clone();
    tracing::debug!("Received request for {key}");
    let cache_result = cache.get(&key).await;

    match cache_result {
        Ok(Some(cached)) => {
            tracing::debug!("Found in cache");
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::wrap_stream(cached))
                .unwrap_or_else(|error| {
                    tracing::error!(%error, "Failed to build a proxy request");
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap()
                }))
        }
        Ok(None) => {
            tracing::debug!("Not found in cache");
            let response = inner.call(request).await?;

            if response.status().is_success() {
                let (parts, body) = response.into_parts();

                let (sender, receiver) = channel::<Bytes>(10);
                let cache_cloned = cache.clone();

                tokio::spawn(async move {
                    match cache_cloned.set(&key, receiver).await {
                        Ok(()) => tracing::debug!("Wrote to cache"),
                        Err(err) => tracing::error!("Failed to write to cache {err:?}"),
                    }
                });

                let res_body =
                    Body::wrap_stream(body.then(move |part| send_part(part, sender.clone())));

                Ok(Response::from_parts(parts, res_body))
            } else {
                Ok(response)
            }
        }
        Err(error) => {
            tracing::error!(%error, "Failed to read from cache");
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap())
        }
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
