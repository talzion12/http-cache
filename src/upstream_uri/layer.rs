use std::task::{Context, Poll};

use http::{header, Request, Response, StatusCode, Uri};
use hyper::Body;

use super::get_upstream_url::{get_upstream_uri, UPSTREAM_URL_HEADER_KEY};

pub struct ExtractUpstreamUriLayer {
    base_uri: Option<Uri>,
}

impl ExtractUpstreamUriLayer {
    pub fn new(base_uri: Option<Uri>) -> Self {
        Self { base_uri }
    }
}

impl<S: Clone> tower::Layer<S> for ExtractUpstreamUriLayer {
    type Service = ExtractUpstreamUriService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ExtractUpstreamUriService {
            inner,
            base_uri: self.base_uri.clone(),
        }
    }
}

pub struct ExtractUpstreamUriService<S> {
    inner: S,
    base_uri: Option<Uri>,
}

impl<S> Clone for ExtractUpstreamUriService<S>
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

impl<S> tower::Service<Request<Body>> for ExtractUpstreamUriService<S>
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

        futures::future::Either::Left(self.inner.call(request))
    }
}

pub struct UpstreamUriExt(pub Uri);
