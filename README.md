# HTTP Cache

An HTTP server that proxies requests to a backend and stores responses in a cache backend.

- Currently only filesystem and gcs cache are supported.
- Caches all requests forever regardless of HTTP cache semantics.
- Experimental, do not use in production.
