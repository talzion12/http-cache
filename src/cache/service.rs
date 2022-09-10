use std::{task::{Context, Poll}, sync::Arc};

use futures::{future::BoxFuture, FutureExt};
use http::{Request, Response, StatusCode};
use hyper::{Body, body::to_bytes};

use super::base::Cache;

pub struct CachingLayer<C> {
    cache: Arc<C>,
}

impl<C> CachingLayer<C> {
    pub fn new(cache: C) -> Self {
        Self {cache: Arc::from(cache)}
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

impl <S, C> tower::Service<Request<Body>> for CachingService<S, C>
where
    S: tower::Service<
        Request<Body>,
        Response = Response<Body>,
        Error = hyper::Error,
    > + Send + Sync,
    S: Clone + 'static,
    S::Future: Send,
    C: Cache + Send + Sync + 'static {
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<S::Response, S::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[tracing::instrument(skip(self))]
    fn call(&mut self, request: Request<Body>) -> Self::Future {
        call_impl(
            self.cache.clone(),
            self.inner.clone(),
            request,
        ).boxed()
    }
}

async fn call_impl<S, C>(
    cache: Arc<C>,
    mut inner: S,
    request: Request<Body>,
) -> Result<Response<Body>, hyper::Error>
    where
        S: tower::Service<
            Request<Body>,
            Response = Response<Body>,
            Error = hyper::Error,
        >,
        C: Cache {
    let key = request.uri().to_string();
    let cache_result = cache.get(key.as_str()).await;

    match cache_result {
        Ok(Some(cached)) => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(cached))
                .unwrap_or_else(|error| {
                    tracing::error!(%error, "Failed to build a proxy request");
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::empty())
                        .unwrap()
                }))
        },
        Ok(None) => {
            let response = inner.call(request).await?;

            if response.status().is_success() {
                let (parts, body) = response.into_parts();
                let body_bytes = to_bytes(body).await?;
                match cache.set(key.as_str(), &body_bytes).await {
                    Err(error) => {
                        tracing::error!(%error, "Failed to set");
                    },
                    _ => {}
                };

                Ok(Response::from_parts(parts, Body::from(body_bytes)))
            }
            else {
                Ok(response)
            }
        },
        Err(error) => {
            tracing::error!(%error, "Failed to read from cache");
            Ok(
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap()
            )
        },
    }
}