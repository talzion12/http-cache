use std::{task::{Context, Poll}, sync::Arc};

use futures::{future::BoxFuture, FutureExt};
use http::{Request, Response, StatusCode};
use hyper::Body;

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
        S: tower::Service<Request<Body>, Response = Response<Body>, Error = hyper::Error>,
        C: Cache {
    let key = request.uri().to_string();
    let cache_res = cache.get(key.as_str()).await;

    match cache_res {
        Ok(Some(response)) => {
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(response))
                .unwrap_or_else(|err| 
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::empty())
                        .unwrap()
                ));
        },
        Ok(None) => {
            let result = inner.call(request).await;
            if let Ok(response) = &result {
                match cache.set(key.as_str(), "value".to_string()).await {
                    Err(error) => {
                        tracing::error!(%error, "Failed to get from cache");
                    },
                    _ => {}
                };
            };
            result
        },
        Err(err) => {
            println!("Err {:?}", err);
            panic!("");
        },
    }
}