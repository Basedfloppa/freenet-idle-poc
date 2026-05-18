//! JSON-driven theme loader. Theme files in
//! `frontend/themes/*.json` are bundled via `include_dir!`, parsed
//! once on first access, and exposed through `available_codes`,
//! `endonym`, `vars`. Adding a theme is one file drop.
//!
//! Mirror of `i18n_loader.rs` — same scan-once-then-cache pattern,
//! same `include_dir!` source-of-truth, same `available_codes`
//! API shape.

use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

static THEMES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/themes");

#[derive(Debug, Deserialize)]
pub struct ThemeMeta {
    pub endonym: String,
    pub code: String,
    #[serde(default)]
    pub scheme: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Theme {
    #[serde(rename = "_meta")]
    pub meta: ThemeMeta,
    pub vars: BTreeMap<String, String>,
}

static THEMES: Lazy<HashMap<&'static str, Theme>> = Lazy::new(load_all);
static CODES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut v: Vec<&'static str> = THEMES.keys().copied().collect();
    v.sort();
    v
});

fn load_all() -> HashMap<&'static str, Theme> {
    let mut out = HashMap::new();
    for file in THEMES_DIR.files() {
        let Some(name) = file.path().file_stem().and_then(|s| s.to_str()) else { continue };
        let Some(ext) = file.path().extension().and_then(|s| s.to_str()) else { continue };
        if ext != "json" {
            continue;
        }
        let Ok(text) = std::str::from_utf8(file.contents()) else { continue };
        let Ok(theme): Result<Theme, _> = serde_json::from_str(text) else { continue };
        let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
        out.insert(leaked, theme);
    }
    out
}

pub fn available_codes() -> &'static [&'static str] {
    &CODES
}

pub fn get(code: &str) -> Option<&'static Theme> {
    THEMES.get(code).map(|t| t as _)
}

pub fn endonym(code: &str) -> &str {
    get(code).map(|t| t.meta.endonym.as_str()).unwrap_or(code)
}

/// Render a theme's vars as a single `:root { ... }` CSS block.
/// Used by `apply_theme` to populate the `<style id="dynamic-theme">`
/// element at switch time.
pub fn render_root_css(code: &str) -> String {
    let Some(theme) = get(code) else {
        return ":root {}".into();
    };
    let mut out = String::from(":root {\n");
    for (k, v) in &theme.vars {
        out.push_str(&format!("  {k}: {v};\n"));
    }
    out.push_str("}\n");
    out
}

pub fn is_known(code: &str) -> bool {
    THEMES.contains_key(code)
}
