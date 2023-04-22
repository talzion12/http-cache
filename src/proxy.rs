use std::task::{Context, Poll};

use hyper::{
    client::{HttpConnector, ResponseFuture},
    Body, Request, Uri,
};
use hyper_rustls::HttpsConnector;

#[derive(Clone)]
pub struct ProxyService {
    uri: Uri,
    client: hyper::Client<HttpsConnector<HttpConnector>>,
}

impl ProxyService {
    pub fn new(uri: Uri) -> Self {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build();

        let client = hyper::Client::builder().build(connector);

        Self { uri, client }
    }
}

impl tower::Service<Request<Body>> for ProxyService {
    type Response = hyper::Response<Body>;
    type Error = hyper::Error;
    type Future = ResponseFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[tracing::instrument(skip(self))]
    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        let mut parts = self.uri.clone().into_parts();
        parts.path_and_query = request.uri().path_and_query().cloned();
        *request.uri_mut() = Uri::from_parts(parts).unwrap();

        tracing::debug!("Proxying request {} {}", request.method(), request.uri());

        self.client.request(request)
    }
}
