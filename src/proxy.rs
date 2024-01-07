use std::task::{Context, Poll};

use hyper::{client::ResponseFuture, Body, Request};

use crate::metadata::layer::UpstreamUriExt;

pub struct ProxyService<C> {
    client: hyper::Client<C>,
}

impl<C> ProxyService<C> {
    pub fn new(client: hyper::Client<C>) -> Self {
        Self { client }
    }
}

impl<C> Clone for ProxyService<C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl<C> tower::Service<Request<Body>> for ProxyService<C>
where
    C: hyper::client::connect::Connect + Clone + Send + Sync + 'static,
{
    type Response = hyper::Response<Body>;
    type Error = hyper::Error;
    type Future = ResponseFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        let UpstreamUriExt(upstream_uri) = request
            .extensions_mut()
            .remove()
            .expect("Upstream uri extension is missing");

        *request.uri_mut() = upstream_uri;

        tracing::debug!("Proxying request {} {}", request.method(), request.uri());

        self.client.request(request)
    }
}
