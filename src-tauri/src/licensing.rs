use crate::command_errors::CommandError;
use crate::config::AppConfig;
use crate::global_config;
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use rsa::pkcs1v15::{Signature as RsaSignature, VerifyingKey as RsaVerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use rsa::RsaPublicKey;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sha2::Sha256;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_LICENSE_ISSUER: &str = "whisperdict";

pub const ENTITLEMENT_FREE: &str = "free";
pub const ENTITLEMENT_PRO: &str = "pro";

pub const LICENSE_STATUS_NONE: &str = "none";
pub const LICENSE_STATUS_VALID: &str = "valid";
pub const LICENSE_STATUS_INVALID: &str = "invalid";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseState {
    pub entitlement: String,
    pub license_status: String,
    pub free_transcriptions_left: u32,
    pub total_transcriptions_count: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseImportResponse {
    pub ok: bool,
    pub entitlement: String,
    pub license_status: String,
}

#[derive(Debug, Clone)]
pub struct LicenseValidationResult {
    pub entitlement: String,
    pub license_status: String,
    pub message: Option<String>,
}

impl LicenseValidationResult {
    pub fn is_pro(&self) -> bool {
        self.entitlement == ENTITLEMENT_PRO && self.license_status == LICENSE_STATUS_VALID
    }
}

#[derive(Debug, Deserialize)]
struct LicenseContainer {
    version: String,
    payload: Box<RawValue>,
    signature: LicenseSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LicensePayload {
    invoice_number: String,
    checkout_id: String,
    product_id: String,
    product_price_id: String,
    amount: u64,
    customer_id: String,
    email: String,
    name: String,
    mac_address: String,
    source: String,
    platform: String,
    expires_at: Option<String>,
    issued_at: u64,
    issuer: String,
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LicenseSignature {
    algorithm: String,
    kid: String,
    value: String,
}

#[derive(Debug, Clone)]
struct TrustedPublicKey {
    key: RsaPublicKey,
}

pub fn trusted_public_keys() -> Vec<String> {
    global_config::trusted_license_public_keys()
}

pub fn license_issuer() -> String {
    let issuer = global_config::LICENSE_ISSUER.trim();
    if issuer.is_empty() {
        DEFAULT_LICENSE_ISSUER.to_string()
    } else {
        issuer.to_string()
    }
}

pub fn sanitize_config(config: &mut AppConfig) {
    if config.entitlement != ENTITLEMENT_PRO {
        config.entitlement = ENTITLEMENT_FREE.to_string();
    }

    if !matches!(
        config.license_status.as_str(),
        LICENSE_STATUS_NONE | LICENSE_STATUS_VALID | LICENSE_STATUS_INVALID
    ) {
        config.license_status = LICENSE_STATUS_NONE.to_string();
    }

    if let Some(path) = config.license_file_path.as_ref() {
        if path.trim().is_empty() {
            config.license_file_path = None;
        }
    }
}

pub fn import_license_file(
    path: &str,
    config: &mut AppConfig,
    trusted_public_keys: &[String],
    issuer: &str,
) -> Result<()> {
    let normalized_path = path.trim();
    config.license_file_path = if normalized_path.is_empty() {
        None
    } else {
        Some(normalized_path.to_string())
    };
    config.license_last_validated_at = Some(unix_timestamp());

    if normalized_path.is_empty() {
        config.entitlement = ENTITLEMENT_FREE.to_string();
        config.license_status = LICENSE_STATUS_INVALID.to_string();
        return Err(CommandError::license_invalid().into());
    }

    match validate_license_path(normalized_path, trusted_public_keys, issuer) {
        Ok(()) => {
            config.entitlement = ENTITLEMENT_PRO.to_string();
            config.license_status = LICENSE_STATUS_VALID.to_string();
            Ok(())
        }
        Err(_) => {
            config.entitlement = ENTITLEMENT_FREE.to_string();
            config.license_status = LICENSE_STATUS_INVALID.to_string();
            Err(CommandError::license_invalid().into())
        }
    }
}

pub fn clear_license(config: &mut AppConfig) {
    config.entitlement = ENTITLEMENT_FREE.to_string();
    config.license_status = LICENSE_STATUS_NONE.to_string();
    config.license_file_path = None;
    config.license_last_validated_at = Some(unix_timestamp());
}

pub fn validate_current_license(
    config: &mut AppConfig,
    trusted_public_keys: &[String],
    issuer: &str,
) -> Result<LicenseValidationResult> {
    sanitize_config(config);

    let mut message = None;
    match config.license_file_path.as_deref() {
        None => {
            config.entitlement = ENTITLEMENT_FREE.to_string();
            config.license_status = LICENSE_STATUS_NONE.to_string();
        }
        Some(path) => {
            if validate_license_path(path, trusted_public_keys, issuer).is_ok() {
                config.entitlement = ENTITLEMENT_PRO.to_string();
                config.license_status = LICENSE_STATUS_VALID.to_string();
            } else {
                config.entitlement = ENTITLEMENT_FREE.to_string();
                config.license_status = LICENSE_STATUS_INVALID.to_string();
                message = Some("Imported license file is invalid.".to_string());
            }
        }
    }

    config.license_last_validated_at = Some(unix_timestamp());

    Ok(LicenseValidationResult {
        entitlement: config.entitlement.clone(),
        license_status: config.license_status.clone(),
        message,
    })
}

pub fn build_license_state(config: &AppConfig, message: Option<String>) -> LicenseState {
    LicenseState {
        entitlement: config.entitlement.clone(),
        license_status: config.license_status.clone(),
        free_transcriptions_left: config.free_transcriptions_left,
        total_transcriptions_count: config.total_transcriptions_count,
        message,
    }
}

pub fn build_import_response(config: &AppConfig) -> LicenseImportResponse {
    LicenseImportResponse {
        ok: true,
        entitlement: config.entitlement.clone(),
        license_status: config.license_status.clone(),
    }
}

fn validate_license_path(path: &str, trusted_public_keys: &[String], issuer: &str) -> Result<()> {
    let raw = fs::read_to_string(path).context("read license file")?;
    validate_license_contents(&raw, trusted_public_keys, issuer)
}

fn validate_license_contents(
    raw: &str,
    trusted_public_keys: &[String],
    issuer: &str,
) -> Result<()> {
    let container: LicenseContainer =
        serde_json::from_str(raw).context("invalid license format")?;
    if container.version.trim() != "1" {
        anyhow::bail!("unsupported license version");
    }
    if container.signature.algorithm.trim() != "RSA-SHA256" {
        anyhow::bail!("unsupported license algorithm");
    }
    if container.signature.kid.trim() != "1" {
        anyhow::bail!("unsupported license key id");
    }

    let parsed_keys = parse_trusted_public_keys(trusted_public_keys)?;
    if parsed_keys.is_empty() {
        anyhow::bail!("no trusted public keys configured");
    }

    let payload_to_sign = container.payload.get();
    let payload: LicensePayload =
        serde_json::from_str(payload_to_sign).context("invalid license payload")?;
    let compact_payload = serde_json::to_string(&payload).context("serialize license payload")?;
    let signature_bytes = decode_base64(&container.signature.value).context("decode signature")?;
    let verified = parsed_keys.iter().any(|entry| {
        verify_signature(&entry.key, payload_to_sign.as_bytes(), &signature_bytes).is_ok()
            || verify_signature(&entry.key, compact_payload.as_bytes(), &signature_bytes).is_ok()
    });
    if !verified {
        anyhow::bail!("license signature verification failed");
    }

    validate_payload(&payload, issuer)
}

fn validate_payload(payload: &LicensePayload, issuer: &str) -> Result<()> {
    if payload.issuer != issuer {
        anyhow::bail!("license issuer mismatch");
    }
    if payload.version.trim() != "1"
        || payload.invoice_number.trim().is_empty()
        || payload.checkout_id.trim().is_empty()
        || payload.product_id.trim().is_empty()
        || payload.product_price_id.trim().is_empty()
        || payload.amount == 0
        || payload.customer_id.trim().is_empty()
        || payload.email.trim().is_empty()
        || payload.name.trim().is_empty()
        || payload.mac_address.trim().is_empty()
        || payload.source.trim().is_empty()
        || payload.platform.trim().is_empty()
        || payload.issued_at == 0
    {
        anyhow::bail!("license payload is incomplete");
    }

    if let Some(expires_at) = payload.expires_at.as_deref() {
        if expires_at.trim().is_empty() {
            anyhow::bail!("invalid expiresAt");
        }
    }

    let current_mac = current_device_mac_address();
    let payload_mac = normalize_mac_address(&payload.mac_address)?;
    let device_mac = normalize_mac_address(&current_mac)?;
    if payload_mac != device_mac {
        anyhow::bail!("license macAddress mismatch");
    }

    Ok(())
}

fn current_device_mac_address() -> String {
    mac_address::get_mac_address()
        .ok()
        .flatten()
        .map(|address| address.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn normalize_mac_address(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("unknown") {
        return Ok("UNKNOWN".to_string());
    }

    let normalized: String = trimmed
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .map(|ch| ch.to_ascii_uppercase())
        .collect();

    if normalized.len() != 12 {
        anyhow::bail!("invalid macAddress format");
    }

    Ok(normalized)
}

fn parse_trusted_public_keys(entries: &[String]) -> Result<Vec<TrustedPublicKey>> {
    entries
        .iter()
        .map(|entry| parse_trusted_public_key(entry))
        .collect()
}

fn parse_trusted_public_key(entry: &str) -> Result<TrustedPublicKey> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("empty trusted key"));
    }

    let key = parse_verifying_key(trimmed)?;
    Ok(TrustedPublicKey { key })
}

fn parse_verifying_key(encoded: &str) -> Result<RsaPublicKey> {
    let trimmed = encoded.trim();

    if trimmed.contains("-----BEGIN") {
        if let Ok(rsa_key) = RsaPublicKey::from_public_key_pem(trimmed) {
            return Ok(rsa_key);
        }
    }

    let bytes = decode_base64(trimmed).context("decode verifying key")?;

    if let Ok(rsa_key) = RsaPublicKey::from_public_key_der(&bytes) {
        return Ok(rsa_key);
    }

    Err(anyhow!("trusted key must be RSA public key"))
}

fn verify_signature(
    key: &RsaPublicKey,
    signed_payload: &[u8],
    signature_bytes: &[u8],
) -> Result<()> {
    let signature = RsaSignature::try_from(signature_bytes).context("parse rsa signature")?;
    let verifier = RsaVerifyingKey::<Sha256>::new(key.clone());
    verifier
        .verify(signed_payload, &signature)
        .map_err(|_| anyhow!("license signature verification failed"))
}

fn decode_base64(input: &str) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(input.trim())
        .or_else(|_| URL_SAFE.decode(input.trim()))
        .or_else(|_| STANDARD_NO_PAD.decode(input.trim()))
        .or_else(|_| STANDARD.decode(input.trim()))
        .map_err(|_| anyhow!("invalid base64 value"))
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        import_license_file, validate_current_license, DEFAULT_LICENSE_ISSUER, ENTITLEMENT_FREE,
        ENTITLEMENT_PRO, LICENSE_STATUS_INVALID, LICENSE_STATUS_NONE, LICENSE_STATUS_VALID,
    };
    use crate::command_errors::{CommandError, LICENSE_INVALID_CODE};
    use crate::config::AppConfig;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use rsa::pkcs1v15::SigningKey as RsaSigningKey;
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    use rsa::rand_core::OsRng;
    use rsa::signature::{SignatureEncoding, Signer};
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use serde_json::json;
    use sha2::Sha256;
    use std::fs;

    fn make_license(issuer: &str) -> (String, String) {
        let mac_address = super::current_device_mac_address();
        make_license_with_mac(issuer, &mac_address)
    }

    fn make_license_with_mac(issuer: &str, mac_address: &str) -> (String, String) {
        let private_key = RsaPrivateKey::new(&mut OsRng, 2048).expect("generate rsa key");
        let public_key = RsaPublicKey::from(&private_key);
        let payload = json!({
            "invoiceNumber": "WHISPERDICT-SNYLHAUPNP-0001",
            "checkoutId": "478f6541-9c64-499c-ad9a-79b4e3bbf482",
            "productId": "d41c1607-1b71-4372-8280-fe6cc459aecb",
            "productPriceId": "335d4284-bc11-40f2-b6de-c3a3a2c4fbd5",
            "amount": 2900,
            "customerId": "366c0b17-6838-4cf2-a694-7c62382c2db6",
            "email": "test-whisperdict@icordoba.dev",
            "name": "Ivan",
            "macAddress": mac_address,
            "source": "whisperdict-desktop",
            "platform": "linux",
            "expiresAt": null,
            "issuedAt": 1770830962462u64,
            "issuer": issuer,
            "version": "1"
        });
        let payload_string = serde_json::to_string(&payload).expect("serialize payload");

        let signing_key = RsaSigningKey::<Sha256>::new(private_key);
        let signature = signing_key.sign(payload_string.as_bytes());

        let container = json!({
            "version": "1",
            "payload": payload,
            "signature": {
                "algorithm": "RSA-SHA256",
                "kid": "1",
                "value": STANDARD.encode(signature.to_bytes())
            }
        });
        let license_json = serde_json::to_string(&container).expect("serialize container");
        let public_key_pem = public_key
            .to_public_key_pem(LineEnding::LF)
            .expect("encode rsa public key");
        (license_json, public_key_pem)
    }

    #[test]
    fn valid_license_promotes_to_pro() {
        let (license_json, public_key) = make_license(DEFAULT_LICENSE_ISSUER);
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("valid.wdlic");
        fs::write(&path, license_json).expect("write license");
        let trusted_keys = vec![public_key];

        let mut config = AppConfig::default();
        import_license_file(
            path.to_str().expect("path str"),
            &mut config,
            &trusted_keys,
            DEFAULT_LICENSE_ISSUER,
        )
        .expect("license import should pass");

        assert_eq!(config.entitlement, ENTITLEMENT_PRO);
        assert_eq!(config.license_status, LICENSE_STATUS_VALID);

        let result =
            validate_current_license(&mut config, &trusted_keys, DEFAULT_LICENSE_ISSUER).unwrap();
        assert!(result.is_pro());
    }

    #[test]
    fn invalid_signature_is_rejected() {
        let (mut license_json, public_key) = make_license(DEFAULT_LICENSE_ISSUER);
        license_json = license_json.replacen("a", "b", 1);

        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("invalid.wdlic");
        fs::write(&path, license_json).expect("write license");

        let mut config = AppConfig::default();
        let err = import_license_file(
            path.to_str().expect("path str"),
            &mut config,
            &[public_key],
            DEFAULT_LICENSE_ISSUER,
        )
        .expect_err("import should fail");

        let command_error = err
            .downcast_ref::<CommandError>()
            .expect("must return command error");
        assert_eq!(command_error.code, LICENSE_INVALID_CODE);
        assert_eq!(config.entitlement, ENTITLEMENT_FREE);
        assert_eq!(config.license_status, LICENSE_STATUS_INVALID);
    }

    #[test]
    fn missing_license_path_stays_on_free() {
        let mut config = AppConfig::default();
        let result = validate_current_license(&mut config, &[], DEFAULT_LICENSE_ISSUER).unwrap();

        assert_eq!(result.entitlement, ENTITLEMENT_FREE);
        assert_eq!(result.license_status, LICENSE_STATUS_NONE);
    }

    #[test]
    fn old_license_format_is_rejected() {
        let (_, public_key_pem) = make_license(DEFAULT_LICENSE_ISSUER);
        let old_format = json!({
            "version": 1,
            "alg": "RS256",
            "payload": "abc",
            "signature": "def"
        });
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("old-format.wdlic");
        fs::write(&path, old_format.to_string()).expect("write license");

        let mut config = AppConfig::default();
        let err = import_license_file(
            path.to_str().expect("path str"),
            &mut config,
            &[public_key_pem],
            DEFAULT_LICENSE_ISSUER,
        )
        .expect_err("old format should fail");

        let command_error = err
            .downcast_ref::<CommandError>()
            .expect("must return command error");
        assert_eq!(command_error.code, LICENSE_INVALID_CODE);
        assert_eq!(config.entitlement, ENTITLEMENT_FREE);
        assert_eq!(config.license_status, LICENSE_STATUS_INVALID);
    }

    #[test]
    fn mac_address_mismatch_is_rejected() {
        let (license_json, public_key) =
            make_license_with_mac(DEFAULT_LICENSE_ISSUER, "00:00:00:00:00:00");

        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("invalid-mac.wdlic");
        fs::write(&path, license_json).expect("write license");

        let mut config = AppConfig::default();
        let err = import_license_file(
            path.to_str().expect("path str"),
            &mut config,
            &[public_key],
            DEFAULT_LICENSE_ISSUER,
        )
        .expect_err("mismatch mac should fail");

        let command_error = err
            .downcast_ref::<CommandError>()
            .expect("must return command error");
        assert_eq!(command_error.code, LICENSE_INVALID_CODE);
        assert_eq!(config.entitlement, ENTITLEMENT_FREE);
        assert_eq!(config.license_status, LICENSE_STATUS_INVALID);
    }
}
