//! JSON-driven translation loader. Locale files in
//! `frontend/locales/*.json` are bundled via `include_dir!`, parsed
//! once on first access, and exposed through `tr`/`fmt`/`tr_list`.
//! Adding a language is one file drop; adding a key requires only an
//! entry in `en.json` (others fall back at runtime). Plural rules
//! stay in Rust — they're code, not data.

use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

static LOCALES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/locales");

#[derive(Debug, Clone, Copy)]
enum Leaf {
    Str(&'static str),
    List(&'static [&'static str]),
}

struct LocaleData {
    entries: HashMap<&'static str, Leaf>,
}

static DATA: Lazy<HashMap<&'static str, LocaleData>> = Lazy::new(load_all);
static CODES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut v: Vec<&'static str> = DATA.keys().copied().collect();
    v.sort();
    v
});

fn load_all() -> HashMap<&'static str, LocaleData> {
    let mut out = HashMap::new();
    for file in LOCALES_DIR.files() {
        let Some(name) = file.path().file_stem().and_then(|s| s.to_str()) else { continue };
        let Some(ext) = file.path().extension().and_then(|s| s.to_str()) else { continue };
        if ext != "json" {
            continue;
        }
        let Ok(text) = std::str::from_utf8(file.contents()) else { continue };
        let Ok(root): Result<Value, _> = serde_json::from_str(text) else { continue };
        let Value::Object(map) = root else { continue };
        let mut entries: HashMap<&'static str, Leaf> = HashMap::new();
        for (k, v) in map {
            // `_meta.*` keys are flattened from a nested object so
            // callers can ask `tr("_meta.endonym")` uniformly.
            if let Value::Object(meta) = &v {
                if k == "_meta" {
                    for (mk, mv) in meta {
                        if let Some(s) = mv.as_str() {
                            let leaked_key = Box::leak(format!("_meta.{mk}").into_boxed_str());
                            let leaked_val: &'static str = Box::leak(s.to_string().into_boxed_str());
                            entries.insert(leaked_key, Leaf::Str(leaked_val));
                        }
                    }
                    continue;
                }
            }
            let leaked_key: &'static str = Box::leak(k.into_boxed_str());
            match v {
                Value::String(s) => {
                    let leaked: &'static str = Box::leak(s.into_boxed_str());
                    entries.insert(leaked_key, Leaf::Str(leaked));
                }
                Value::Array(items) => {
                    let leaked_items: Vec<&'static str> = items
                        .into_iter()
                        .filter_map(|item| item.as_str().map(|s| {
                            let owned: &'static str = Box::leak(s.to_string().into_boxed_str());
                            owned
                        }))
                        .collect();
                    let slice: &'static [&'static str] = Box::leak(leaked_items.into_boxed_slice());
                    entries.insert(leaked_key, Leaf::List(slice));
                }
                _ => {}
            }
        }
        let leaked_code: &'static str = Box::leak(name.to_string().into_boxed_str());
        out.insert(leaked_code, LocaleData { entries });
    }
    out
}

pub fn tr(locale: &str, key: &str) -> &'static str {
    if let Some(data) = DATA.get(locale) {
        if let Some(Leaf::Str(s)) = data.entries.get(key) {
            return s;
        }
    }
    if locale != "en" {
        if let Some(data) = DATA.get("en") {
            if let Some(Leaf::Str(s)) = data.entries.get(key) {
                return s;
            }
        }
    }
    // Last-resort: visible `?key` diagnostic, leaked once per missing
    // key so the returned slice stays `'static`.
    static MISSING: Lazy<std::sync::Mutex<HashMap<String, &'static str>>> =
        Lazy::new(|| std::sync::Mutex::new(HashMap::new()));
    let mut cache = MISSING.lock().unwrap();
    if let Some(&v) = cache.get(key) {
        return v;
    }
    let leaked: &'static str = Box::leak(format!("?{key}").into_boxed_str());
    cache.insert(key.to_string(), leaked);
    leaked
}

pub fn tr_list(locale: &str, key: &str) -> &'static [&'static str] {
    if let Some(data) = DATA.get(locale) {
        if let Some(Leaf::List(items)) = data.entries.get(key) {
            return items;
        }
    }
    if locale != "en" {
        if let Some(data) = DATA.get("en") {
            if let Some(Leaf::List(items)) = data.entries.get(key) {
                return items;
            }
        }
    }
    &[]
}

/// Substitute `{placeholder}` in the looked-up template. Unknown
/// placeholders are left as `{name}` so JSON typos surface visibly.
pub fn fmt(locale: &str, key: &str, args: &[(&str, &str)]) -> String {
    let template = tr(locale, key);
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(close_rel) = template[i + 1..].find('}') {
                let close = i + 1 + close_rel;
                let name = &template[i + 1..close];
                let mut substituted = false;
                for (pname, pval) in args {
                    if *pname == name {
                        out.push_str(pval);
                        substituted = true;
                        break;
                    }
                }
                if !substituted {
                    out.push_str(&template[i..=close]);
                }
                i = close + 1;
                continue;
            }
        }
        // UTF-8-safe char boundary; we can't index by byte mid-codepoint.
        let ch = template[i..].chars().next().expect("non-empty");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

pub fn available_codes() -> &'static [&'static str] {
    &CODES
}

pub fn is_known(code: &str) -> bool {
    DATA.contains_key(code)
}

pub fn ru_plural<'a>(n: u64, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    let mod10 = n % 10;
    let mod100 = n % 100;
    if mod10 == 1 && mod100 != 11 {
        one
    } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
        few
    } else {
        many
    }
}
