//! Top action-bar tab definition.

use crate::app::i18n::{Locale, MessageId};
use crate::app::types::Tab;

/// Top action-bar tabs. Each tuple is (icon, label, tab). Mirrors
/// the Shop / World Map / Work-on-Farm strip at the top of ICSBAH
/// and the section sidebar of YC. Clicking a tab swaps the entire
/// main view to its dedicated content; only one section is visible
/// at a time so the page stays focused.
///
/// Takes `locale` so tab labels honour the active UI language. The
/// icon and `Tab` variant are locale-independent; only the human
/// label flips.
pub fn top_actions(locale: Locale) -> [(&'static str, &'static str, Tab); 8] {
    [
        ("🛡", locale.tr(MessageId::TabFarm), Tab::Farm),
        ("🗺", locale.tr(MessageId::TabWorldMap), Tab::WorldMap),
        ("🛒", locale.tr(MessageId::TabShop), Tab::Shop),
        ("⚔", locale.tr(MessageId::TabGuilds), Tab::Guilds),
        ("🏆", locale.tr(MessageId::TabAchievements), Tab::Achievements),
        ("⭐", locale.tr(MessageId::TabMastery), Tab::Mastery),
        ("⚙", locale.tr(MessageId::TabSettings), Tab::Settings),
        ("❔", locale.tr(MessageId::TabHelp), Tab::Help),
    ]
}
