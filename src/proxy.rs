use std::task::{Context, Poll};

use hyper::{client::ResponseFuture, Body, Request, Uri};

pub struct ProxyService<C, B> {
    uri: Uri,
    client: hyper::Client<C, B>,
}

impl<C, B> ProxyService<C, B> {
    pub fn new(uri: Uri, client: hyper::Client<C, B>) -> Self {
        Self { uri, client }
    }
}

impl<C, B> Clone for ProxyService<C, B>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            uri: self.uri.clone(),
            client: self.client.clone(),
        }
    }
}

impl<C, B> tower::Service<Request<B>> for ProxyService<C, B>
where
    C: hyper::client::connect::Connect + Clone + Send + Sync + 'static,
    B: hyper::body::HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = hyper::Response<Body>;
    type Error = hyper::Error;
    type Future = ResponseFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let mut parts = self.uri.clone().into_parts();
        parts.path_and_query = request.uri().path_and_query().cloned();
        *request.uri_mut() = Uri::from_parts(parts).unwrap();

        tracing::debug!("Proxying request {} {}", request.method(), request.uri());

        self.client.request(request)
    }
}
