use std::str::FromStr;

use http::{header::ToStrError, uri::InvalidUri, HeaderValue, Uri};

pub const UPSTREAM_URL_HEADER_KEY: &'static str = "x-http-cache-upstream-uri";

pub fn get_upstream_uri(
    uri: &Uri,
    upstream_header_value: Option<HeaderValue>,
    base_url: Option<&Uri>,
) -> Result<Uri, GetBaseUrlError> {
    match get_upstream_uri_from_headers(upstream_header_value) {
        Ok(upstream_uri) => Ok(upstream_uri),
        Err(error) => {
            if let Some(base_url) = base_url {
                let mut parts = base_url.clone().into_parts();
                parts.path_and_query = uri.path_and_query().cloned();
                Ok(Uri::from_parts(parts).unwrap())
            } else {
                Err(GetBaseUrlError(error))
            }
        }
    }
}

fn get_upstream_uri_from_headers(
    upstream_header_value: Option<HeaderValue>,
) -> Result<Uri, GetBaseUrlFromHeadersError> {
    let base_url_value =
        upstream_header_value.ok_or(GetBaseUrlFromHeadersError::HeaderNotFound {
            header_name: UPSTREAM_URL_HEADER_KEY,
        })?;
    let base_url_str = base_url_value
        .to_str()
        .map_err(GetBaseUrlFromHeadersError::HeaderNotString)?;
    let base_url = Uri::from_str(base_url_str).map_err(GetBaseUrlFromHeadersError::HeaderNotUri)?;
    Ok(base_url)
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to read upstream url from request headers and upstream url not configured")]
pub struct GetBaseUrlError(#[source] GetBaseUrlFromHeadersError);

#[derive(thiserror::Error, Debug)]
pub enum GetBaseUrlFromHeadersError {
    #[error("{header_name} header not found")]
    HeaderNotFound { header_name: &'static str },
    #[error("Header value is not a valid string")]
    HeaderNotString(#[source] ToStrError),
    #[error("Header value is not a valid uri")]
    HeaderNotUri(#[source] InvalidUri),
}

#[cfg(test)]
mod test {
    use super::*;

    fn get_upstream_uri(
        request_uri: &str,
        upstream_header: Option<&str>,
        base_url: Option<&str>,
    ) -> eyre::Result<Uri> {
        let uri = Uri::from_str(request_uri)?;
        let upstream_header_value = upstream_header.map(HeaderValue::from_str).transpose()?;
        let base_url = base_url.map(Uri::from_str).transpose()?;

        Ok(super::get_upstream_uri(
            &uri,
            upstream_header_value,
            base_url.as_ref(),
        )?)
    }

    #[test]
    fn get_upstream_uri_from_headers() -> eyre::Result<()> {
        let result = get_upstream_uri(
            "https://www.google.com/path/wow",
            Some("https://www.facebok.com"),
            Some("https://www.amazon.com"),
        )?;
        assert_eq!(result, Uri::from_static("https://www.facebok.com"));

        Ok(())
    }

    #[test]
    fn get_upstream_uri_from_base_url() -> eyre::Result<()> {
        let result = get_upstream_uri(
            "https://www.google.com/path/wow",
            None,
            Some("https://www.amazon.com"),
        )?;
        assert_eq!(result, Uri::from_static("https://www.amazon.com/path/wow"));

        Ok(())
    }
}
