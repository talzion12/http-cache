use std::{
    sync::Arc,
    task::{Context, Poll},
};

use http::{header, Request, Response, StatusCode, Uri};
use hyper::Body;

use super::{
    get_cache_key_prefix::{get_cache_key_prefix, CACHE_KEY_PREFIX_HEADER_KEY},
    get_upstream_url::{get_upstream_uri, UPSTREAM_URL_HEADER_KEY},
};

pub struct ExtractMetadataLayer {
    base_uri: Option<Uri>,
}

impl ExtractMetadataLayer {
    pub fn new(base_uri: Option<Uri>) -> Self {
        Self { base_uri }
    }
}

impl<S: Clone> tower::Layer<S> for ExtractMetadataLayer {
    type Service = ExtractMetadataLayerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ExtractMetadataLayerService {
            inner,
            base_uri: self.base_uri.clone(),
        }
    }
}

pub struct ExtractMetadataLayerService<S> {
    inner: S,
    base_uri: Option<Uri>,
}

impl<S> Clone for ExtractMetadataLayerService<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            base_uri: self.base_uri.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<S> tower::Service<Request<Body>> for ExtractMetadataLayerService<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::Either<
        S::Future,
        futures::future::Ready<Result<Self::Response, Self::Error>>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        let upstream_header_value = request.headers_mut().remove(UPSTREAM_URL_HEADER_KEY);
        let cache_key_header_value = request.headers_mut().remove(CACHE_KEY_PREFIX_HEADER_KEY);

        let upstream_uri =
            match get_upstream_uri(request.uri(), upstream_header_value, self.base_uri.as_ref()) {
                Ok(base_url) => base_url,
                Err(error) => {
                    return futures::future::Either::Right(futures::future::ready(Ok(
                        Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(
                                serde_json::json!({
                                    "error": serde_error::Error::new(&error)
                                })
                                .to_string()
                                .into(),
                            )
                            .unwrap(),
                    )))
                }
            };

        request
            .extensions_mut()
            .insert(UpstreamUriExt(upstream_uri));

        if let Some(cache_key_prefix) = get_cache_key_prefix(cache_key_header_value) {
            request
                .extensions_mut()
                .insert(CacheKeyPrefixExt(cache_key_prefix));
        }

        futures::future::Either::Left(self.inner.call(request))
    }
}

pub struct UpstreamUriExt(pub Uri);
pub struct CacheKeyPrefixExt(pub Arc<str>);
