use http::Uri;
use sha2::{Digest, Sha256};

pub fn hash_uri(uri: &Uri) -> String {
    let mut hasher = Sha256::new();
    if let Some(scheme) = uri.scheme_str() {
        hasher.update(scheme);
    };
    if let Some(authority) = uri.authority() {
        hasher.update(authority.as_str());
    }
    if let Some(path_and_query) = uri.path_and_query() {
        hasher.update(path_and_query.as_str());
    }
    hex::encode(hasher.finalize().as_slice())
}
