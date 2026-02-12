pub const CHECKOUT_ENDPOINT: &str =
    "https://n8n.icordoba.dev/webhook/whisperdict/polar/create-checkout";

pub const CHECKOUT_BEARER_TOKEN: Option<&str> = None;

pub const LICENSE_ISSUER: &str = "whisperdict";

const BUNDLED_LICENSE_PUBLIC_KEY: &str =
    include_str!("../keys/whisperdict_license_public_kid1.pem");

pub fn checkout_endpoint() -> Option<String> {
    let endpoint = CHECKOUT_ENDPOINT.trim();
    if endpoint.is_empty() {
        None
    } else {
        Some(endpoint.to_string())
    }
}

pub fn checkout_bearer_token() -> Option<String> {
    CHECKOUT_BEARER_TOKEN
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn trusted_license_public_keys() -> Vec<String> {
    let key = BUNDLED_LICENSE_PUBLIC_KEY.trim();
    if key.is_empty() {
        Vec::new()
    } else {
        vec![key.to_string()]
    }
}
