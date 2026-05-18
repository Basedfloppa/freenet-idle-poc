//! Locale type + thin wrappers around the JSON-driven translation
//! loader (`super::i18n_loader`). `Locale` is `Copy` so existing
//! call sites that used the prior enum continue to work. `MessageId`
//! survives as a typed namespace mapping variants to JSON keys via
//! `MessageId::key()`. Plural rules stay in Rust per locale code.

use serde::{Deserialize, Serialize, Serializer, Deserializer};

use super::i18n_loader;

impl Serialize for Locale {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for Locale {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        Ok(Locale::from_str(&s))
    }
}

/// Wraps a `&'static str` locale code (e.g. `"en"`, `"ru"`). Known
/// codes reuse the static slice from the `include_dir!` scan; unknown
/// codes (from old prefs blobs) are leaked once at deserialize so the
/// type stays `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locale(pub &'static str);

impl Default for Locale {
    fn default() -> Self {
        Self("en")
    }
}

impl Locale {
    pub const fn from_static(code: &'static str) -> Self {
        Self(code)
    }

    /// If `code` is a bundled locale, reuse its `'static` slice;
    /// otherwise leak once so the returned `Locale` is `Copy`.
    pub fn from_str(code: &str) -> Self {
        for c in i18n_loader::available_codes() {
            if *c == code {
                return Self(*c);
            }
        }
        Self(Box::leak(code.to_string().into_boxed_str()))
    }

    pub fn new(code: &str) -> Self {
        Self::from_str(code)
    }

    pub fn as_str(&self) -> &'static str {
        self.0
    }

    pub fn code(&self) -> &'static str {
        self.0
    }

    /// Locale's own name from its `_meta.endonym` JSON entry.
    pub fn endonym(&self) -> &'static str {
        i18n_loader::tr(self.0, "_meta.endonym")
    }

    pub fn tr(&self, msg: MessageId) -> &'static str {
        i18n_loader::tr(self.0, msg.key())
    }

    /// Raw-key lookup for strings that don't warrant a `MessageId`
    /// variant (ad-hoc copy migrated from inline match arms).
    pub fn tr_key(&self, key: &str) -> &'static str {
        i18n_loader::tr(self.0, key)
    }
}

impl Locale {
    pub fn fmt_tutorial_run_mission(&self) -> String {
        self.tr(MessageId::TutorialBody1).to_string()
    }

    /// "Auto ran for {N} ({W} missions). {L} ended in defeat."
    /// The lost-tail is appended only when missions_lost > 0 so the
    /// translator can keep the period in the main sentence.
    pub fn fmt_catchup_summary(&self, elapsed_human: &str, missions_won: u32, missions_lost: u32) -> String {
        let won = missions_won.to_string();
        let lost = missions_lost.to_string();
        let mut out = i18n_loader::fmt(
            self.0,
            "fmt.catchup_summary",
            &[("elapsed", elapsed_human), ("won", won.as_str())],
        );
        if missions_lost > 0 {
            out.push_str(&i18n_loader::fmt(
                self.0,
                "fmt.catchup_summary_lost_tail",
                &[("lost", lost.as_str())],
            ));
        }
        out
    }

    pub fn fmt_catchup_rewards(&self, gold: &str, essence: &str, xp: &str, dmg: &str) -> String {
        i18n_loader::fmt(
            self.0,
            "fmt.catchup_rewards",
            &[("gold", gold), ("essence", essence), ("xp", xp), ("dmg", dmg)],
        )
    }

    pub fn fmt_onboarding_step(&self, current: u8, total: u8) -> String {
        let c = current.to_string();
        let t = total.to_string();
        i18n_loader::fmt(self.0, "fmt.onboarding_step", &[("current", c.as_str()), ("total", t.as_str())])
    }

    pub fn fmt_seconds_ago(&self, seconds: u64) -> String {
        let s = seconds.to_string();
        i18n_loader::fmt(self.0, "fmt.seconds_ago", &[("seconds", s.as_str())])
    }

    pub fn term_never(&self) -> &'static str {
        self.tr(MessageId::TermNever)
    }

    pub fn fmt_estate_hint(&self, form_name: &str) -> String {
        i18n_loader::fmt(self.0, "fmt.estate_hint", &[("form_name", form_name)])
    }

    pub fn fmt_legacy_header(&self, stars: u64, ascensions: u64, next_star_level: u64) -> String {
        let s = stars.to_string();
        let a = ascensions.to_string();
        let n = next_star_level.to_string();
        i18n_loader::fmt(
            self.0,
            "fmt.legacy_header",
            &[("stars", s.as_str()), ("ascensions", a.as_str()), ("next_star_level", n.as_str())],
        )
    }

    pub fn fmt_whats_new(&self, version: &str) -> String {
        i18n_loader::fmt(self.0, "fmt.whats_new", &[("version", version)])
    }

    pub fn fmt_now_running(&self, version: &str) -> String {
        i18n_loader::fmt(self.0, "fmt.now_running", &[("version", version)])
    }

    pub fn fmt_estate_worker_line(&self, tier_name: &str, count: u64) -> String {
        let c = count.to_string();
        i18n_loader::fmt(self.0, "fmt.estate_worker_line", &[("tier_name", tier_name), ("count", c.as_str())])
    }

    /// Plural-aware: routes to `.one` / `.few` / `.many` / `.other`
    /// JSON keys via `plural_key()` per locale rule.
    pub fn fmt_inbox_count(&self, n: usize) -> String {
        let key = plural_key("fmt.inbox_count", self.0, n as u64);
        let n_str = n.to_string();
        i18n_loader::fmt(self.0, &key, &[("n", n_str.as_str())])
    }

    pub fn fmt_stash_count(&self, n: usize) -> String {
        let key = plural_key("fmt.stash_count", self.0, n as u64);
        let n_str = n.to_string();
        i18n_loader::fmt(self.0, &key, &[("n", n_str.as_str())])
    }

    pub fn fmt_count_of(&self, label: &str, n: usize, total: usize) -> String {
        let n_str = n.to_string();
        let t_str = total.to_string();
        i18n_loader::fmt(self.0, "fmt.count_of", &[("label", label), ("n", n_str.as_str()), ("total", t_str.as_str())])
    }

    pub fn fmt_boss_summary(&self, era: u64, hp: &str, max_hp: &str, total_dmg: &str, players: usize) -> String {
        let e = era.to_string();
        let p = players.to_string();
        i18n_loader::fmt(
            self.0,
            "fmt.boss_summary",
            &[("era", e.as_str()), ("hp", hp), ("max_hp", max_hp), ("total_dmg", total_dmg), ("players", p.as_str())],
        )
    }

    pub fn fmt_currently_farming(&self, area: &str, lvl: u64) -> String {
        let l = lvl.to_string();
        i18n_loader::fmt(self.0, "fmt.currently_farming", &[("area", area), ("lvl", l.as_str())])
    }

    pub fn fmt_shop_balance(&self, gold: &str, potions: &str, fireballs: &str) -> String {
        i18n_loader::fmt(
            self.0,
            "fmt.shop_balance",
            &[("gold", gold), ("potions", potions), ("fireballs", fireballs)],
        )
    }

    pub fn fmt_buy_gold(&self, price: u64) -> String {
        let p = price.to_string();
        i18n_loader::fmt(self.0, "fmt.buy_gold", &[("price", p.as_str())])
    }

    pub fn fmt_buy_essence(&self, price: u64) -> String {
        let p = price.to_string();
        i18n_loader::fmt(self.0, "fmt.buy_essence", &[("price", p.as_str())])
    }

    pub fn fmt_active_players(&self, n: usize) -> String {
        let n = n.to_string();
        i18n_loader::fmt(self.0, "fmt.active_players", &[("n", n.as_str())])
    }

    pub fn fmt_you_are_in_guild(&self, name: &str) -> String {
        i18n_loader::fmt(self.0, "fmt.you_are_in_guild", &[("name", name)])
    }

    pub fn fmt_guild_meta(&self, members: usize, max_members: usize, leader_label: &str) -> String {
        let m = members.to_string();
        let mm = max_members.to_string();
        i18n_loader::fmt(
            self.0,
            "fmt.guild_meta",
            &[("members", m.as_str()), ("max_members", mm.as_str()), ("leader_label", leader_label)],
        )
    }

    pub fn fmt_directory(&self, n: usize) -> String {
        let n = n.to_string();
        i18n_loader::fmt(self.0, "fmt.directory", &[("n", n.as_str())])
    }

    pub fn fmt_stash_header(&self, n: usize) -> String {
        let n = n.to_string();
        i18n_loader::fmt(self.0, "fmt.stash_header", &[("n", n.as_str())])
    }

    pub fn fmt_sync_cadence(&self, cadence: crate::app::prefs::SyncCadence) -> &'static str {
        use crate::app::prefs::SyncCadence as C;
        let key = match cadence {
            C::Aggressive => "fmt.sync_cadence_aggressive",
            C::Normal => "fmt.sync_cadence_normal",
            C::Easy => "fmt.sync_cadence_easy",
        };
        i18n_loader::tr(self.0, key)
    }

    pub fn fmt_hp_pause_label(&self, pct: u8) -> String {
        if pct == 0 {
            i18n_loader::tr(self.0, "fmt.hp_pause_label_zero").to_string()
        } else {
            let p = pct.to_string();
            i18n_loader::fmt(self.0, "fmt.hp_pause_label_pct", &[("pct", p.as_str())])
        }
    }

    pub fn fmt_lvl_required(&self, min_level: u64) -> String {
        let l = min_level.to_string();
        i18n_loader::fmt(self.0, "fmt.lvl_required", &[("min_level", l.as_str())])
    }

    pub fn fmt_clears_required(&self, have: u64, need: u64) -> String {
        let h = have.to_string();
        let n = need.to_string();
        i18n_loader::fmt(self.0, "fmt.clears_required", &[("have", h.as_str()), ("need", n.as_str())])
    }

    pub fn fmt_cleared_count(&self, n: u64) -> String {
        let n = n.to_string();
        i18n_loader::fmt(self.0, "fmt.cleared_count", &[("n", n.as_str())])
    }

    pub fn fmt_encounter_progress(&self, idx: u32, total: u32) -> String {
        let i = idx.to_string();
        let t = total.to_string();
        i18n_loader::fmt(self.0, "fmt.encounter_progress", &[("idx", i.as_str()), ("total", t.as_str())])
    }

    pub fn fmt_no_spare_loot(&self, every_n: u32) -> String {
        let n = every_n.to_string();
        i18n_loader::fmt(self.0, "fmt.no_spare_loot", &[("every_n", n.as_str())])
    }

    pub fn fmt_chapter(&self, n: u64) -> String {
        let n = n.to_string();
        i18n_loader::fmt(self.0, "fmt.chapter", &[("n", n.as_str())])
    }

    pub fn fmt_plot_backstory(&self, home: &str, mac: &str, vil: &str, mthd: &str, dest: &str) -> String {
        i18n_loader::fmt(
            self.0,
            "fmt.plot_backstory",
            &[("home", home), ("mac", mac), ("vil", vil), ("mthd", mthd), ("dest", dest)],
        )
    }

    pub fn fmt_mission_summary(&self, area: &str, encounters: u32, essence: u64, mission_damage: u64) -> String {
        let e = encounters.to_string();
        let es = essence.to_string();
        let md = mission_damage.to_string();
        let key = if mission_damage == 0 { "fmt.mission_summary_no_boss" } else { "fmt.mission_summary_with_boss" };
        i18n_loader::fmt(
            self.0,
            key,
            &[
                ("area", area),
                ("encounters", e.as_str()),
                ("essence", es.as_str()),
                ("mission_damage", md.as_str()),
            ],
        )
    }

    pub fn fmt_last_publish(&self, age: &str, gold: &str, damage: &str) -> String {
        i18n_loader::fmt(self.0, "fmt.last_publish", &[("age", age), ("gold", gold), ("damage", damage)])
    }

    pub fn fmt_equipped_bonus(&self, atk: u64, def: u64, hp: u64) -> String {
        let a = atk.to_string();
        let d = def.to_string();
        let h = hp.to_string();
        i18n_loader::fmt(self.0, "fmt.equipped_bonus", &[("atk", a.as_str()), ("def", d.as_str()), ("hp", h.as_str())])
    }

    pub fn fmt_fireball_idle(&self, dmg: u64) -> String {
        let d = dmg.to_string();
        i18n_loader::fmt(self.0, "fmt.fireball_idle", &[("dmg", d.as_str())])
    }

    pub fn fmt_sell_wheat_tooltip(&self, ratio: u64) -> String {
        let r = ratio.to_string();
        i18n_loader::fmt(self.0, "fmt.sell_wheat_tooltip", &[("ratio", r.as_str())])
    }

    pub fn fmt_wheat_balance(&self, wheat: &str, gold: &str) -> String {
        i18n_loader::fmt(self.0, "fmt.wheat_balance", &[("wheat", wheat), ("gold", gold)])
    }

    pub fn confirm_reset_progress(&self) -> &'static str {
        i18n_loader::tr(self.0, "confirm.reset_progress")
    }

    pub fn confirm_reveal_seed(&self) -> &'static str {
        i18n_loader::tr(self.0, "confirm.reveal_seed")
    }

    pub fn confirm_disband_guild(&self, guild_name: &str) -> String {
        i18n_loader::fmt(self.0, "confirm.disband_guild", &[("guild_name", guild_name)])
    }

    pub fn status_seed_exported(&self) -> &'static str {
        i18n_loader::tr(self.0, "status.seed_exported")
    }

    pub fn fmt_status_seed_export_failed(&self, err: &str) -> String {
        i18n_loader::fmt(self.0, "fmt.status_seed_export_failed", &[("err", err)])
    }

    pub fn help_body(&self) -> HelpBody {
        HelpBody::load(self.0)
    }
}

/// JSON sub-key for `n` items in `locale`. RU has 3 forms,
/// EN/FR/ES/DE binary, JA has no plural (single form).
fn plural_key(base: &str, locale: &str, n: u64) -> String {
    match locale {
        "ru" => {
            let mod10 = n % 10;
            let mod100 = n % 100;
            let suffix = if mod10 == 1 && mod100 != 11 {
                "one"
            } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
                "few"
            } else {
                "many"
            };
            format!("{base}.{suffix}")
        }
        "ja" => format!("{base}.other"),
        _ => {
            if n == 1 { format!("{base}.one") } else { format!("{base}.other") }
        }
    }
}

/// Help-tab content. Struct API preserved for `tabs/help.rs`;
/// fields are populated from JSON in `HelpBody::load`.
pub struct HelpBody {
    pub loop_p1: &'static str,
    pub loop_p2: &'static str,
    pub stats_p1: &'static str,
    pub stats_p2: &'static str,
    pub forms_p1: &'static str,
    pub forms_p2: &'static str,
    pub tabs: &'static [&'static str],
    pub shop_p1: &'static str,
    pub shop_p2: &'static str,
    pub consumables_p1: &'static str,
    pub world_boss_p1: &'static str,
    pub delegate_p1: &'static str,
    pub delegate_p2: &'static str,
    pub guilds_p1: &'static str,
    pub guilds_p2: &'static str,
    pub estate_p1: &'static str,
    pub estate_p2: &'static str,
    pub legacy_p1: &'static str,
    pub area_graph_p1: &'static str,
}

impl HelpBody {
    fn load(locale: &str) -> Self {
        Self {
            loop_p1: i18n_loader::tr(locale, "help.loop_p1"),
            loop_p2: i18n_loader::tr(locale, "help.loop_p2"),
            stats_p1: i18n_loader::tr(locale, "help.stats_p1"),
            stats_p2: i18n_loader::tr(locale, "help.stats_p2"),
            forms_p1: i18n_loader::tr(locale, "help.forms_p1"),
            forms_p2: i18n_loader::tr(locale, "help.forms_p2"),
            tabs: i18n_loader::tr_list(locale, "help.tabs_list"),
            shop_p1: i18n_loader::tr(locale, "help.shop_p1"),
            shop_p2: i18n_loader::tr(locale, "help.shop_p2"),
            consumables_p1: i18n_loader::tr(locale, "help.consumables_p1"),
            world_boss_p1: i18n_loader::tr(locale, "help.world_boss_p1"),
            delegate_p1: i18n_loader::tr(locale, "help.delegate_p1"),
            delegate_p2: i18n_loader::tr(locale, "help.delegate_p2"),
            guilds_p1: i18n_loader::tr(locale, "help.guilds_p1"),
            guilds_p2: i18n_loader::tr(locale, "help.guilds_p2"),
            estate_p1: i18n_loader::tr(locale, "help.estate_p1"),
            estate_p2: i18n_loader::tr(locale, "help.estate_p2"),
            legacy_p1: i18n_loader::tr(locale, "help.legacy_p1"),
            area_graph_p1: i18n_loader::tr(locale, "help.area_graph_p1"),
        }
    }
}

/// Typed namespace of JSON keys. Variants give call sites compile-time
/// safety against typos; the translation text itself lives in JSON.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
pub enum MessageId {
    BootLoading,

    StatusAskingDelegate, StatusRegisteringDelegate, StatusSubscribing,

    TabHome, TabWorldMap, TabShop, TabGuilds, TabAchievements, TabMastery, TabSettings, TabHelp,

    PillDefeated, PillAdventuring, PillFocusing, PillRecovering, PillReady, PillEstate,

    SettingsTitle, SettingsTheme, SettingsLanguage, SettingsSyncCadence,
    SettingsAutoMission, SettingsPublishBehavior, SettingsIdentityBackup,
    SettingsAdvanced, SettingsResetUiPrefs, SettingsMailbox, SettingsWhereStateLives,

    LocaleEnglish, LocaleRussian,

    BtnExportSeed, BtnResetProgress, BtnHide, BtnResetDefaults, BtnSendTestSelf,

    SourceLink,

    PanelHero, PanelEquipment, PanelConsumables, PanelResources, PanelShop,
    PanelBuyGear, PanelSage, PanelWorldMap, PanelWorldBoss,
    PanelPlotSoFar, PanelGuilds, PanelCreateGuild, PanelTutorialWelcome,
    PanelWhileAway, PanelEndings, PanelSkillsLine, PanelFormsVisited,
    PanelAchievementsLow, PanelHowToPlay,

    StatName, StatForm, StatLevel, StatXp, StatHp, StatAttack, StatDefence, StatSpeed, StatEvasion,
    ResGold, ResEssence, ResMissions, ResBossDamage, ResPotions, ResFireballs,
    ColSlot, ColName, ColDamage, ColArea, ColSeen,

    BtnRunMission, BtnAutoOn, BtnAutoOff, BtnAutoEquipBest, BtnUse, BtnBuy,
    BtnSellAllWheat, BtnCreate, BtnLeaveGuild, BtnDisbandGuild,
    BtnJoin, BtnEquip, BtnNext, BtnStartPlaying, BtnSkipIntro,

    ItemPotion, ItemFireball,

    TermYouBattle, TermYouBadge, TermYouLeader, TermLive, TermActive, TermOwned,
    TermMaxTier, TermEmpty, TermFormNa, TermFormLocks, TermNever, TermWin, TermDefeat,
    TermPubkeyHidden, TermPubkeyPending, TermPubkeyPendingShort,

    OnbTitleWelcome, OnbBodyWelcome1, OnbBodyWelcome2,
    OnbTitleLoop, OnbBodyLoop1, OnbBodyLoop2,
    OnbTitleAuto, OnbBodyAuto1, OnbBodyAuto2,
    OnbTitleTabs, OnbBodyTabs1, OnbBodyTabs2,

    TutorialBody1, TutorialBody2,

    BattleOpeningTurn, BattleNoEncounters, BattlePotionQueued, BattleFireballQueued, BattleMissed,

    MailboxEmpty, MailboxKindChat, MailboxKindGift, MailboxKindGuildInvite, MailboxKindTradeOffer,

    CatchupClearsHint,

    HelpTheLoop, HelpStats, HelpFormsTransformation, HelpTabs, HelpShopGear,
    HelpConsumables, HelpWorldBoss, HelpDelegateWhat, HelpGuildsMailbox,
    HelpEstate, HelpLegacy, HelpAreaGraph,

    PanelEstate, EstateBtnPause, EstateBtnRun, EstateColTier, EstateColOwned,
    EstateColYield, EstateColNextPrice, BtnHire,
    EstateResWheat, EstateResGold, EstateResEssence,

    PanelLegacy, LegacyColNode, LegacyColLevel, LegacyColMultiplier, LegacyColNextCost,
    BtnAscend, LegacyAscendBlurb, LegacyAscendConfirm,

    CatchupModalTitle, BtnGotIt, NewerBuildDesc,

    PanelFormsShop, FormsShopDesc, FormsShopBaselineDesc, TipFormAlreadyActive,

    PanelActivities, ActivitiesDesc, ActivityStart, ActivityStop,
    PanelRoutine, RoutineDesc, RoutineColTier, RoutineColCurrent, RoutineColTarget,
    PanelInsight, InsightDesc, InsightColNode, InsightColLevel, InsightColNextCost,
    PanelBossAttack, BossAttackBtn, BossAttackDesc, BossAttackLocked,
    PanelTokens, TokensDesc, TokenColPerk, TokenColPrice, BtnUnlock,
    ResInsight, ResTokens, MasteryIntro,
    PanelWilds, WildsDesc, MapViewLinear, MapViewWilds,

    SettingsThemeDesc, SettingsCadenceDesc, SettingsAutoMissionDesc,
    SettingsPublishCheckbox, SettingsIdentityBody, SettingsIdentityBodyStrong,
    SettingsIdentityBodyTail, SettingsAdvancedDesc, SettingsHidePubkey,
    SettingsHideStale, SettingsWsOverride, SettingsResetUiPrefsDesc,
    SettingsWhereStateBody, SettingsSeedRevealWarn,

    GuildsPanelDesc, GuildsContractMissing, GuildsContractMissingTail,
    GuildsEmptyList, GuildsViaScript, GuildNamePlaceholder,
    MailboxNotConfiguredHead, MailboxNotConfiguredVia, MailboxNotConfiguredTail,
    MailboxNotConfiguredIn,

    ShopStashDesc, ShopBuyGearDesc, ShopSageDesc, ShopFarmDescPassive,

    TipFightInProgress, TipAutoToggleMidFight, TipAutoEquipBest, TipAutoEquipNothing,
    TipEstateBlocksCombat, TipPotionQueue, TipPotionIdle, TipFireballQueue,
    TipUnequipSlot, TipDisbandLeader, PotionShopDesc, TermCorrupt,
}

impl MessageId {
    /// JSON-side key. New variant ⇒ add a matching entry in
    /// `frontend/locales/en.json` (others fall back to EN at runtime).
    pub fn key(self) -> &'static str {
        use MessageId::*;
        match self {
            BootLoading => "boot_loading",
            StatusAskingDelegate => "status.asking_delegate",
            StatusRegisteringDelegate => "status.registering_delegate",
            StatusSubscribing => "status.subscribing",
            TabHome => "tab.home",
            TabWorldMap => "tab.world_map",
            TabShop => "tab.shop",
            TabGuilds => "tab.guilds",
            TabAchievements => "tab.achievements",
            TabMastery => "tab.mastery",
            TabSettings => "tab.settings",
            TabHelp => "tab.help",
            PillDefeated => "pill.defeated",
            PillAdventuring => "pill.adventuring",
            PillFocusing => "pill.focusing",
            PillRecovering => "pill.recovering",
            PillReady => "pill.ready",
            PillEstate => "pill.estate",
            SettingsTitle => "settings.title",
            SettingsTheme => "settings.theme",
            SettingsLanguage => "settings.language",
            SettingsSyncCadence => "settings.sync_cadence",
            SettingsAutoMission => "settings.auto_mission",
            SettingsPublishBehavior => "settings.publish_behavior",
            SettingsIdentityBackup => "settings.identity_backup",
            SettingsAdvanced => "settings.advanced",
            SettingsResetUiPrefs => "settings.reset_ui_prefs",
            SettingsMailbox => "settings.mailbox",
            SettingsWhereStateLives => "settings.where_state_lives",
            LocaleEnglish => "_meta.endonym",
            LocaleRussian => "_meta.endonym",
            BtnExportSeed => "btn.export_seed",
            BtnResetProgress => "btn.reset_progress",
            BtnHide => "btn.hide",
            BtnResetDefaults => "btn.reset_defaults",
            BtnSendTestSelf => "btn.send_test_self",
            SourceLink => "source_link",
            PanelHero => "panel.hero",
            PanelEquipment => "panel.equipment",
            PanelConsumables => "panel.consumables",
            PanelResources => "panel.resources",
            PanelShop => "panel.shop",
            PanelBuyGear => "panel.buy_gear",
            PanelSage => "panel.sage",
            PanelWorldMap => "panel.world_map",
            PanelWorldBoss => "panel.world_boss",
            PanelPlotSoFar => "panel.plot_so_far",
            PanelGuilds => "panel.guilds",
            PanelCreateGuild => "panel.create_guild",
            PanelTutorialWelcome => "panel.tutorial_welcome",
            PanelWhileAway => "panel.while_away",
            PanelEndings => "panel.endings",
            PanelSkillsLine => "panel.skills_line",
            PanelFormsVisited => "panel.forms_visited",
            PanelAchievementsLow => "panel.achievements_low",
            PanelHowToPlay => "panel.how_to_play",
            StatName => "stat.name",
            StatForm => "stat.form",
            StatLevel => "stat.level",
            StatXp => "stat.xp",
            StatHp => "stat.hp",
            StatAttack => "stat.attack",
            StatDefence => "stat.defence",
            StatSpeed => "stat.speed",
            StatEvasion => "stat.evasion",
            ResGold => "res.gold",
            ResEssence => "res.essence",
            ResMissions => "res.missions",
            ResBossDamage => "res.boss_damage",
            ResPotions => "res.potions",
            ResFireballs => "res.fireballs",
            ColSlot => "col.slot",
            ColName => "col.name",
            ColDamage => "col.damage",
            ColArea => "col.area",
            ColSeen => "col.seen",
            BtnRunMission => "btn.run_mission",
            BtnAutoOn => "btn.auto_on",
            BtnAutoOff => "btn.auto_off",
            BtnAutoEquipBest => "btn.auto_equip_best",
            BtnUse => "btn.use",
            BtnBuy => "btn.buy",
            BtnSellAllWheat => "btn.sell_all_wheat",
            BtnCreate => "btn.create",
            BtnLeaveGuild => "btn.leave_guild",
            BtnDisbandGuild => "btn.disband_guild",
            BtnJoin => "btn.join",
            BtnEquip => "btn.equip",
            BtnNext => "btn.next",
            BtnStartPlaying => "btn.start_playing",
            BtnSkipIntro => "btn.skip_intro",
            ItemPotion => "item.potion",
            ItemFireball => "item.fireball",
            TermYouBattle => "term.you_battle",
            TermYouBadge => "term.you_badge",
            TermYouLeader => "term.you_leader",
            TermLive => "term.live",
            TermActive => "term.active",
            TermOwned => "term.owned",
            TermMaxTier => "term.max_tier",
            TermEmpty => "term.empty",
            TermFormNa => "term.form_na",
            TermFormLocks => "term.form_locks",
            TermNever => "term.never",
            TermWin => "term.win",
            TermDefeat => "term.defeat",
            TermPubkeyHidden => "term.pubkey_hidden",
            TermPubkeyPending => "term.pubkey_pending",
            TermPubkeyPendingShort => "term.pubkey_pending_short",
            OnbTitleWelcome => "onb.title_welcome",
            OnbBodyWelcome1 => "onb.body_welcome_1",
            OnbBodyWelcome2 => "onb.body_welcome_2",
            OnbTitleLoop => "onb.title_loop",
            OnbBodyLoop1 => "onb.body_loop_1",
            OnbBodyLoop2 => "onb.body_loop_2",
            OnbTitleAuto => "onb.title_auto",
            OnbBodyAuto1 => "onb.body_auto_1",
            OnbBodyAuto2 => "onb.body_auto_2",
            OnbTitleTabs => "onb.title_tabs",
            OnbBodyTabs1 => "onb.body_tabs_1",
            OnbBodyTabs2 => "onb.body_tabs_2",
            TutorialBody1 => "tutorial.body_1",
            TutorialBody2 => "tutorial.body_2",
            BattleOpeningTurn => "battle.opening_turn",
            BattleNoEncounters => "battle.no_encounters",
            BattlePotionQueued => "battle.potion_queued",
            BattleFireballQueued => "battle.fireball_queued",
            BattleMissed => "battle.missed",
            MailboxEmpty => "mailbox.empty",
            MailboxKindChat => "mailbox.kind_chat",
            MailboxKindGift => "mailbox.kind_gift",
            MailboxKindGuildInvite => "mailbox.kind_guild_invite",
            MailboxKindTradeOffer => "mailbox.kind_trade_offer",
            CatchupClearsHint => "catchup.clears_hint",
            HelpTheLoop => "help.the_loop",
            HelpStats => "help.stats",
            HelpFormsTransformation => "help.forms_transformation",
            HelpTabs => "help.tabs",
            HelpShopGear => "help.shop_gear",
            HelpConsumables => "help.consumables",
            HelpWorldBoss => "help.world_boss",
            HelpDelegateWhat => "help.delegate_what",
            HelpGuildsMailbox => "help.guilds_mailbox",
            HelpEstate => "help.estate",
            HelpLegacy => "help.legacy",
            HelpAreaGraph => "help.area_graph",
            PanelEstate => "panel.estate",
            EstateBtnPause => "btn.estate_pause",
            EstateBtnRun => "btn.estate_run",
            EstateColTier => "col.estate_tier",
            EstateColOwned => "col.estate_owned",
            EstateColYield => "col.estate_yield",
            EstateColNextPrice => "col.estate_next_price",
            BtnHire => "btn.hire",
            EstateResWheat => "term.estate_wheat",
            EstateResGold => "term.estate_gold",
            EstateResEssence => "term.estate_essence",
            PanelLegacy => "panel.legacy",
            LegacyColNode => "col.legacy_node",
            LegacyColLevel => "col.legacy_level",
            LegacyColMultiplier => "col.legacy_multiplier",
            LegacyColNextCost => "col.legacy_next_cost",
            BtnAscend => "btn.ascend",
            LegacyAscendBlurb => "legacy.ascend_blurb",
            LegacyAscendConfirm => "legacy.ascend_confirm",
            CatchupModalTitle => "catchup.modal_title",
            BtnGotIt => "btn.got_it",
            NewerBuildDesc => "catchup.newer_build_desc",
            PanelFormsShop => "panel.forms_shop",
            FormsShopDesc => "forms_shop.desc",
            FormsShopBaselineDesc => "forms_shop.baseline_desc",
            TipFormAlreadyActive => "tip.form_already_active",
            PanelActivities => "panel.activities",
            ActivitiesDesc => "activities.desc",
            ActivityStart => "btn.activity_start",
            ActivityStop => "btn.activity_stop",
            PanelRoutine => "panel.routine",
            RoutineDesc => "routine.desc",
            RoutineColTier => "col.routine_tier",
            RoutineColCurrent => "col.routine_current",
            RoutineColTarget => "col.routine_target",
            PanelInsight => "panel.insight",
            InsightDesc => "insight.desc",
            InsightColNode => "col.insight_node",
            InsightColLevel => "col.insight_level",
            InsightColNextCost => "col.insight_next_cost",
            PanelBossAttack => "panel.boss_attack",
            BossAttackBtn => "btn.boss_attack",
            BossAttackDesc => "boss_attack.desc",
            BossAttackLocked => "boss_attack.locked",
            PanelTokens => "panel.tokens",
            TokensDesc => "tokens.desc",
            TokenColPerk => "col.token_perk",
            TokenColPrice => "col.token_price",
            BtnUnlock => "btn.unlock",
            ResInsight => "res.insight",
            ResTokens => "res.tokens",
            MasteryIntro => "mastery.intro",
            PanelWilds => "panel.wilds",
            WildsDesc => "wilds.desc",
            MapViewLinear => "map_view.linear",
            MapViewWilds => "map_view.wilds",
            SettingsThemeDesc => "settings.theme_desc",
            SettingsCadenceDesc => "settings.cadence_desc",
            SettingsAutoMissionDesc => "settings.auto_mission_desc",
            SettingsPublishCheckbox => "settings.publish_checkbox",
            SettingsIdentityBody => "settings.identity_body",
            SettingsIdentityBodyStrong => "settings.identity_body_strong",
            SettingsIdentityBodyTail => "settings.identity_body_tail",
            SettingsAdvancedDesc => "settings.advanced_desc",
            SettingsHidePubkey => "settings.hide_pubkey",
            SettingsHideStale => "settings.hide_stale",
            SettingsWsOverride => "settings.ws_override",
            SettingsResetUiPrefsDesc => "settings.reset_ui_prefs_desc",
            SettingsWhereStateBody => "settings.where_state_body",
            SettingsSeedRevealWarn => "settings.seed_reveal_warn",
            GuildsPanelDesc => "guilds.panel_desc",
            GuildsContractMissing => "guilds.contract_missing",
            GuildsContractMissingTail => "guilds.contract_missing_tail",
            GuildsEmptyList => "guilds.empty_list",
            GuildsViaScript => "guilds.via_script",
            GuildNamePlaceholder => "guilds.name_placeholder",
            MailboxNotConfiguredHead => "mailbox.not_configured_head",
            MailboxNotConfiguredVia => "mailbox.not_configured_via",
            MailboxNotConfiguredTail => "mailbox.not_configured_tail",
            MailboxNotConfiguredIn => "mailbox.not_configured_in",
            ShopStashDesc => "shop.stash_desc",
            ShopBuyGearDesc => "shop.buy_gear_desc",
            ShopSageDesc => "shop.sage_desc",
            ShopFarmDescPassive => "shop.farm_desc_passive",
            TipFightInProgress => "tip.fight_in_progress",
            TipAutoToggleMidFight => "tip.auto_toggle_mid_fight",
            TipAutoEquipBest => "tip.auto_equip_best",
            TipAutoEquipNothing => "tip.auto_equip_nothing",
            TipEstateBlocksCombat => "tip.estate_blocks_combat",
            TipPotionQueue => "tip.potion_queue",
            TipPotionIdle => "tip.potion_idle",
            TipFireballQueue => "tip.fireball_queue",
            TipUnequipSlot => "tip.unequip_slot",
            TipDisbandLeader => "tip.disband_leader",
            PotionShopDesc => "shop.potion_desc",
            TermCorrupt => "term.corrupt",
        }
    }
}

/// Locales discovered at compile time. Drop a JSON in
/// `frontend/locales/` and its code appears here automatically.
pub fn available_locales() -> Vec<Locale> {
    i18n_loader::available_codes()
        .iter()
        .map(|c| Locale::new(*c))
        .collect()
}

pub fn locale_code(l: &Locale) -> &str {
    l.as_str()
}

pub fn locale_from_code(code: &str) -> Locale {
    Locale::new(code)
}

/// First-load default: first available locale whose code is a prefix
/// of `navigator.language`, else `en`.
pub fn detect_browser_locale() -> Locale {
    let Some(win) = web_sys::window() else {
        return Locale::default();
    };
    let lang = win.navigator().language().unwrap_or_default().to_lowercase();
    for code in i18n_loader::available_codes() {
        if lang.starts_with(code) {
            return Locale::new(*code);
        }
    }
    Locale::default()
}
