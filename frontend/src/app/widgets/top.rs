//! Top action-bar tab definition.

use crate::app::types::Tab;

/// Top action-bar tabs. Each tuple is (icon, label, tab). Mirrors
/// the Shop / World Map / Work-on-Farm strip at the top of ICSBAH
/// and the section sidebar of YC. Clicking a tab swaps the entire
/// main view to its dedicated content; only one section is visible
/// at a time so the page stays focused.
pub fn top_actions() -> [(&'static str, &'static str, Tab); 7] {
    [
        ("🛡", "Farm", Tab::Farm),
        ("🗺", "World Map", Tab::WorldMap),
        ("🛒", "Shop", Tab::Shop),
        ("⚔", "Guilds", Tab::Guilds),
        ("🏆", "Achievements", Tab::Achievements),
        ("⚙", "Settings", Tab::Settings),
        ("❔", "Help", Tab::Help),
    ]
}
