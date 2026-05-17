//! Contract / delegate ids — compile-time defaults plus a runtime
//! override read from `./dev-keys.json` (written by
//! `scripts/dev-publish.sh` on every publish).

use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

// Prod ids — all four artefacts published on orange 2026-05-13 via
// the locally-built fdev that carries the freenet/freenet-core#4075
// fix (`load_contract_for_publish` strips the 40-byte version+hash
// prefix before sending). Orange's freenet 0.2.56 accepts these
// puts cleanly; once the fdev release with #4075 lands, anyone
// republishing from a fresh checkout will get the same ids back
// (they're deterministic from the WASM hash).
pub const CONTRACT_ID_B58: &str = "E7oPxKH7HzWbbtokFetxeQyuwg3BGyUYckZFcnZDLd7a";
pub const CODE_HASH_B58: &str = "HGiSb56BTVPBV8uX6Bjkezn1bEA2uyWRFsqWtCocmLhH";

pub const DELEGATE_KEY_B58: &str = "EZtPQJ1cDZc9kSZTmMVjifoXoftqcBbFZrv49eygjLyc";
pub const DELEGATE_CODE_HASH_B58: &str = "6a4GuYBVUkr2b58XCS1iVhajxwFVP7ShnoSz1voiUhaV";

/// Mailbox contract — payload-agnostic player-to-player message
/// bus. Subscribed on connect like the presence contract.
pub const MAILBOX_CONTRACT_ID_B58: &str = "3rNxmZTJmYVn5vrtu6GkuKspTjSpL5636f2gmdtWVZit";
pub const MAILBOX_CODE_HASH_B58: &str = "4c2znr6EG2pbT6PZXXrcwXirPHpB9LWzrPsiUVytzFfC";

/// Guilds contract — cooperative group registry.
pub const GUILDS_CONTRACT_ID_B58: &str = "59F9ZiwZJ2eGEPvgBpChM2dMDRqnDZWxcCk38Mm2FfS7";
pub const GUILDS_CODE_HASH_B58: &str = "5dp43FWKcCpsbQpoDGJr7QYezhSnD428LFp1JYc9MaE7";

/// Runtime overrides served as `./dev-keys.json` next to index.html.
/// `scripts/dev-publish.sh` rewrites this file on every (re-)publish
/// — empty strings fall back to the compile-time constants above.
#[derive(Debug, Default, Deserialize)]
pub struct DevKeys {
    #[serde(default)]
    contract_id_b58: String,
    #[serde(default)]
    code_hash_b58: String,
    #[serde(default)]
    delegate_key_b58: String,
    #[serde(default)]
    delegate_code_hash_b58: String,
    #[serde(default)]
    mailbox_contract_id_b58: String,
    #[serde(default)]
    mailbox_code_hash_b58: String,
    #[serde(default)]
    guilds_contract_id_b58: String,
    #[serde(default)]
    guilds_code_hash_b58: String,
}

impl DevKeys {
    pub fn contract_or(&self, fallback: &str) -> String {
        if self.contract_id_b58.is_empty() { fallback.to_string() } else { self.contract_id_b58.clone() }
    }
    pub fn code_or(&self, fallback: &str) -> String {
        if self.code_hash_b58.is_empty() { fallback.to_string() } else { self.code_hash_b58.clone() }
    }
    pub fn delegate_or(&self, fallback: &str) -> String {
        if self.delegate_key_b58.is_empty() { fallback.to_string() } else { self.delegate_key_b58.clone() }
    }
    pub fn delegate_code_or(&self, fallback: &str) -> String {
        if self.delegate_code_hash_b58.is_empty() {
            fallback.to_string()
        } else {
            self.delegate_code_hash_b58.clone()
        }
    }
    pub fn mailbox_contract_or(&self, fallback: &str) -> String {
        if self.mailbox_contract_id_b58.is_empty() {
            fallback.to_string()
        } else {
            self.mailbox_contract_id_b58.clone()
        }
    }
    pub fn mailbox_code_or(&self, fallback: &str) -> String {
        if self.mailbox_code_hash_b58.is_empty() {
            fallback.to_string()
        } else {
            self.mailbox_code_hash_b58.clone()
        }
    }
    pub fn guilds_contract_or(&self, fallback: &str) -> String {
        if self.guilds_contract_id_b58.is_empty() {
            fallback.to_string()
        } else {
            self.guilds_contract_id_b58.clone()
        }
    }
    pub fn guilds_code_or(&self, fallback: &str) -> String {
        if self.guilds_code_hash_b58.is_empty() {
            fallback.to_string()
        } else {
            self.guilds_code_hash_b58.clone()
        }
    }
}

pub async fn load_dev_keys() -> DevKeys {
    async fn fetch() -> Result<DevKeys, String> {
        let win = web_sys::window().ok_or("no window")?;
        let resp_val = JsFuture::from(win.fetch_with_str("./dev-keys.json"))
            .await
            .map_err(|e| format!("fetch: {e:?}"))?;
        let response: Response =
            resp_val.dyn_into().map_err(|_| "not a Response".to_string())?;
        if !response.ok() {
            return Err(format!("HTTP {}", response.status()));
        }
        let text_promise = response.text().map_err(|e| format!("text(): {e:?}"))?;
        let text = JsFuture::from(text_promise)
            .await
            .map_err(|e| format!("text body: {e:?}"))?
            .as_string()
            .ok_or("text body not a string")?;
        serde_json::from_str(&text).map_err(|e| format!("parse: {e}"))
    }
    fetch().await.unwrap_or_else(|e| {
        web_sys::console::log_1(
            &format!("[dev-keys] using compile-time defaults: {e}").into(),
        );
        DevKeys::default()
    })
}
