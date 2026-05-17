//! Locale-aware wrappers around static name/blurb tables in the
//! `shared` crate. Every lookup routes through `i18n_loader`, keyed
//! by the numeric ids the shared crate uses.

use shared::{
    AreaDef, EnemyDef, GearTemplate, Inventory, AchievementCheck,
    ACHIEVEMENT_TABLE,
};

use super::i18n::Locale;
use super::i18n_loader;

pub fn form_name(locale: Locale, form: u8) -> &'static str {
    let key = format!("form.{form}");
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') {
        // Loader's `?missing-key` diagnostic → use the unknown bucket.
        i18n_loader::tr(locale.as_str(), "form.unknown")
    } else {
        v
    }
}

pub fn area_name(locale: Locale, area: &AreaDef) -> &'static str {
    // Wilds nodes carry a `wilds:<r>:<s>` marker in their static
    // `name` slot — assemble a locale-aware name from
    // `wilds_root.<r>` + `wilds_suffix.<s>`, falling back to the
    // English wordlists. The assembled name is leaked once per
    // (locale, area_id) so callers still get a `'static` slice.
    if let Some((r, s)) = shared::parse_wilds_name(area.name) {
        return cached_wilds_name(locale, area.id, r, s);
    }
    let key = format!("area_name.{}", area.id);
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') { area.name } else { v }
}

pub fn area_blurb(locale: Locale, area: &AreaDef) -> &'static str {
    if let Some(idx) = shared::parse_wilds_blurb(area.blurb) {
        return cached_wilds_blurb(locale, area.id, idx);
    }
    let key = format!("area_blurb.{}", area.id);
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') { area.blurb } else { v }
}

/// Assemble + intern a localized Wilds name. The `(locale,
/// area_id)` key is the cache index so a player switching locales
/// at runtime sees an up-to-date name; the table is bounded by
/// `locales × WILDS_NODE_COUNT` (≈ 6 × 8 = 48) so the leak budget
/// is trivial.
fn cached_wilds_name(locale: Locale, area_id: u8, r: usize, s: usize) -> &'static str {
    static CACHE: once_cell::sync::Lazy<
        std::sync::Mutex<std::collections::HashMap<(&'static str, u8), &'static str>>,
    > = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let key = (locale.as_str(), area_id);
    let mut guard = CACHE.lock().unwrap();
    if let Some(&v) = guard.get(&key) {
        return v;
    }
    let root_key = format!("wilds_root.{r}");
    let suffix_key = format!("wilds_suffix.{s}");
    let root = i18n_loader::tr(locale.as_str(), &root_key);
    let suffix = i18n_loader::tr(locale.as_str(), &suffix_key);
    let root = if root.starts_with('?') {
        shared::WILDS_ROOTS_EN.get(r).copied().unwrap_or("Wilds")
    } else { root };
    let suffix = if suffix.starts_with('?') {
        shared::WILDS_SUFFIXES_EN.get(s).copied().unwrap_or("Reach")
    } else { suffix };
    let assembled: &'static str = Box::leak(format!("{root} {suffix}").into_boxed_str());
    guard.insert(key, assembled);
    assembled
}

fn cached_wilds_blurb(locale: Locale, area_id: u8, idx: usize) -> &'static str {
    static CACHE: once_cell::sync::Lazy<
        std::sync::Mutex<std::collections::HashMap<(&'static str, u8), &'static str>>,
    > = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let key = (locale.as_str(), area_id);
    let mut guard = CACHE.lock().unwrap();
    if let Some(&v) = guard.get(&key) {
        return v;
    }
    let body_key = format!("wilds_atmos.{idx}");
    let body = i18n_loader::tr(locale.as_str(), &body_key);
    let body = if body.starts_with('?') {
        // English fallback — keep the original atmosphere list inline
        // so an out-of-date locale still reads coherently.
        match idx {
            0 => "off-path, unmarked on the village map",
            1 => "moss-thick and breath-quiet between the columns of stone",
            2 => "where the path forks back on itself if you blink wrong",
            3 => "the wind here sounds like other people's footsteps",
            4 => "old battle ground — the ghosts are uninterested but watching",
            _ => "places named here only when you've stayed past dusk",
        }
    } else { body };
    let assembled: &'static str = Box::leak(body.to_string().into_boxed_str());
    guard.insert(key, assembled);
    assembled
}

pub fn enemy_name(locale: Locale, enemy: &EnemyDef) -> &'static str {
    let key = format!("enemy_name.{}", enemy.id);
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') { enemy.name } else { v }
}

pub fn enemy_death_blurb(locale: Locale, enemy: &EnemyDef) -> &'static str {
    let key = format!("enemy_death.{}", enemy.id);
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') { enemy.death_blurb } else { v }
}

pub fn skill_name(locale: Locale, id: u8) -> &'static str {
    let key = format!("skill_name.{id}");
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') { "?" } else { v }
}

pub fn skill_blurb(locale: Locale, id: u8) -> &'static str {
    let key = format!("skill_blurb.{id}");
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') { "" } else { v }
}

pub fn ending_name(locale: Locale, id: u8) -> &'static str {
    let key = format!("ending_name.{id}");
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') { "?" } else { v }
}

pub fn ending_blurb(locale: Locale, id: u8) -> &'static str {
    let key = format!("ending_blurb.{id}");
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') { "" } else { v }
}

pub fn achievement_label(locale: Locale, id: u8) -> &'static str {
    let key = format!("achievement_label.{id}");
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') { "?" } else { v }
}

pub fn achievement_reason(locale: Locale, id: u8) -> String {
    for (aid, check) in ACHIEVEMENT_TABLE {
        if *aid == id {
            return match *check {
                AchievementCheck::Missions(n) => {
                    let n = n.to_string();
                    i18n_loader::fmt(locale.as_str(), "fmt.achievement_reason.missions", &[("n", n.as_str())])
                }
                AchievementCheck::BossDamage(n) => {
                    let n = n.to_string();
                    i18n_loader::fmt(locale.as_str(), "fmt.achievement_reason.boss_damage", &[("n", n.as_str())])
                }
                AchievementCheck::Gold(n) => {
                    let n = n.to_string();
                    i18n_loader::fmt(locale.as_str(), "fmt.achievement_reason.gold", &[("n", n.as_str())])
                }
                AchievementCheck::Essence(n) => {
                    let n = n.to_string();
                    i18n_loader::fmt(locale.as_str(), "fmt.achievement_reason.essence", &[("n", n.as_str())])
                }
                AchievementCheck::WinCount(n) => {
                    let n = n.to_string();
                    i18n_loader::fmt(locale.as_str(), "fmt.achievement_reason.win_count", &[("n", n.as_str())])
                }
                AchievementCheck::LegendaryEquipped => {
                    i18n_loader::tr(locale.as_str(), "achievement_reason.legendary_equipped").to_string()
                }
            };
        }
    }
    i18n_loader::tr(locale.as_str(), "achievement_label.unknown").to_string()
}

pub fn slot_name(locale: Locale, idx: usize) -> &'static str {
    let key = format!("slot_name.{idx}");
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') {
        shared::SLOT_NAMES.get(idx).copied().unwrap_or("?")
    } else {
        v
    }
}

pub fn tier_prefix(locale: Locale, tier: u8) -> &'static str {
    let key = format!("tier_prefix.{tier}");
    let v = i18n_loader::tr(locale.as_str(), &key);
    if v.starts_with('?') {
        let idx = tier.saturating_sub(1) as usize;
        shared::TIER_PREFIXES.get(idx).copied().unwrap_or("?")
    } else {
        v
    }
}

pub fn gear_name(locale: Locale, t: &GearTemplate) -> String {
    format!("{} {}", tier_prefix(locale, t.tier), slot_name(locale, t.slot as usize))
}

pub fn chapter(locale: Locale, inv: &Inventory) -> (u8, String, String) {
    let area_id = inv.current_area;
    let area = shared::current_area_def(inv);
    let name_l = area_name(locale.clone(), &area);
    let is_wilds = area_id >= shared::WILDS_AREA_BASE;
    let chap_no = if is_wilds {
        area_id.saturating_sub(shared::WILDS_AREA_BASE).saturating_add(1)
    } else {
        area_id.saturating_add(1)
    };
    let chap_no_str = chap_no.to_string();
    let title_key = if is_wilds { "fmt.chapter_title_wilds" } else { "fmt.chapter_title_linear" };
    let title = i18n_loader::fmt(
        locale.as_str(),
        title_key,
        &[("chap_no", chap_no_str.as_str()), ("area_name", name_l)],
    );
    // Area 0 has a special first-mission body; other areas use the
    // numbered key, falling back to the area blurb if missing.
    let body: String = if area_id == 0 && inv.mission_count == 0 {
        let v = i18n_loader::tr(locale.as_str(), "chapter_body.0_first");
        if v.starts_with('?') { area_blurb(locale.clone(), &area).to_string() } else { v.to_string() }
    } else {
        let key = format!("chapter_body.{area_id}");
        let v = i18n_loader::tr(locale.as_str(), &key);
        if v.starts_with('?') { area_blurb(locale.clone(), &area).to_string() } else { v.to_string() }
    };
    (chap_no, title, body)
}

/// Mad Libs expansion from a stable seed. Each plot list has 6 JSON
/// entries (`plot_homes.0`..`.5` etc.) indexed modularly.
pub fn plot_tuple_l10n(locale: Locale, seed: u32) -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
    let s = seed as u64;
    let home = lookup_plot(&locale, "plot_homes", s % 6);
    let mac = lookup_plot(&locale, "plot_macguffins", (s / 7) % 6);
    let vil = lookup_plot(&locale, "plot_villains", (s / 53) % 6);
    let mthd = lookup_plot(&locale, "plot_methods", (s / 211) % 6);
    let dest = lookup_plot(&locale, "plot_destinations", (s / 1009) % 6);
    (home, mac, vil, mthd, dest)
}

fn lookup_plot(locale: &Locale, base: &str, idx: u64) -> &'static str {
    let key = format!("{base}.{idx}");
    i18n_loader::tr(locale.as_str(), &key)
}
