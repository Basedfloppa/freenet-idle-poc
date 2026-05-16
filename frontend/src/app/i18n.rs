//! Minimal in-process i18n for the frontend.
//!
//! Design:
//! - `Locale` is a small `enum` (`En` / `Ru`); serializable so it
//!   round-trips through `UserPrefs` in `localStorage`.
//! - `MessageId` is a `#[non_exhaustive]` enum of every translated
//!   string. Adding a new visible string is a two-step change:
//!   1. Add a variant to `MessageId`.
//!   2. Add its English + Russian translation in `tr`.
//!   The compiler enforces step 2 — every new variant must be matched
//!   in both arms, so a forgotten translation is a build error.
//! - `tr(locale, msg)` returns a `&'static str` — zero allocation,
//!   pointer copy. For dynamic substitution use `format!("{}", tr(...))`
//!   at the call site, or one of the `fmt_*` methods on `Locale` for
//!   compound formatted strings whose template differs between locales.
//!
//! Scope: this round translates the full visible UI chrome — tab
//! labels, status pills, connection-status text, every panel header,
//! buttons, stat names, table columns, the onboarding wizard, the
//! mailbox panel, the catch-up banner, and the help tab. Names that
//! live in the `shared` crate (form names, enemy names, area names,
//! skill names, ending names, achievement labels) intentionally stay
//! English here — translating them needs a parallel pass in `shared`
//! and would split this change in two.
//!
//! Adding a new visible string still follows the two-step rule:
//! grow `MessageId`, grow `tr`. The compiler catches forgotten
//! arms, so the matrix never silently drifts.
//!
//! Russian wording note: the panel/section style uses lowercase
//! headers in English ("hero", "shop", "stash"), and the Russian
//! mirror keeps the same casing convention so the visual rhythm
//! survives translation.

use serde::{Deserialize, Serialize};

/// Languages the UI knows how to render. Default is `En`. `Ru` is the
/// only locale with full coverage; the rest (`De`, `Fr`, `Es`, `Ja`)
/// are partial — see each language's `tr_*` table — and route compound
/// `fmt_*` strings back to English via [`Locale::fmt_locale`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Locale {
    #[default]
    En,
    Ru,
    /// German (C5). Auto-selected when `navigator.language` starts
    /// with `de` and the player has no stored preference yet.
    /// Translation coverage is partial: see `Locale::tr_de` for
    /// the curated set — every other `MessageId` falls back to
    /// English via the catch-all arm in `tr`. Compound `fmt_*`
    /// helpers go through `Locale::fmt_locale` which normalises
    /// `De → En` so they don't need a third match arm each.
    De,
    /// French. Auto-selected on `navigator.language` prefix `fr`.
    /// Scaffolded with English fallback; curated overrides live in
    /// `Locale::tr_fr`. Compound `fmt_*` helpers fall through to
    /// English via [`Locale::fmt_locale`].
    Fr,
    /// Spanish. Auto-selected on `navigator.language` prefix `es`.
    /// Same fallback shape as `Fr`; curated overrides in
    /// `Locale::tr_es`.
    Es,
    /// Japanese. Auto-selected on `navigator.language` prefix `ja`.
    /// Same fallback shape as `Fr`; curated overrides in
    /// `Locale::tr_ja`.
    Ja,
}

impl Locale {
    /// Normalise compound-format dispatch. `Fr`, `Es`, `Ja`, and `Ru`
    /// have full compound-string coverage and pass through unchanged.
    /// `De` is the only locale still on the partial-coverage track —
    /// compound `fmt_*` strings collapse to English; its curated
    /// override table is `tr_de`. The wildcard `_` arms in each
    /// compound match below remain `unreachable!` because the only
    /// locale that ever lands in them is `De` after collapsing to
    /// `En`, which is matched explicitly.
    #[inline]
    pub fn fmt_locale(self) -> Self {
        match self {
            Self::En | Self::Ru | Self::Fr | Self::Es | Self::Ja => self,
            Self::De => Self::En,
        }
    }
}

impl Locale {
    /// Lookup of every translated string. Keep this exhaustive — any
    /// added variant of `MessageId` is enforced here by the compiler,
    /// so a forgotten translation is a build error rather than an
    /// untranslated string leaking into the UI.
    pub fn tr(self, msg: MessageId) -> &'static str {
        match (self, msg) {
            // ── Boot-time loader (shown until prefs_loaded flips) ──
            (Self::En, MessageId::BootLoading) => "Loading…",
            (Self::Ru, MessageId::BootLoading) => "Загрузка…",

            // ── Connection status (set as `c.status` from reconnect.rs) ──
            (Self::En, MessageId::StatusAskingDelegate) => "asking delegate for identity…",
            (Self::Ru, MessageId::StatusAskingDelegate) => "запрос личности у делегата…",
            (Self::En, MessageId::StatusRegisteringDelegate) => "registering delegate…",
            (Self::Ru, MessageId::StatusRegisteringDelegate) => "регистрация делегата…",
            (Self::En, MessageId::StatusSubscribing) => "subscribing…",
            (Self::Ru, MessageId::StatusSubscribing) => "подписка…",

            // ── Tab labels (top action bar) ──
            (Self::En, MessageId::TabFarm) => "Farm",
            (Self::Ru, MessageId::TabFarm) => "Ферма",
            (Self::En, MessageId::TabWorldMap) => "World Map",
            (Self::Ru, MessageId::TabWorldMap) => "Карта мира",
            (Self::En, MessageId::TabShop) => "Shop",
            (Self::Ru, MessageId::TabShop) => "Магазин",
            (Self::En, MessageId::TabGuilds) => "Guilds",
            (Self::Ru, MessageId::TabGuilds) => "Гильдии",
            (Self::En, MessageId::TabAchievements) => "Achievements",
            (Self::Ru, MessageId::TabAchievements) => "Достижения",
            (Self::En, MessageId::TabMastery) => "Mastery",
            (Self::Ru, MessageId::TabMastery) => "Мастерство",
            (Self::En, MessageId::TabSettings) => "Settings",
            (Self::Ru, MessageId::TabSettings) => "Настройки",
            (Self::En, MessageId::TabHelp) => "Help",
            (Self::Ru, MessageId::TabHelp) => "Помощь",

            // ── Status pills (uppercase by convention; CSS handles styling) ──
            (Self::En, MessageId::PillDefeated) => "DEFEATED",
            (Self::Ru, MessageId::PillDefeated) => "ПОБЕЖДЁН",
            (Self::En, MessageId::PillAdventuring) => "ADVENTURING",
            (Self::Ru, MessageId::PillAdventuring) => "В ПОИСКАХ",
            (Self::En, MessageId::PillFocusing) => "FOCUSING",
            (Self::Ru, MessageId::PillFocusing) => "СОСРЕДОТОЧЕН",
            (Self::En, MessageId::PillRecovering) => "RECOVERING",
            (Self::Ru, MessageId::PillRecovering) => "ВОССТАНОВЛЕНИЕ",
            (Self::En, MessageId::PillReady) => "READY",
            (Self::Ru, MessageId::PillReady) => "ГОТОВ",
            (Self::En, MessageId::PillEstate) => "ESTATE",
            (Self::Ru, MessageId::PillEstate) => "ПОМЕСТЬЕ",

            // ── Settings tab section headers ──
            (Self::En, MessageId::SettingsTitle) => "settings",
            (Self::Ru, MessageId::SettingsTitle) => "настройки",
            (Self::En, MessageId::SettingsTheme) => "theme",
            (Self::Ru, MessageId::SettingsTheme) => "тема",
            (Self::En, MessageId::SettingsLanguage) => "language",
            (Self::Ru, MessageId::SettingsLanguage) => "язык",
            (Self::En, MessageId::SettingsSyncCadence) => "sync cadence",
            (Self::Ru, MessageId::SettingsSyncCadence) => "частота синхронизации",
            (Self::En, MessageId::SettingsAutoMission) => "auto-mission",
            (Self::Ru, MessageId::SettingsAutoMission) => "авто-миссия",
            (Self::En, MessageId::SettingsPublishBehavior) => "publish behavior",
            (Self::Ru, MessageId::SettingsPublishBehavior) => "поведение публикации",
            (Self::En, MessageId::SettingsIdentityBackup) => "identity & backup",
            (Self::Ru, MessageId::SettingsIdentityBackup) => "личность и резервная копия",
            (Self::En, MessageId::SettingsAdvanced) => "advanced",
            (Self::Ru, MessageId::SettingsAdvanced) => "продвинутые",
            (Self::En, MessageId::SettingsResetUiPrefs) => "reset UI preferences",
            (Self::Ru, MessageId::SettingsResetUiPrefs) => "сброс настроек UI",
            (Self::En, MessageId::SettingsMailbox) => "mailbox (D2D test)",
            (Self::Ru, MessageId::SettingsMailbox) => "почта (тест D2D)",
            (Self::En, MessageId::SettingsWhereStateLives) => "where state lives",
            (Self::Ru, MessageId::SettingsWhereStateLives) => "где живёт состояние",

            // ── Locale picker labels (rendered IN the target locale) ──
            (Self::En, MessageId::LocaleEnglish) => "English",
            (Self::Ru, MessageId::LocaleEnglish) => "English",
            (Self::En, MessageId::LocaleRussian) => "Русский",
            (Self::Ru, MessageId::LocaleRussian) => "Русский",

            // ── Action buttons in Settings → identity & backup ──
            (Self::En, MessageId::BtnExportSeed) => "Export seed",
            (Self::Ru, MessageId::BtnExportSeed) => "Экспорт ключа",
            (Self::En, MessageId::BtnResetProgress) => "Reset progress",
            (Self::Ru, MessageId::BtnResetProgress) => "Сброс прогресса",
            (Self::En, MessageId::BtnHide) => "Hide",
            (Self::Ru, MessageId::BtnHide) => "Скрыть",
            (Self::En, MessageId::BtnResetDefaults) => "Reset to defaults",
            (Self::Ru, MessageId::BtnResetDefaults) => "Сбросить к умолчаниям",
            (Self::En, MessageId::BtnSendTestSelf) => "Send test message to self",
            (Self::Ru, MessageId::BtnSendTestSelf) => "Отправить тестовое сообщение себе",

            // ── Repo link in header ──
            (Self::En, MessageId::SourceLink) => "source ↗",
            (Self::Ru, MessageId::SourceLink) => "исходник ↗",

            // ── Panel headers (h2 / h3) ──
            (Self::En, MessageId::PanelHero) => "hero",
            (Self::Ru, MessageId::PanelHero) => "герой",
            (Self::En, MessageId::PanelEquipment) => "equipment",
            (Self::Ru, MessageId::PanelEquipment) => "снаряжение",
            (Self::En, MessageId::PanelConsumables) => "consumables",
            (Self::Ru, MessageId::PanelConsumables) => "расходники",
            (Self::En, MessageId::PanelResources) => "resources",
            (Self::Ru, MessageId::PanelResources) => "ресурсы",
            (Self::En, MessageId::PanelShop) => "shop",
            (Self::Ru, MessageId::PanelShop) => "магазин",
            (Self::En, MessageId::PanelBuyGear) => "buy gear",
            (Self::Ru, MessageId::PanelBuyGear) => "купить снаряжение",
            (Self::En, MessageId::PanelSage) => "the sage (buy skills)",
            (Self::Ru, MessageId::PanelSage) => "мудрец (купить навыки)",
            (Self::En, MessageId::PanelFarm) => "farm",
            (Self::Ru, MessageId::PanelFarm) => "ферма",
            (Self::En, MessageId::PanelWorldMap) => "world map",
            (Self::Ru, MessageId::PanelWorldMap) => "карта мира",
            (Self::En, MessageId::PanelWorldBoss) => "World Boss",
            (Self::Ru, MessageId::PanelWorldBoss) => "Мировой Босс",
            (Self::En, MessageId::PanelPlotSoFar) => "The Plot So Far…",
            (Self::Ru, MessageId::PanelPlotSoFar) => "Сюжет до сих пор…",
            (Self::En, MessageId::PanelGuilds) => "guilds",
            (Self::Ru, MessageId::PanelGuilds) => "гильдии",
            (Self::En, MessageId::PanelCreateGuild) => "create a guild",
            (Self::Ru, MessageId::PanelCreateGuild) => "создать гильдию",
            (Self::En, MessageId::PanelTutorialWelcome) => "welcome, wanderer",
            (Self::Ru, MessageId::PanelTutorialWelcome) => "добро пожаловать, странник",
            (Self::En, MessageId::PanelWhileAway) => "while you were away",
            (Self::Ru, MessageId::PanelWhileAway) => "пока тебя не было",
            (Self::En, MessageId::PanelEndings) => "endings",
            (Self::Ru, MessageId::PanelEndings) => "финалы",
            (Self::En, MessageId::PanelSkillsLine) => "skills",
            (Self::Ru, MessageId::PanelSkillsLine) => "навыки",
            (Self::En, MessageId::PanelFormsVisited) => "forms visited",
            (Self::Ru, MessageId::PanelFormsVisited) => "посещённые формы",
            (Self::En, MessageId::PanelAchievementsLow) => "achievements",
            (Self::Ru, MessageId::PanelAchievementsLow) => "достижения",
            (Self::En, MessageId::PanelHowToPlay) => "how to play",
            (Self::Ru, MessageId::PanelHowToPlay) => "как играть",

            // ── Stat / column names ──
            (Self::En, MessageId::StatName) => "Name",
            (Self::Ru, MessageId::StatName) => "Имя",
            (Self::En, MessageId::StatForm) => "Form",
            (Self::Ru, MessageId::StatForm) => "Форма",
            (Self::En, MessageId::StatLevel) => "Level",
            (Self::Ru, MessageId::StatLevel) => "Уровень",
            (Self::En, MessageId::StatXp) => "XP",
            (Self::Ru, MessageId::StatXp) => "Опыт",
            (Self::En, MessageId::StatHp) => "HP",
            (Self::Ru, MessageId::StatHp) => "ОЗ",
            (Self::En, MessageId::StatAttack) => "Attack",
            (Self::Ru, MessageId::StatAttack) => "Атака",
            (Self::En, MessageId::StatDefence) => "Defence",
            (Self::Ru, MessageId::StatDefence) => "Защита",
            (Self::En, MessageId::StatSpeed) => "Speed",
            (Self::Ru, MessageId::StatSpeed) => "Скорость",
            (Self::En, MessageId::StatEvasion) => "Evasion",
            (Self::Ru, MessageId::StatEvasion) => "Уклонение",
            (Self::En, MessageId::ResGold) => "gold",
            (Self::Ru, MessageId::ResGold) => "золото",
            (Self::En, MessageId::ResEssence) => "essence",
            (Self::Ru, MessageId::ResEssence) => "эссенция",
            (Self::En, MessageId::ResMissions) => "missions",
            (Self::Ru, MessageId::ResMissions) => "миссии",
            (Self::En, MessageId::ResBossDamage) => "boss damage",
            (Self::Ru, MessageId::ResBossDamage) => "урон по боссу",
            (Self::En, MessageId::ResPotions) => "potions",
            (Self::Ru, MessageId::ResPotions) => "зелья",
            (Self::En, MessageId::ResFireballs) => "fireballs",
            (Self::Ru, MessageId::ResFireballs) => "фаерболы",
            (Self::En, MessageId::ColSlot) => "slot",
            (Self::Ru, MessageId::ColSlot) => "слот",
            (Self::En, MessageId::ColName) => "name",
            (Self::Ru, MessageId::ColName) => "имя",
            (Self::En, MessageId::ColDamage) => "damage",
            (Self::Ru, MessageId::ColDamage) => "урон",
            (Self::En, MessageId::ColArea) => "area",
            (Self::Ru, MessageId::ColArea) => "область",
            (Self::En, MessageId::ColSeen) => "seen",
            (Self::Ru, MessageId::ColSeen) => "видели",

            // ── Action buttons ──
            (Self::En, MessageId::BtnRunMission) => "Run Mission",
            (Self::Ru, MessageId::BtnRunMission) => "В миссию",
            (Self::En, MessageId::BtnAutoOn) => "auto: on",
            (Self::Ru, MessageId::BtnAutoOn) => "авто: вкл",
            (Self::En, MessageId::BtnAutoOff) => "auto: off",
            (Self::Ru, MessageId::BtnAutoOff) => "авто: выкл",
            (Self::En, MessageId::BtnAutoEquipBest) => "Auto-Equip Best",
            (Self::Ru, MessageId::BtnAutoEquipBest) => "Лучшее снаряжение",
            (Self::En, MessageId::BtnUse) => "Use",
            (Self::Ru, MessageId::BtnUse) => "Использовать",
            (Self::En, MessageId::BtnBuy) => "Buy",
            (Self::Ru, MessageId::BtnBuy) => "Купить",
            (Self::En, MessageId::BtnWorkFarm) => "Work the Farm (+1 wheat)",
            (Self::Ru, MessageId::BtnWorkFarm) => "Работа на ферме (+1 пшеница)",
            (Self::En, MessageId::BtnSellAllWheat) => "Sell All Wheat",
            (Self::Ru, MessageId::BtnSellAllWheat) => "Продать всю пшеницу",
            (Self::En, MessageId::BtnCreate) => "Create",
            (Self::Ru, MessageId::BtnCreate) => "Создать",
            (Self::En, MessageId::BtnLeaveGuild) => "Leave guild",
            (Self::Ru, MessageId::BtnLeaveGuild) => "Покинуть гильдию",
            (Self::En, MessageId::BtnDisbandGuild) => "Disband guild",
            (Self::Ru, MessageId::BtnDisbandGuild) => "Распустить гильдию",
            (Self::En, MessageId::BtnJoin) => "Join",
            (Self::Ru, MessageId::BtnJoin) => "Вступить",
            (Self::En, MessageId::BtnEquip) => "equip",
            (Self::Ru, MessageId::BtnEquip) => "надеть",
            (Self::En, MessageId::BtnNext) => "Next",
            (Self::Ru, MessageId::BtnNext) => "Далее",
            (Self::En, MessageId::BtnStartPlaying) => "Start playing",
            (Self::Ru, MessageId::BtnStartPlaying) => "Начать игру",
            (Self::En, MessageId::BtnSkipIntro) => "Skip intro",
            (Self::Ru, MessageId::BtnSkipIntro) => "Пропустить",

            // ── Item / consumable names ──
            (Self::En, MessageId::ItemPotion) => "Potion",
            (Self::Ru, MessageId::ItemPotion) => "Зелье",
            (Self::En, MessageId::ItemFireball) => "Fireball",
            (Self::Ru, MessageId::ItemFireball) => "Фаербол",

            // ── Common terms / micro-strings ──
            (Self::En, MessageId::TermYouBattle) => "you",
            (Self::Ru, MessageId::TermYouBattle) => "ты",
            (Self::En, MessageId::TermYouBadge) => "you",
            (Self::Ru, MessageId::TermYouBadge) => "ты",
            (Self::En, MessageId::TermYouLeader) => "you",
            (Self::Ru, MessageId::TermYouLeader) => "ты",
            (Self::En, MessageId::TermLive) => "live",
            (Self::Ru, MessageId::TermLive) => "онлайн",
            (Self::En, MessageId::TermActive) => "active",
            (Self::Ru, MessageId::TermActive) => "активная",
            (Self::En, MessageId::TermOwned) => "owned",
            (Self::Ru, MessageId::TermOwned) => "куплено",
            (Self::En, MessageId::TermMaxTier) => "max tier",
            (Self::Ru, MessageId::TermMaxTier) => "макс. уровень",
            (Self::En, MessageId::TermEmpty) => "Empty",
            (Self::Ru, MessageId::TermEmpty) => "Пусто",
            (Self::En, MessageId::TermFormNa) => "n/a (form)",
            (Self::Ru, MessageId::TermFormNa) => "недоступно (форма)",
            (Self::En, MessageId::TermFormLocks) => "form locks this slot",
            (Self::Ru, MessageId::TermFormLocks) => "форма блокирует этот слот",
            (Self::En, MessageId::TermNever) => "never",
            (Self::Ru, MessageId::TermNever) => "никогда",
            (Self::En, MessageId::TermWin) => "win",
            (Self::Ru, MessageId::TermWin) => "победа",
            (Self::En, MessageId::TermDefeat) => "defeat",
            (Self::Ru, MessageId::TermDefeat) => "поражение",
            (Self::En, MessageId::TermPubkeyHidden) => "pubkey hidden (toggle in advanced to reveal)",
            (Self::Ru, MessageId::TermPubkeyHidden) => "ключ скрыт (откройте «продвинутые», чтобы показать)",
            (Self::En, MessageId::TermPubkeyPending) => "pubkey: pending delegate response",
            (Self::Ru, MessageId::TermPubkeyPending) => "ключ: ждём ответа делегата",
            (Self::En, MessageId::TermPubkeyPendingShort) => "pubkey pending...",
            (Self::Ru, MessageId::TermPubkeyPendingShort) => "ключ загружается…",

            // ── Onboarding wizard titles & body (plain text — bold markup is dropped) ──
            (Self::En, MessageId::OnbTitleWelcome) => "Welcome to Freenet Idle",
            (Self::Ru, MessageId::OnbTitleWelcome) => "Добро пожаловать в Freenet Idle",
            (Self::En, MessageId::OnbBodyWelcome1) => "Your hero, inventory, and identity live on the local Freenet node — not in this browser tab. Clearing your cookies, switching browsers, or reloading the page won't lose anything.",
            (Self::Ru, MessageId::OnbBodyWelcome1) => "Твой герой, инвентарь и личность хранятся на локальном узле Freenet — а не в этой вкладке браузера. Очистка cookie, смена браузера или перезагрузка страницы ничего не потеряют.",
            (Self::En, MessageId::OnbBodyWelcome2) => "If the node ever rebuilds, you can back up your seed from Settings → Backup & Migration.",
            (Self::Ru, MessageId::OnbBodyWelcome2) => "Если узел придётся пересобрать, сохрани seed в Настройки → Резервная копия и миграция.",

            (Self::En, MessageId::OnbTitleLoop) => "The loop",
            (Self::Ru, MessageId::OnbTitleLoop) => "Игровой цикл",
            (Self::En, MessageId::OnbBodyLoop1) => "Click Run Mission on the Farm tab. Each mission is a chain of up to 5 encounters — wins drop gear, potions, and fireballs at fixed cadences. Lose to a non-mundane enemy and you'll transform into them, permanently.",
            (Self::Ru, MessageId::OnbBodyLoop1) => "Нажми «В миссию» на вкладке Ферма. Каждая миссия — цепочка до 5 сражений; победы дают снаряжение, зелья и фаерболы по фиксированному расписанию. Проиграешь не-обычному врагу — навсегда превратишься в него.",
            (Self::En, MessageId::OnbBodyLoop2) => "Every form you wear leaves a permanent skill — the prestige loop.",
            (Self::Ru, MessageId::OnbBodyLoop2) => "Каждая форма оставляет постоянный навык — это и есть петля прокачки.",

            (Self::En, MessageId::OnbTitleAuto) => "Auto-mission",
            (Self::Ru, MessageId::OnbTitleAuto) => "Авто-миссия",
            (Self::En, MessageId::OnbBodyAuto1) => "Toggle auto: on to let the hero fight on its own. Close the tab and come back later — the delegate simulates the missions you missed (up to ~1 hour at a time) and shows a summary when you return.",
            (Self::Ru, MessageId::OnbBodyAuto1) => "Включи «авто: вкл», чтобы герой сражался сам. Можешь закрыть вкладку и вернуться позже — делегат проиграет пропущенные миссии (до ~1 часа за раз) и покажет сводку при возвращении.",
            (Self::En, MessageId::OnbBodyAuto2) => "Set an HP-pause threshold in Settings if you'd rather not get auto-defeated.",
            (Self::Ru, MessageId::OnbBodyAuto2) => "Если не хочешь автоматических поражений, выстави порог по ОЗ в Настройках.",

            (Self::En, MessageId::OnbTitleTabs) => "Tabs & Settings",
            (Self::Ru, MessageId::OnbTitleTabs) => "Вкладки и настройки",
            (Self::En, MessageId::OnbBodyTabs1) => "🗺 World Map switches biomes once you out-level the current one. 🛒 Shop buys gear and potions, sells stash, forges duplicates, and trades wheat for gold. ⚙ Settings has themes, sync cadence, identity backup, and advanced toggles.",
            (Self::Ru, MessageId::OnbBodyTabs1) => "🗺 «Карта мира» меняет биом, когда перерастаешь текущий. 🛒 «Магазин» покупает снаряжение и зелья, продаёт запас, кует дубликаты и меняет пшеницу на золото. ⚙ «Настройки» — темы, частота синхронизации, резервная копия личности и продвинутые переключатели.",
            (Self::En, MessageId::OnbBodyTabs2) => "Click ❔ Help any time for the full reference.",
            (Self::Ru, MessageId::OnbBodyTabs2) => "Нажми ❔ «Помощь» в любой момент, чтобы открыть полный справочник.",

            // ── Tutorial banner shown to brand-new account ──
            (Self::En, MessageId::TutorialBody1) => "Click Run Mission to fight the area's enemy. Every 5 wins drop gear (manage at the Shop tab), every 13 wins drop a potion, every 19 a fireball.",
            (Self::Ru, MessageId::TutorialBody1) => "Нажми «В миссию», чтобы сразиться с врагом локации. Каждые 5 побед выпадает снаряжение (управление на вкладке Магазин), каждые 13 побед — зелье, каждые 19 — фаербол.",
            (Self::En, MessageId::TutorialBody2) => "Take damage in combat? HP regenerates over time, or use a potion to heal instantly. Pick a different battlefield from the World Map when you out-level the current one.",
            (Self::Ru, MessageId::TutorialBody2) => "Получил урон в бою? ОЗ восстанавливается со временем, либо используй зелье для мгновенного лечения. Сменить поле боя можно на Карте мира, когда перерастёшь текущее.",

            // ── Mid-battle queue hints ──
            (Self::En, MessageId::BattleOpeningTurn) => "(opening turn — combatants are sizing each other up)",
            (Self::Ru, MessageId::BattleOpeningTurn) => "(первый ход — соперники присматриваются друг к другу)",
            (Self::En, MessageId::BattleNoEncounters) => "no encounters yet — Run Mission to fight",
            (Self::Ru, MessageId::BattleNoEncounters) => "пока сражений не было — нажми «В миссию»",
            (Self::En, MessageId::BattlePotionQueued) => "potion queued — applies on next turn",
            (Self::Ru, MessageId::BattlePotionQueued) => "зелье в очереди — сработает на следующем ходу",
            (Self::En, MessageId::BattleFireballQueued) => "fireball queued — applies on next turn",
            (Self::Ru, MessageId::BattleFireballQueued) => "фаербол в очереди — сработает на следующем ходу",
            (Self::En, MessageId::BattleMissed) => "(missed)",
            (Self::Ru, MessageId::BattleMissed) => "(промах)",

            // ── Mailbox panel ──
            (Self::En, MessageId::MailboxEmpty) => "(no messages yet — click the button above to round-trip a chat)",
            (Self::Ru, MessageId::MailboxEmpty) => "(сообщений пока нет — нажми кнопку выше, чтобы прогнать тестовый чат)",
            (Self::En, MessageId::MailboxKindChat) => "chat",
            (Self::Ru, MessageId::MailboxKindChat) => "чат",
            (Self::En, MessageId::MailboxKindGift) => "gift",
            (Self::Ru, MessageId::MailboxKindGift) => "подарок",
            (Self::En, MessageId::MailboxKindGuildInvite) => "guild-invite",
            (Self::Ru, MessageId::MailboxKindGuildInvite) => "приглашение",
            (Self::En, MessageId::MailboxKindTradeOffer) => "trade-offer",
            (Self::Ru, MessageId::MailboxKindTradeOffer) => "обмен",

            // ── Catch-up banner ──
            (Self::En, MessageId::CatchupClearsHint) => "(Banner clears when you run a mission.)",
            (Self::Ru, MessageId::CatchupClearsHint) => "(Баннер исчезнет после следующей миссии.)",

            // ── Settings descriptive paragraphs ──
            (Self::En, MessageId::SettingsThemeDesc) => "Pick a palette. Saved to this browser's local storage; takes effect immediately and persists across reloads.",
            (Self::Ru, MessageId::SettingsThemeDesc) => "Выбери палитру. Сохраняется в локальном хранилище браузера; применяется мгновенно и переживает перезагрузки.",
            (Self::En, MessageId::SettingsCadenceDesc) => "How often the webapp talks to your local node. Aggressive = snappier leaderboard, more node traffic. Easy = lighter, but the contract prunes you after 60 s of silence so don't go past that.",
            (Self::Ru, MessageId::SettingsCadenceDesc) => "Как часто веб-приложение общается с локальным узлом. Агрессивно = таблица лидеров обновляется быстрее, больше трафика. Спокойно = легче, но контракт удаляет тебя после 60 с тишины — не выходи за этот предел.",
            (Self::En, MessageId::SettingsAutoMissionDesc) => "Pause the auto-loop when HP drops below this fraction of your maximum. 0% keeps the old behaviour — only stop at 0 HP. Higher values save you from losing HP/forms/consumables to a string of bad rolls.",
            (Self::Ru, MessageId::SettingsAutoMissionDesc) => "Останавливать авто-цикл, когда ОЗ опускаются ниже этой доли от максимума. 0% — старое поведение, остановка только при 0 ОЗ. Большие значения спасают ОЗ/форму/расходники от череды неудач.",
            (Self::En, MessageId::SettingsPublishCheckbox) => " publish immediately after a mission (in addition to the periodic heartbeat)",
            (Self::Ru, MessageId::SettingsPublishCheckbox) => " публиковать сразу после миссии (в дополнение к периодическому heartbeat)",
            (Self::En, MessageId::SettingsIdentityBody) => "Export the Ed25519 seed to move identity to another node, or wipe inventory back to a fresh-character state. ",
            (Self::Ru, MessageId::SettingsIdentityBody) => "Экспортируй Ed25519 seed, чтобы перенести личность на другой узел, или обнули инвентарь до состояния новичка. ",
            (Self::En, MessageId::SettingsIdentityBodyStrong) => "Reset progress is destructive",
            (Self::Ru, MessageId::SettingsIdentityBodyStrong) => "Сброс прогресса разрушителен",
            (Self::En, MessageId::SettingsIdentityBodyTail) => " — pubkey survives but every counter, item, skill, and ending goes to zero.",
            (Self::Ru, MessageId::SettingsIdentityBodyTail) => " — публичный ключ остаётся, но все счётчики, предметы, навыки и финалы обнуляются.",
            (Self::En, MessageId::SettingsAdvancedDesc) => "Lower-traffic / privacy / debug switches. Defaults are fine for most players.",
            (Self::Ru, MessageId::SettingsAdvancedDesc) => "Переключатели трафика / приватности / отладки. Для большинства игроков подходят значения по умолчанию.",
            (Self::En, MessageId::SettingsHidePubkey) => " hide pubkey (Hero panel + Settings)",
            (Self::Ru, MessageId::SettingsHidePubkey) => " скрыть публичный ключ (на панели Героя и в Настройках)",
            (Self::En, MessageId::SettingsHideStale) => " hide stale players from leaderboard (last seen > 30 s ago)",
            (Self::Ru, MessageId::SettingsHideStale) => " скрывать неактивных игроков из таблицы лидеров (видели > 30 с назад)",
            (Self::En, MessageId::SettingsWsOverride) => "WS URL override (empty = use ?ws= or default; takes effect after page reload):",
            (Self::Ru, MessageId::SettingsWsOverride) => "Переопределение URL WS (пусто = ?ws= или по умолчанию; применится после перезагрузки страницы):",
            (Self::En, MessageId::SettingsResetUiPrefsDesc) => "Clears theme + cadence + auto-pause + advanced toggles and reloads the page. Doesn't touch your inventory — that lives on the node.",
            (Self::Ru, MessageId::SettingsResetUiPrefsDesc) => "Сбрасывает тему + частоту + авто-паузу + продвинутые переключатели и перезагружает страницу. Инвентарь не трогается — он живёт на узле.",
            (Self::En, MessageId::SettingsWhereStateBody) => "Local view is just a cache of what lives on the node. Reload the page — identity and inventory come straight back from the delegate. To actually delete your save, wipe `~/.config/freenet/secrets/local/<delegate-key>/` on the node.",
            (Self::Ru, MessageId::SettingsWhereStateBody) => "Локальное представление — лишь кеш того, что живёт на узле. Перезагрузи страницу — личность и инвентарь придут от делегата. Чтобы реально удалить сохранение, очисти `~/.config/freenet/secrets/local/<delegate-key>/` на узле.",
            (Self::En, MessageId::SettingsSeedRevealWarn) => "Copy this once. Anyone with these bytes can impersonate you on the contract.",
            (Self::Ru, MessageId::SettingsSeedRevealWarn) => "Скопируй один раз. Любой, кто получит эти байты, сможет выдать себя за тебя в контракте.",

            // ── Guilds panel descriptive copy ──
            (Self::En, MessageId::GuildsPanelDesc) => "Cooperative groups — early scaffolding. Create one, others can join by id. Each player is in at most one guild; leaders auto-handoff on leave.",
            (Self::Ru, MessageId::GuildsPanelDesc) => "Кооперативные группы — ранний каркас. Создай свою, другие могут вступить по id. Игрок может быть только в одной гильдии; лидер передаётся автоматически при уходе.",
            (Self::En, MessageId::GuildsContractMissing) => "Guilds contract not configured. Publish ",
            (Self::Ru, MessageId::GuildsContractMissing) => "Контракт гильдий не настроен. Опубликуй ",
            (Self::En, MessageId::GuildsContractMissingTail) => " (extension WIP) or override the keys in ",
            (Self::Ru, MessageId::GuildsContractMissingTail) => " (доработка WIP) или переопредели ключи в ",
            (Self::En, MessageId::GuildsEmptyList) => "(no guilds yet — be the first)",
            (Self::Ru, MessageId::GuildsEmptyList) => "(гильдий пока нет — будь первым)",
            (Self::En, MessageId::GuildsViaScript) => " via ",
            (Self::Ru, MessageId::GuildsViaScript) => " через ",
            (Self::En, MessageId::GuildNamePlaceholder) => "guild name (≤ 32 bytes)",
            (Self::Ru, MessageId::GuildNamePlaceholder) => "название гильдии (≤ 32 байт)",

            (Self::En, MessageId::MailboxNotConfiguredHead) => "Mailbox contract not configured. Publish ",
            (Self::Ru, MessageId::MailboxNotConfiguredHead) => "Контракт почты не настроен. Опубликуй ",
            (Self::En, MessageId::MailboxNotConfiguredVia) => " via ",
            (Self::Ru, MessageId::MailboxNotConfiguredVia) => " через ",
            (Self::En, MessageId::MailboxNotConfiguredTail) => " (extension WIP) or set ",
            (Self::Ru, MessageId::MailboxNotConfiguredTail) => " (доработка WIP) или укажи ",
            (Self::En, MessageId::MailboxNotConfiguredIn) => " in ",
            (Self::Ru, MessageId::MailboxNotConfiguredIn) => " в ",

            // ── Tooltips & ambient hints (HTML title attribute) ──
            (Self::En, MessageId::TipFightInProgress) => "fight in progress — wait for the current battle to end",
            (Self::Ru, MessageId::TipFightInProgress) => "идёт бой — дождись окончания текущей схватки",
            (Self::En, MessageId::TipEstateBlocksCombat) => "Estate is active — pause it from the Estate panel to fight",
            (Self::Ru, MessageId::TipEstateBlocksCombat) => "Поместье активно — останови его на панели поместья, чтобы сражаться",
            (Self::En, MessageId::TipAutoToggleMidFight) => "auto toggle still works during a fight — the new setting takes effect once the current battle ends",
            (Self::Ru, MessageId::TipAutoToggleMidFight) => "переключатель авто работает и в бою — новое значение применится после окончания текущей схватки",
            (Self::En, MessageId::TipAutoEquipBest) => "walk every slot and equip the highest stat-sum piece you own",
            (Self::Ru, MessageId::TipAutoEquipBest) => "пройти по слотам и надеть лучшие предметы с наибольшей суммой характеристик",
            (Self::En, MessageId::TipAutoEquipNothing) => "nothing in the stash beats what you're already wearing — drop some loot or change form first",
            (Self::Ru, MessageId::TipAutoEquipNothing) => "в запасе нет ничего лучше уже надетого — собери ещё лута или смени форму",
            (Self::En, MessageId::TipPotionQueue) => "queue: heal to full on the next combat turn",
            (Self::Ru, MessageId::TipPotionQueue) => "очередь: полное лечение на следующем ходу",
            (Self::En, MessageId::TipPotionIdle) => "heals HP fully",
            (Self::Ru, MessageId::TipPotionIdle) => "полностью восстанавливает ОЗ",
            (Self::En, MessageId::TipFireballQueue) => "queue: bonus damage on the next combat turn",
            (Self::Ru, MessageId::TipFireballQueue) => "очередь: дополнительный урон на следующем ходу",
            (Self::En, MessageId::TipUnequipSlot) => "unequip — return to stash",
            (Self::Ru, MessageId::TipUnequipSlot) => "снять — отправить в запас",
            (Self::En, MessageId::TipDisbandLeader) => "leader-only: delete the guild for everyone",
            (Self::Ru, MessageId::TipDisbandLeader) => "только для лидера: удалить гильдию для всех",
            (Self::En, MessageId::PotionShopDesc) => "fully heals your HP",
            (Self::Ru, MessageId::PotionShopDesc) => "полностью восстанавливает ОЗ",
            (Self::En, MessageId::TermCorrupt) => "(corrupt)",
            (Self::Ru, MessageId::TermCorrupt) => "(повреждено)",

            // ── Shop sub-panel descriptive copy ──
            (Self::En, MessageId::ShopStashDesc) => "items grouped by slot — equip to wear, sell back to the merchant for tier-priced gold",
            (Self::Ru, MessageId::ShopStashDesc) => "предметы по слотам — надевай или продавай купцу за золото по тиру",
            (Self::En, MessageId::ShopBuyGearDesc) => "pre-rolled equipment at the smithy. each click of Buy adds one piece of the requested slot+tier to your stash. legendary (T4) only via forge or drop.",
            (Self::Ru, MessageId::ShopBuyGearDesc) => "готовое снаряжение в кузнице. Каждое нажатие «Купить» добавляет один предмет нужного слота и тира в запас. Легендарный (T4) — только ковка или дроп.",
            (Self::En, MessageId::ShopSageDesc) => "the Sage trades permanent skill lore for essence. Veteran/Champion still come from level milestones — those aren't for sale.",
            (Self::Ru, MessageId::ShopSageDesc) => "Мудрец меняет постоянные знания навыков на эссенцию. Ветеран/Чемпион по-прежнему открываются за уровни — их купить нельзя.",
            (Self::En, MessageId::ShopFarmDesc) => "safe non-combat income. each Work click yields +1 wheat; the merchant pays 1 gold per 10 wheat.",
            (Self::Ru, MessageId::ShopFarmDesc) => "безопасный доход без боя. Каждое нажатие «Работа» приносит +1 пшеницы; купец платит 1 золото за 10 пшеницы.",
            (Self::En, MessageId::ShopFarmDescPassive) => "your Estate Farmhand workers now produce wheat passively — the merchant still buys it at 1 gold per 10.",
            (Self::Ru, MessageId::ShopFarmDescPassive) => "работники-крестьяне твоего Поместья теперь приносят пшеницу пассивно — купец по-прежнему берёт её по 1 золоту за 10.",

            // ── Help-tab subheaders & sections ──
            (Self::En, MessageId::HelpTheLoop) => "the loop",
            (Self::Ru, MessageId::HelpTheLoop) => "цикл",
            (Self::En, MessageId::HelpStats) => "stats",
            (Self::Ru, MessageId::HelpStats) => "характеристики",
            (Self::En, MessageId::HelpFormsTransformation) => "forms & transformation",
            (Self::Ru, MessageId::HelpFormsTransformation) => "формы и перевоплощения",
            (Self::En, MessageId::HelpTabs) => "tabs",
            (Self::Ru, MessageId::HelpTabs) => "вкладки",
            (Self::En, MessageId::HelpShopGear) => "shop & gear",
            (Self::Ru, MessageId::HelpShopGear) => "магазин и снаряжение",
            (Self::En, MessageId::HelpConsumables) => "consumables",
            (Self::Ru, MessageId::HelpConsumables) => "расходники",
            (Self::En, MessageId::HelpWorldBoss) => "world boss",
            (Self::Ru, MessageId::HelpWorldBoss) => "мировой босс",
            (Self::En, MessageId::HelpDelegateWhat) => "what does the delegate do?",
            (Self::Ru, MessageId::HelpDelegateWhat) => "что делает делегат?",
            (Self::En, MessageId::HelpGuildsMailbox) => "guilds & mailbox (early)",
            (Self::Ru, MessageId::HelpGuildsMailbox) => "гильдии и почта (ранняя стадия)",
            (Self::En, MessageId::HelpEstate) => "estate — passive income loop",
            (Self::Ru, MessageId::HelpEstate) => "поместье — петля пассивного дохода",
            (Self::En, MessageId::HelpLegacy) => "legacy — personal prestige",
            (Self::Ru, MessageId::HelpLegacy) => "наследие — личный престиж",
            (Self::En, MessageId::HelpAreaGraph) => "world map — graph layout",
            (Self::Ru, MessageId::HelpAreaGraph) => "карта мира — граф",

            // Estate panel (B2)
            (Self::En, MessageId::PanelEstate) => "Estate",
            (Self::Ru, MessageId::PanelEstate) => "Поместье",
            (Self::En, MessageId::EstateBtnPause) => "Pause Estate",
            (Self::Ru, MessageId::EstateBtnPause) => "Остановить поместье",
            (Self::En, MessageId::EstateBtnRun) => "Run Estate",
            (Self::Ru, MessageId::EstateBtnRun) => "Запустить поместье",
            (Self::En, MessageId::EstateColTier) => "Tier",
            (Self::Ru, MessageId::EstateColTier) => "Звено",
            (Self::En, MessageId::EstateColOwned) => "Owned",
            (Self::Ru, MessageId::EstateColOwned) => "Нанято",
            (Self::En, MessageId::EstateColYield) => "Yield/s",
            (Self::Ru, MessageId::EstateColYield) => "Доход/с",
            (Self::En, MessageId::EstateColNextPrice) => "Next price",
            (Self::Ru, MessageId::EstateColNextPrice) => "След. цена",
            (Self::En, MessageId::BtnHire) => "Hire",
            (Self::Ru, MessageId::BtnHire) => "Нанять",
            (Self::En, MessageId::EstateResWheat) => "wheat",
            (Self::Ru, MessageId::EstateResWheat) => "пшеницы",
            (Self::En, MessageId::EstateResGold) => "gold",
            (Self::Ru, MessageId::EstateResGold) => "золота",
            (Self::En, MessageId::EstateResEssence) => "essence",
            (Self::Ru, MessageId::EstateResEssence) => "эссенции",

            // Legacy panel (C1)
            (Self::En, MessageId::PanelLegacy) => "Legacy",
            (Self::Ru, MessageId::PanelLegacy) => "Наследие",
            (Self::En, MessageId::LegacyColNode) => "Node",
            (Self::Ru, MessageId::LegacyColNode) => "Узел",
            (Self::En, MessageId::LegacyColLevel) => "Level",
            (Self::Ru, MessageId::LegacyColLevel) => "Уровень",
            (Self::En, MessageId::LegacyColMultiplier) => "Multiplier",
            (Self::Ru, MessageId::LegacyColMultiplier) => "Множитель",
            (Self::En, MessageId::LegacyColNextCost) => "Next cost",
            (Self::Ru, MessageId::LegacyColNextCost) => "След. цена",
            (Self::En, MessageId::BtnAscend) => "Ascend",
            (Self::Ru, MessageId::BtnAscend) => "Вознестись",
            (Self::En, MessageId::LegacyAscendBlurb) =>
                "Soft-reset: keep stars, level, missions, skills. Wipe gold, gear, Estate.",
            (Self::Ru, MessageId::LegacyAscendBlurb) =>
                "Мягкий сброс: звёзды, уровень, миссии и навыки остаются. Сбрасываются золото, экипировка и поместье.",
            (Self::En, MessageId::LegacyAscendConfirm) =>
                "Ascend — soft-reset run? Keeps stars, level, mission count, and skills. Wipes gold, gear, and Estate.",
            (Self::Ru, MessageId::LegacyAscendConfirm) =>
                "Вознестись — мягкий сброс? Звёзды, уровень, счётчик миссий и навыки сохранятся. Золото, экипировка и поместье обнулятся.",

            // Catchup modal (B4)
            (Self::En, MessageId::CatchupModalTitle) => "Welcome back",
            (Self::Ru, MessageId::CatchupModalTitle) => "С возвращением",
            (Self::En, MessageId::BtnGotIt) => "Got it",
            (Self::Ru, MessageId::BtnGotIt) => "Понятно",
            (Self::En, MessageId::NewerBuildDesc) =>
                "A newer build is live — no detailed changelog this time.",
            (Self::Ru, MessageId::NewerBuildDesc) =>
                "Сейчас работает более свежая сборка — подробного списка изменений на этот раз нет.",

            (Self::En, MessageId::PanelFormsShop) => "Forms",
            (Self::Ru, MessageId::PanelFormsShop) => "Формы",
            (Self::En, MessageId::FormsShopDesc) =>
                "Reset your shape (Human is cheap) or commit gold to one of the other four forms. Each form also drives Estate affinity — Horse buffs Farmhand + Forager, Dragon buffs Trader + Sage, Cat buffs Forager + Sage, Slime gives a flat +30% across the board, Human is neutral. Direct purchase mirrors a defeat-induced transformation: the form is added to your visited set so its skill unlocks at the Sage.",
            (Self::Ru, MessageId::FormsShopDesc) =>
                "Сбрось облик (Человек стоит дёшево) или вложи золото в одну из четырёх остальных форм. Каждая форма ещё и определяет аффинити в Поместье — Конь усиливает Работника+Собирателя, Дракон — Торговца+Мудреца, Кот — Собирателя+Мудреца, Слизь даёт +30% по всем звеньям, Человек нейтрален. Покупка работает как трансформация после поражения: форма добавляется в твой набор посещённых, и её навык открывается у Мудреца.",
            (Self::En, MessageId::FormsShopBaselineDesc) => "balanced baseline — no stat bundle",
            (Self::Ru, MessageId::FormsShopBaselineDesc) => "сбалансированная база — без бонусов",
            (Self::En, MessageId::TipFormAlreadyActive) => "you are already in this form",
            (Self::Ru, MessageId::TipFormAlreadyActive) => "ты уже в этой форме",

            // Activities (A1)
            (Self::En, MessageId::PanelActivities) => "Activities",
            (Self::Ru, MessageId::PanelActivities) => "Занятия",
            (Self::En, MessageId::ActivitiesDesc) =>
                "Non-combat actions tied to the current zone. Picking one sets it as your idle action — auto-mission and Estate pause until you stop.",
            (Self::Ru, MessageId::ActivitiesDesc) =>
                "Неборевые действия, привязанные к текущей зоне. Выбор делает занятие активным простойным действием — авто-миссия и Поместье паузятся до остановки.",
            (Self::En, MessageId::ActivityStart) => "Start",
            (Self::Ru, MessageId::ActivityStart) => "Начать",
            (Self::En, MessageId::ActivityStop) => "Stop",
            (Self::Ru, MessageId::ActivityStop) => "Остановить",

            // Routine (B1)
            (Self::En, MessageId::PanelRoutine) => "Routine",
            (Self::Ru, MessageId::PanelRoutine) => "Распорядок",
            (Self::En, MessageId::RoutineDesc) =>
                "Auto-hire Estate workers up to the target count when gold permits. Capped at 50 hires per delegate tick so a fat catchup window can't drain the treasury.",
            (Self::Ru, MessageId::RoutineDesc) =>
                "Автоматический найм работников Поместья до целевой численности при наличии золота. Ограничено 50 наймами за один тик делегата, чтобы окно догонки не выкосило казну.",
            (Self::En, MessageId::RoutineColTier) => "Tier",
            (Self::Ru, MessageId::RoutineColTier) => "Звено",
            (Self::En, MessageId::RoutineColCurrent) => "Owned",
            (Self::Ru, MessageId::RoutineColCurrent) => "Нанято",
            (Self::En, MessageId::RoutineColTarget) => "Target",
            (Self::Ru, MessageId::RoutineColTarget) => "Цель",

            // Insight (B5)
            (Self::En, MessageId::PanelInsight) => "Insight",
            (Self::Ru, MessageId::PanelInsight) => "Прозрение",
            (Self::En, MessageId::InsightDesc) =>
                "Rare currency. Earned every 25 missions and by the Astral 'Decode sigils' activity. Spent on small, permanent buffs.",
            (Self::Ru, MessageId::InsightDesc) =>
                "Редкая валюта. Капает каждые 25 миссий + от астрального занятия «Расшифровка рун». Тратится на маленькие постоянные бонусы.",
            (Self::En, MessageId::InsightColNode) => "Node",
            (Self::Ru, MessageId::InsightColNode) => "Узел",
            (Self::En, MessageId::InsightColLevel) => "Level",
            (Self::Ru, MessageId::InsightColLevel) => "Уровень",
            (Self::En, MessageId::InsightColNextCost) => "Next cost",
            (Self::Ru, MessageId::InsightColNextCost) => "След. цена",

            // Boss attack (C1)
            (Self::En, MessageId::PanelBossAttack) => "Personal Boss Attack",
            (Self::Ru, MessageId::PanelBossAttack) => "Личный удар по Боссу",
            (Self::En, MessageId::BossAttackBtn) => "Attack (-200 essence, +50 boss dmg)",
            (Self::Ru, MessageId::BossAttackBtn) => "Атаковать (-200 эсс, +50 урон по Боссу)",
            (Self::En, MessageId::BossAttackDesc) =>
                "Spend essence to chip the shared World Boss outside combat. Unlocks at mission_count ≥ 100, level ≥ 10, and at least one Estate worker.",
            (Self::Ru, MessageId::BossAttackDesc) =>
                "Тратишь эссенцию, чтобы ударить общего Мирового Босса вне боя. Открывается при ≥ 100 миссиях, ≥ 10 уровне и хотя бы одном работнике Поместья.",
            (Self::En, MessageId::BossAttackLocked) =>
                "Locked — need 100 missions, level 10, and at least one Estate worker.",
            (Self::Ru, MessageId::BossAttackLocked) =>
                "Закрыто — нужны 100 миссий, 10 уровень и хотя бы один работник Поместья.",

            // Tokens (C2)
            (Self::En, MessageId::PanelTokens) => "Tokens",
            (Self::Ru, MessageId::PanelTokens) => "Жетоны",
            (Self::En, MessageId::TokensDesc) =>
                "Earned one per 500 personal boss damage. Spent on cosmetic perks today; gameplay perks (gear slot, second auto-mission preset) unlock as their plumbing lands.",
            (Self::Ru, MessageId::TokensDesc) =>
                "Один жетон за каждые 500 личного урона по Боссу. Сейчас тратятся на косметические бонусы; игровые (слот, второй пресет авто-миссии) активируются по мере готовности механики.",
            (Self::En, MessageId::TokenColPerk) => "Perk",
            (Self::Ru, MessageId::TokenColPerk) => "Бонус",
            (Self::En, MessageId::TokenColPrice) => "Price",
            (Self::Ru, MessageId::TokenColPrice) => "Цена",
            (Self::En, MessageId::BtnUnlock) => "Unlock",
            (Self::Ru, MessageId::BtnUnlock) => "Открыть",

            (Self::En, MessageId::ResInsight) => "insight",
            (Self::Ru, MessageId::ResInsight) => "прозрения",
            (Self::En, MessageId::ResTokens) => "tokens",
            (Self::Ru, MessageId::ResTokens) => "жетонов",
            (Self::En, MessageId::MasteryIntro) =>
                "Permanent upgrades — bought once, kept forever. Legacy stars come from level milestones; Insight from missions and Astral activities; Tokens from personal boss damage. Routine targets and the World Boss Attack live here too.",
            (Self::Ru, MessageId::MasteryIntro) =>
                "Постоянные улучшения — покупаются один раз, остаются навсегда. Звёзды Наследия — за уровни; Прозрение — за миссии и астральные занятия; Жетоны — за личный урон по Боссу. Цели Распорядка и Удар по Мировому Боссу — тоже здесь.",
            (Self::En, MessageId::PanelWilds) => "Wilds",
            (Self::Ru, MessageId::PanelWilds) => "Дикие земли",
            (Self::En, MessageId::WildsDesc) =>
                "Late-game alternate map, procedurally generated from your plot seed. Names + enemy stat noise are unique to you; topology is fixed (8 nodes, two branches off the entrance, a confluence node). No World Boss contribution.",
            (Self::Ru, MessageId::WildsDesc) =>
                "Альтернативная карта поздней игры, процедурно сгенерированная из твоего сюжетного зерна. Названия и разброс характеристик уникальны для тебя; топология фиксирована (8 узлов, две ветви от входа, узел-схождение). Урон по Мировому Боссу не наносится.",
            (Self::En, MessageId::MapViewLinear) => "Linear",
            (Self::Ru, MessageId::MapViewLinear) => "Основная",
            (Self::En, MessageId::MapViewWilds) => "Wilds",
            (Self::Ru, MessageId::MapViewWilds) => "Дикие земли",

            // German / French / Spanish / Japanese (C5). Selective
            // overrides for the highest-impact surface area (tabs,
            // status pills, boot strings) — anything not listed falls
            // through to English via each language's catch-all.
            (Self::De, m) => Self::tr_de(m).unwrap_or_else(|| Self::En.tr(m)),
            (Self::Fr, m) => Self::tr_fr(m).unwrap_or_else(|| Self::En.tr(m)),
            (Self::Es, m) => Self::tr_es(m).unwrap_or_else(|| Self::En.tr(m)),
            (Self::Ja, m) => Self::tr_ja(m).unwrap_or_else(|| Self::En.tr(m)),
        }
    }

    /// Curated German overrides. Add entries here as translation
    /// coverage grows; missing ids fall back to English.
    fn tr_de(msg: MessageId) -> Option<&'static str> {
        Some(match msg {
            MessageId::BootLoading => "Lädt…",
            MessageId::StatusAskingDelegate => "frage Delegate nach Identität…",
            MessageId::StatusRegisteringDelegate => "registriere Delegate…",
            MessageId::StatusSubscribing => "abonniere…",
            MessageId::TabFarm => "Hof",
            MessageId::TabWorldMap => "Weltkarte",
            MessageId::TabShop => "Laden",
            MessageId::TabGuilds => "Gilden",
            MessageId::TabAchievements => "Erfolge",
            MessageId::TabSettings => "Einstellungen",
            MessageId::TabHelp => "Hilfe",
            MessageId::PillDefeated => "BESIEGT",
            MessageId::PillAdventuring => "AUF ABENTEUER",
            MessageId::PillFocusing => "KONZENTRIERT",
            MessageId::PillRecovering => "ERHOLUNG",
            MessageId::PillReady => "BEREIT",
            MessageId::PillEstate => "GUT",
            _ => return None,
        })
    }

    /// French overrides. Exhaustive — every `MessageId` is covered.
    /// Returns `Some` always; the `Option` shape is kept for symmetry
    /// with `tr_de` (partial coverage).
    fn tr_fr(msg: MessageId) -> Option<&'static str> {
        Some(match msg {
            MessageId::BootLoading => "Chargement…",
            MessageId::StatusAskingDelegate => "demande d'identité au délégué…",
            MessageId::StatusRegisteringDelegate => "enregistrement du délégué…",
            MessageId::StatusSubscribing => "abonnement…",
            MessageId::TabFarm => "Ferme",
            MessageId::TabWorldMap => "Carte du monde",
            MessageId::TabShop => "Boutique",
            MessageId::TabGuilds => "Guildes",
            MessageId::TabAchievements => "Succès",
            MessageId::TabMastery => "Maîtrise",
            MessageId::TabSettings => "Paramètres",
            MessageId::TabHelp => "Aide",
            MessageId::PillDefeated => "VAINCU",
            MessageId::PillAdventuring => "EN AVENTURE",
            MessageId::PillFocusing => "CONCENTRATION",
            MessageId::PillRecovering => "RÉCUPÉRATION",
            MessageId::PillReady => "PRÊT",
            MessageId::PillEstate => "DOMAINE",
            MessageId::SettingsTitle => "paramètres",
            MessageId::SettingsTheme => "thème",
            MessageId::SettingsLanguage => "langue",
            MessageId::SettingsSyncCadence => "cadence de synchronisation",
            MessageId::SettingsAutoMission => "mission auto",
            MessageId::SettingsPublishBehavior => "comportement de publication",
            MessageId::SettingsIdentityBackup => "identité et sauvegarde",
            MessageId::SettingsAdvanced => "avancé",
            MessageId::SettingsResetUiPrefs => "réinitialiser les préférences UI",
            MessageId::SettingsMailbox => "boîte (test D2D)",
            MessageId::SettingsWhereStateLives => "où vit l'état",
            MessageId::LocaleEnglish => "English",
            MessageId::LocaleRussian => "Русский",
            MessageId::BtnExportSeed => "Exporter la seed",
            MessageId::BtnResetProgress => "Réinitialiser la progression",
            MessageId::BtnHide => "Masquer",
            MessageId::BtnResetDefaults => "Rétablir les défauts",
            MessageId::BtnSendTestSelf => "Envoyer un message test à soi-même",
            MessageId::SourceLink => "source ↗",
            MessageId::PanelHero => "héros",
            MessageId::PanelEquipment => "équipement",
            MessageId::PanelConsumables => "consommables",
            MessageId::PanelResources => "ressources",
            MessageId::PanelShop => "boutique",
            MessageId::PanelBuyGear => "acheter de l'équipement",
            MessageId::PanelSage => "le Sage (acheter des compétences)",
            MessageId::PanelFarm => "ferme",
            MessageId::PanelWorldMap => "carte du monde",
            MessageId::PanelWorldBoss => "Boss du Monde",
            MessageId::PanelPlotSoFar => "L'intrigue jusqu'ici…",
            MessageId::PanelGuilds => "guildes",
            MessageId::PanelCreateGuild => "créer une guilde",
            MessageId::PanelTutorialWelcome => "bienvenue, voyageur",
            MessageId::PanelWhileAway => "pendant votre absence",
            MessageId::PanelEndings => "fins",
            MessageId::PanelSkillsLine => "compétences",
            MessageId::PanelFormsVisited => "formes traversées",
            MessageId::PanelAchievementsLow => "succès",
            MessageId::PanelHowToPlay => "comment jouer",
            MessageId::StatName => "Nom",
            MessageId::StatForm => "Forme",
            MessageId::StatLevel => "Niveau",
            MessageId::StatXp => "XP",
            MessageId::StatHp => "PV",
            MessageId::StatAttack => "Attaque",
            MessageId::StatDefence => "Défense",
            MessageId::StatSpeed => "Vitesse",
            MessageId::StatEvasion => "Esquive",
            MessageId::ResGold => "or",
            MessageId::ResEssence => "essence",
            MessageId::ResMissions => "missions",
            MessageId::ResBossDamage => "dégâts boss",
            MessageId::ResPotions => "potions",
            MessageId::ResFireballs => "boules de feu",
            MessageId::ColSlot => "slot",
            MessageId::ColName => "nom",
            MessageId::ColDamage => "dégâts",
            MessageId::ColArea => "zone",
            MessageId::ColSeen => "vu",
            MessageId::BtnRunMission => "Lancer la mission",
            MessageId::BtnAutoOn => "auto : on",
            MessageId::BtnAutoOff => "auto : off",
            MessageId::BtnAutoEquipBest => "Auto-équiper le meilleur",
            MessageId::BtnUse => "Utiliser",
            MessageId::BtnBuy => "Acheter",
            MessageId::BtnWorkFarm => "Travailler à la ferme (+1 blé)",
            MessageId::BtnSellAllWheat => "Vendre tout le blé",
            MessageId::BtnCreate => "Créer",
            MessageId::BtnLeaveGuild => "Quitter la guilde",
            MessageId::BtnDisbandGuild => "Dissoudre la guilde",
            MessageId::BtnJoin => "Rejoindre",
            MessageId::BtnEquip => "équiper",
            MessageId::BtnNext => "Suivant",
            MessageId::BtnStartPlaying => "Commencer à jouer",
            MessageId::BtnSkipIntro => "Passer l'intro",
            MessageId::ItemPotion => "Potion",
            MessageId::ItemFireball => "Boule de feu",
            MessageId::TermYouBattle => "vous",
            MessageId::TermYouBadge => "vous",
            MessageId::TermYouLeader => "vous",
            MessageId::TermLive => "en ligne",
            MessageId::TermActive => "active",
            MessageId::TermOwned => "possédé",
            MessageId::TermMaxTier => "palier max",
            MessageId::TermEmpty => "Vide",
            MessageId::TermFormNa => "n/a (forme)",
            MessageId::TermFormLocks => "la forme verrouille ce slot",
            MessageId::TermNever => "jamais",
            MessageId::TermWin => "victoire",
            MessageId::TermDefeat => "défaite",
            MessageId::TermPubkeyHidden => "clé publique masquée (activer « avancé » pour la voir)",
            MessageId::TermPubkeyPending => "clé publique : en attente de la réponse du délégué",
            MessageId::TermPubkeyPendingShort => "clé en attente…",
            MessageId::OnbTitleWelcome => "Bienvenue sur Freenet Idle",
            MessageId::OnbBodyWelcome1 => "Votre héros, votre inventaire et votre identité vivent sur le nœud Freenet local — pas dans cet onglet. Effacer les cookies, changer de navigateur ou recharger la page ne perdra rien.",
            MessageId::OnbBodyWelcome2 => "Si le nœud est un jour reconstruit, vous pouvez sauvegarder votre seed depuis Paramètres → Sauvegarde et migration.",
            MessageId::OnbTitleLoop => "La boucle",
            MessageId::OnbBodyLoop1 => "Cliquez sur Lancer la mission dans l'onglet Ferme. Chaque mission est une chaîne de jusqu'à 5 rencontres — les victoires font tomber équipement, potions et boules de feu à des cadences fixes. Perdre contre un ennemi non-ordinaire vous transforme en lui, définitivement.",
            MessageId::OnbBodyLoop2 => "Chaque forme portée laisse une compétence permanente — la boucle de prestige.",
            MessageId::OnbTitleAuto => "Mission auto",
            MessageId::OnbBodyAuto1 => "Activez auto : on pour laisser le héros combattre tout seul. Fermez l'onglet et revenez plus tard — le délégué simule les missions manquées (jusqu'à ~1 h à la fois) et affiche un résumé au retour.",
            MessageId::OnbBodyAuto2 => "Définissez un seuil de PV pour la pause dans Paramètres si vous préférez éviter les défaites automatiques.",
            MessageId::OnbTitleTabs => "Onglets et Paramètres",
            MessageId::OnbBodyTabs1 => "🗺 Carte du monde change de biome quand vous dépassez l'actuel en niveau. 🛒 Boutique achète équipement et potions, vend la réserve, forge les doubles et échange le blé contre de l'or. ⚙ Paramètres regroupe thèmes, cadence de synchronisation, sauvegarde d'identité et options avancées.",
            MessageId::OnbBodyTabs2 => "Cliquez sur ❔ Aide à tout moment pour la référence complète.",
            MessageId::TutorialBody1 => "Cliquez sur Lancer la mission pour combattre l'ennemi de la zone. Toutes les 5 victoires un équipement tombe (à gérer dans l'onglet Boutique), toutes les 13 victoires une potion, toutes les 19 une boule de feu.",
            MessageId::TutorialBody2 => "Dégâts subis en combat ? Les PV se régénèrent avec le temps, ou utilisez une potion pour soigner instantanément. Choisissez un autre champ de bataille depuis la Carte du monde quand vous dépassez l'actuel.",
            MessageId::BattleOpeningTurn => "(tour d'ouverture — les combattants se jaugent)",
            MessageId::BattleNoEncounters => "pas encore de rencontres — lancez une mission pour combattre",
            MessageId::BattlePotionQueued => "potion en file — s'applique au prochain tour",
            MessageId::BattleFireballQueued => "boule de feu en file — s'applique au prochain tour",
            MessageId::BattleMissed => "(raté)",
            MessageId::MailboxEmpty => "(aucun message — cliquez sur le bouton ci-dessus pour faire un aller-retour de chat)",
            MessageId::MailboxKindChat => "chat",
            MessageId::MailboxKindGift => "cadeau",
            MessageId::MailboxKindGuildInvite => "invitation",
            MessageId::MailboxKindTradeOffer => "échange",
            MessageId::CatchupClearsHint => "(La bannière disparaît à la prochaine mission.)",
            MessageId::SettingsThemeDesc => "Choisissez une palette. Enregistrée dans le stockage local du navigateur ; prend effet immédiatement et persiste après rechargement.",
            MessageId::SettingsCadenceDesc => "Fréquence à laquelle la webapp parle à votre nœud local. Agressif = classement plus réactif, plus de trafic. Léger = plus discret, mais le contrat vous expulse après 60 s de silence — ne dépassez pas.",
            MessageId::SettingsAutoMissionDesc => "Met en pause la boucle auto quand les PV passent sous cette fraction de votre max. 0 % conserve l'ancien comportement — arrêt seulement à 0 PV. Des valeurs plus hautes protègent vos PV/formes/consommables d'une série de mauvais jets.",
            MessageId::SettingsPublishCheckbox => " publier immédiatement après une mission (en plus du heartbeat périodique)",
            MessageId::SettingsIdentityBody => "Exportez la seed Ed25519 pour transférer l'identité vers un autre nœud, ou effacez l'inventaire pour repartir à zéro. ",
            MessageId::SettingsIdentityBodyStrong => "Réinitialiser la progression est destructif",
            MessageId::SettingsIdentityBodyTail => " — la clé publique survit, mais tous les compteurs, objets, compétences et fins reviennent à zéro.",
            MessageId::SettingsAdvancedDesc => "Bascules trafic / confidentialité / debug. Les valeurs par défaut conviennent à la plupart des joueurs.",
            MessageId::SettingsHidePubkey => " masquer la clé publique (panneau Héros + Paramètres)",
            MessageId::SettingsHideStale => " masquer les joueurs inactifs du classement (vus il y a > 30 s)",
            MessageId::SettingsWsOverride => "Surcharge d'URL WS (vide = utiliser ?ws= ou défaut ; prend effet au rechargement) :",
            MessageId::SettingsResetUiPrefsDesc => "Efface thème + cadence + auto-pause + bascules avancées et recharge la page. Ne touche pas votre inventaire — il vit sur le nœud.",
            MessageId::SettingsWhereStateBody => "La vue locale n'est qu'un cache de ce qui vit sur le nœud. Rechargez la page — l'identité et l'inventaire reviennent du délégué. Pour réellement supprimer votre sauvegarde, effacez `~/.config/freenet/secrets/local/<delegate-key>/` sur le nœud.",
            MessageId::SettingsSeedRevealWarn => "Copiez-la une seule fois. Quiconque détient ces octets peut se faire passer pour vous sur le contrat.",
            MessageId::GuildsPanelDesc => "Groupes coopératifs — ossature précoce. Créez-en un, les autres peuvent rejoindre par id. Chaque joueur est dans une seule guilde au plus ; les chefs se transmettent automatiquement au départ.",
            MessageId::GuildsContractMissing => "Contrat Guildes non configuré. Publiez ",
            MessageId::GuildsContractMissingTail => " (extension WIP) ou surchargez les clés dans ",
            MessageId::GuildsEmptyList => "(pas encore de guildes — soyez le premier)",
            MessageId::GuildsViaScript => " via ",
            MessageId::GuildNamePlaceholder => "nom de la guilde (≤ 32 octets)",
            MessageId::MailboxNotConfiguredHead => "Contrat Boîte non configuré. Publiez ",
            MessageId::MailboxNotConfiguredVia => " via ",
            MessageId::MailboxNotConfiguredTail => " (extension WIP) ou définissez ",
            MessageId::MailboxNotConfiguredIn => " dans ",
            MessageId::ShopStashDesc => "objets groupés par slot — équipez pour porter, revendez au marchand pour de l'or au tarif du palier",
            MessageId::ShopBuyGearDesc => "équipement préfabriqué à la forge. Chaque clic d'Acheter ajoute une pièce du slot+palier demandé à votre réserve. Légendaire (T4) uniquement par forge ou drop.",
            MessageId::ShopSageDesc => "le Sage échange du savoir permanent contre de l'essence. Vétéran/Champion viennent encore des paliers de niveau — non vendables.",
            MessageId::ShopFarmDesc => "revenu sûr hors combat. Chaque clic de Travailler donne +1 blé ; le marchand paie 1 or pour 10 blé.",
            MessageId::ShopFarmDescPassive => "les Ouvriers de votre Domaine produisent maintenant du blé passivement — le marchand l'achète toujours à 1 or pour 10.",
            MessageId::TipFightInProgress => "combat en cours — attendez la fin de la bataille",
            MessageId::TipAutoToggleMidFight => "la bascule auto fonctionne aussi en combat — le nouveau réglage s'applique après la fin de la bataille",
            MessageId::TipAutoEquipBest => "parcourir chaque slot et équiper la pièce avec la plus haute somme de stats que vous possédez",
            MessageId::TipAutoEquipNothing => "rien dans la réserve ne bat ce que vous portez — récupérez du butin ou changez de forme",
            MessageId::TipEstateBlocksCombat => "le Domaine est actif — mettez-le en pause depuis le panneau Domaine pour combattre",
            MessageId::TipPotionQueue => "file : soin complet au prochain tour",
            MessageId::TipPotionIdle => "soigne les PV à fond",
            MessageId::TipFireballQueue => "file : dégâts bonus au prochain tour",
            MessageId::TipUnequipSlot => "retirer — renvoyer en réserve",
            MessageId::TipDisbandLeader => "chef uniquement : supprimer la guilde pour tous",
            MessageId::PotionShopDesc => "soigne intégralement vos PV",
            MessageId::TermCorrupt => "(corrompu)",
            MessageId::HelpTheLoop => "la boucle",
            MessageId::HelpStats => "stats",
            MessageId::HelpFormsTransformation => "formes et transformations",
            MessageId::HelpTabs => "onglets",
            MessageId::HelpShopGear => "boutique et équipement",
            MessageId::HelpConsumables => "consommables",
            MessageId::HelpWorldBoss => "boss du monde",
            MessageId::HelpDelegateWhat => "que fait le délégué ?",
            MessageId::HelpGuildsMailbox => "guildes et boîte (précoce)",
            MessageId::HelpEstate => "domaine — boucle de revenu passif",
            MessageId::HelpLegacy => "héritage — prestige personnel",
            MessageId::HelpAreaGraph => "carte du monde — disposition en graphe",
            MessageId::PanelEstate => "Domaine",
            MessageId::EstateBtnPause => "Mettre le Domaine en pause",
            MessageId::EstateBtnRun => "Lancer le Domaine",
            MessageId::EstateColTier => "Palier",
            MessageId::EstateColOwned => "Embauchés",
            MessageId::EstateColYield => "Rendement/s",
            MessageId::EstateColNextPrice => "Prochain prix",
            MessageId::BtnHire => "Embaucher",
            MessageId::EstateResWheat => "blé",
            MessageId::EstateResGold => "or",
            MessageId::EstateResEssence => "essence",
            MessageId::PanelLegacy => "Héritage",
            MessageId::LegacyColNode => "Nœud",
            MessageId::LegacyColLevel => "Niveau",
            MessageId::LegacyColMultiplier => "Multiplicateur",
            MessageId::LegacyColNextCost => "Coût suivant",
            MessageId::BtnAscend => "Ascension",
            MessageId::LegacyAscendBlurb => "Soft-reset : garde les étoiles, le niveau, les missions, les compétences. Efface or, équipement, Domaine.",
            MessageId::LegacyAscendConfirm => "Ascension — partie en soft-reset ? Garde étoiles, niveau, compteur de missions et compétences. Efface or, équipement et Domaine.",
            MessageId::CatchupModalTitle => "Bon retour",
            MessageId::BtnGotIt => "Compris",
            MessageId::NewerBuildDesc => "Une build plus récente est en ligne — pas de changelog détaillé cette fois.",
            MessageId::PanelFormsShop => "Formes",
            MessageId::FormsShopDesc => "Réinitialisez votre apparence (Humain est peu cher) ou investissez de l'or dans l'une des quatre autres formes. Chaque forme pilote aussi l'affinité du Domaine — Cheval favorise Ouvrier + Cueilleur, Dragon favorise Marchand + Sage, Chat favorise Cueilleur + Sage, Slime offre un +30 % uniforme, Humain est neutre. L'achat direct reflète une transformation par défaite : la forme est ajoutée à votre set visité et sa compétence s'ouvre chez le Sage.",
            MessageId::FormsShopBaselineDesc => "base équilibrée — pas de bonus de stats",
            MessageId::TipFormAlreadyActive => "vous êtes déjà dans cette forme",
            MessageId::PanelActivities => "Activités",
            MessageId::ActivitiesDesc => "Actions non-combat liées à la zone actuelle. En choisir une la définit comme votre action passive — mission auto et Domaine se mettent en pause jusqu'à l'arrêt.",
            MessageId::ActivityStart => "Démarrer",
            MessageId::ActivityStop => "Arrêter",
            MessageId::PanelRoutine => "Routine",
            MessageId::RoutineDesc => "Embauche automatiquement les travailleurs du Domaine jusqu'à la cible quand l'or le permet. Limitée à 50 embauches par tick de délégué pour qu'une grosse fenêtre de rattrapage ne vide pas le trésor.",
            MessageId::RoutineColTier => "Palier",
            MessageId::RoutineColCurrent => "Embauchés",
            MessageId::RoutineColTarget => "Cible",
            MessageId::PanelInsight => "Perspicacité",
            MessageId::InsightDesc => "Monnaie rare. Gagnée toutes les 25 missions et par l'activité Astrale « Décoder les sigils ». Dépensée en petits bonus permanents.",
            MessageId::InsightColNode => "Nœud",
            MessageId::InsightColLevel => "Niveau",
            MessageId::InsightColNextCost => "Coût suivant",
            MessageId::PanelBossAttack => "Attaque personnelle du Boss",
            MessageId::BossAttackBtn => "Attaquer (-200 essence, +50 dég. boss)",
            MessageId::BossAttackDesc => "Dépensez de l'essence pour entamer le Boss du Monde hors combat. Débloqué à mission_count ≥ 100, niveau ≥ 10 et au moins un travailleur du Domaine.",
            MessageId::BossAttackLocked => "Verrouillé — il faut 100 missions, niveau 10 et au moins un travailleur du Domaine.",
            MessageId::PanelTokens => "Jetons",
            MessageId::TokensDesc => "Un jeton tous les 500 de dégâts personnels au Boss. Dépensés en perks cosmétiques aujourd'hui ; les perks gameplay (slot d'équipement, second preset de mission auto) s'activent à mesure que leur plomberie arrive.",
            MessageId::TokenColPerk => "Perk",
            MessageId::TokenColPrice => "Prix",
            MessageId::BtnUnlock => "Débloquer",
            MessageId::ResInsight => "perspicacité",
            MessageId::ResTokens => "jetons",
            MessageId::MasteryIntro => "Améliorations permanentes — achetées une fois, gardées pour toujours. Les étoiles d'Héritage viennent des paliers de niveau ; la Perspicacité des missions et activités Astrales ; les Jetons des dégâts personnels au Boss. Les cibles de Routine et l'Attaque du Boss du Monde vivent aussi ici.",
            MessageId::PanelWilds => "Terres sauvages",
            MessageId::WildsDesc => "Carte alternative de fin de jeu, générée procéduralement depuis votre seed d'intrigue. Noms et bruit sur les stats d'ennemis sont uniques pour vous ; la topologie est fixe (8 nœuds, deux branches depuis l'entrée, un nœud de confluence). Aucune contribution au Boss du Monde.",
            MessageId::MapViewLinear => "Linéaire",
            MessageId::MapViewWilds => "Terres sauvages",
        })
    }

    /// Spanish overrides. Exhaustive coverage.
    fn tr_es(msg: MessageId) -> Option<&'static str> {
        Some(match msg {
            MessageId::BootLoading => "Cargando…",
            MessageId::StatusAskingDelegate => "pidiendo identidad al delegado…",
            MessageId::StatusRegisteringDelegate => "registrando delegado…",
            MessageId::StatusSubscribing => "suscribiendo…",
            MessageId::TabFarm => "Granja",
            MessageId::TabWorldMap => "Mapa del mundo",
            MessageId::TabShop => "Tienda",
            MessageId::TabGuilds => "Gremios",
            MessageId::TabAchievements => "Logros",
            MessageId::TabMastery => "Maestría",
            MessageId::TabSettings => "Ajustes",
            MessageId::TabHelp => "Ayuda",
            MessageId::PillDefeated => "DERROTADO",
            MessageId::PillAdventuring => "EN AVENTURA",
            MessageId::PillFocusing => "CONCENTRADO",
            MessageId::PillRecovering => "RECUPERANDO",
            MessageId::PillReady => "LISTO",
            MessageId::PillEstate => "FINCA",
            MessageId::SettingsTitle => "ajustes",
            MessageId::SettingsTheme => "tema",
            MessageId::SettingsLanguage => "idioma",
            MessageId::SettingsSyncCadence => "cadencia de sincronización",
            MessageId::SettingsAutoMission => "misión auto",
            MessageId::SettingsPublishBehavior => "comportamiento de publicación",
            MessageId::SettingsIdentityBackup => "identidad y copia",
            MessageId::SettingsAdvanced => "avanzado",
            MessageId::SettingsResetUiPrefs => "restablecer preferencias de UI",
            MessageId::SettingsMailbox => "buzón (test D2D)",
            MessageId::SettingsWhereStateLives => "dónde vive el estado",
            MessageId::LocaleEnglish => "English",
            MessageId::LocaleRussian => "Русский",
            MessageId::BtnExportSeed => "Exportar seed",
            MessageId::BtnResetProgress => "Restablecer progreso",
            MessageId::BtnHide => "Ocultar",
            MessageId::BtnResetDefaults => "Restablecer por defecto",
            MessageId::BtnSendTestSelf => "Enviarme un mensaje de prueba",
            MessageId::SourceLink => "fuente ↗",
            MessageId::PanelHero => "héroe",
            MessageId::PanelEquipment => "equipo",
            MessageId::PanelConsumables => "consumibles",
            MessageId::PanelResources => "recursos",
            MessageId::PanelShop => "tienda",
            MessageId::PanelBuyGear => "comprar equipo",
            MessageId::PanelSage => "el Sabio (comprar habilidades)",
            MessageId::PanelFarm => "granja",
            MessageId::PanelWorldMap => "mapa del mundo",
            MessageId::PanelWorldBoss => "Jefe del Mundo",
            MessageId::PanelPlotSoFar => "La trama hasta ahora…",
            MessageId::PanelGuilds => "gremios",
            MessageId::PanelCreateGuild => "crear un gremio",
            MessageId::PanelTutorialWelcome => "bienvenido, viajero",
            MessageId::PanelWhileAway => "mientras estabas fuera",
            MessageId::PanelEndings => "finales",
            MessageId::PanelSkillsLine => "habilidades",
            MessageId::PanelFormsVisited => "formas visitadas",
            MessageId::PanelAchievementsLow => "logros",
            MessageId::PanelHowToPlay => "cómo jugar",
            MessageId::StatName => "Nombre",
            MessageId::StatForm => "Forma",
            MessageId::StatLevel => "Nivel",
            MessageId::StatXp => "XP",
            MessageId::StatHp => "PV",
            MessageId::StatAttack => "Ataque",
            MessageId::StatDefence => "Defensa",
            MessageId::StatSpeed => "Velocidad",
            MessageId::StatEvasion => "Evasión",
            MessageId::ResGold => "oro",
            MessageId::ResEssence => "esencia",
            MessageId::ResMissions => "misiones",
            MessageId::ResBossDamage => "daño al jefe",
            MessageId::ResPotions => "pociones",
            MessageId::ResFireballs => "bolas de fuego",
            MessageId::ColSlot => "slot",
            MessageId::ColName => "nombre",
            MessageId::ColDamage => "daño",
            MessageId::ColArea => "zona",
            MessageId::ColSeen => "visto",
            MessageId::BtnRunMission => "Lanzar misión",
            MessageId::BtnAutoOn => "auto: on",
            MessageId::BtnAutoOff => "auto: off",
            MessageId::BtnAutoEquipBest => "Auto-equipar lo mejor",
            MessageId::BtnUse => "Usar",
            MessageId::BtnBuy => "Comprar",
            MessageId::BtnWorkFarm => "Trabajar la granja (+1 trigo)",
            MessageId::BtnSellAllWheat => "Vender todo el trigo",
            MessageId::BtnCreate => "Crear",
            MessageId::BtnLeaveGuild => "Salir del gremio",
            MessageId::BtnDisbandGuild => "Disolver gremio",
            MessageId::BtnJoin => "Unirse",
            MessageId::BtnEquip => "equipar",
            MessageId::BtnNext => "Siguiente",
            MessageId::BtnStartPlaying => "Empezar a jugar",
            MessageId::BtnSkipIntro => "Saltar intro",
            MessageId::ItemPotion => "Poción",
            MessageId::ItemFireball => "Bola de fuego",
            MessageId::TermYouBattle => "tú",
            MessageId::TermYouBadge => "tú",
            MessageId::TermYouLeader => "tú",
            MessageId::TermLive => "en línea",
            MessageId::TermActive => "activa",
            MessageId::TermOwned => "poseído",
            MessageId::TermMaxTier => "grado máx.",
            MessageId::TermEmpty => "Vacío",
            MessageId::TermFormNa => "n/d (forma)",
            MessageId::TermFormLocks => "la forma bloquea este slot",
            MessageId::TermNever => "nunca",
            MessageId::TermWin => "victoria",
            MessageId::TermDefeat => "derrota",
            MessageId::TermPubkeyHidden => "clave pública oculta (activa «avanzado» para verla)",
            MessageId::TermPubkeyPending => "clave pública: esperando respuesta del delegado",
            MessageId::TermPubkeyPendingShort => "clave pendiente…",
            MessageId::OnbTitleWelcome => "Bienvenido a Freenet Idle",
            MessageId::OnbBodyWelcome1 => "Tu héroe, inventario e identidad viven en el nodo local de Freenet — no en esta pestaña. Borrar cookies, cambiar de navegador o recargar la página no pierde nada.",
            MessageId::OnbBodyWelcome2 => "Si el nodo se reconstruye, puedes respaldar tu seed desde Ajustes → Copia y migración.",
            MessageId::OnbTitleLoop => "El bucle",
            MessageId::OnbBodyLoop1 => "Pulsa Lanzar misión en la pestaña Granja. Cada misión es una cadena de hasta 5 encuentros — las victorias sueltan equipo, pociones y bolas de fuego con cadencias fijas. Perder ante un enemigo no-mundano te transforma en él, permanentemente.",
            MessageId::OnbBodyLoop2 => "Cada forma que llevas deja una habilidad permanente — el bucle de prestigio.",
            MessageId::OnbTitleAuto => "Misión auto",
            MessageId::OnbBodyAuto1 => "Activa auto: on para que el héroe luche solo. Cierra la pestaña y vuelve más tarde — el delegado simula las misiones perdidas (hasta ~1 h por vez) y muestra un resumen al volver.",
            MessageId::OnbBodyAuto2 => "Define un umbral de PV para la pausa en Ajustes si prefieres evitar derrotas automáticas.",
            MessageId::OnbTitleTabs => "Pestañas y Ajustes",
            MessageId::OnbBodyTabs1 => "🗺 Mapa del mundo cambia de bioma cuando superas el actual en nivel. 🛒 Tienda compra equipo y pociones, vende el alijo, forja duplicados y cambia trigo por oro. ⚙ Ajustes agrupa temas, cadencia de sincronización, copia de identidad y opciones avanzadas.",
            MessageId::OnbBodyTabs2 => "Pulsa ❔ Ayuda en cualquier momento para la referencia completa.",
            MessageId::TutorialBody1 => "Pulsa Lanzar misión para luchar contra el enemigo de la zona. Cada 5 victorias cae equipo (gestiona en la pestaña Tienda), cada 13 victorias una poción, cada 19 una bola de fuego.",
            MessageId::TutorialBody2 => "¿Daño en combate? Los PV se regeneran con el tiempo, o usa una poción para curarte al instante. Escoge otro campo de batalla en el Mapa del mundo cuando superes el actual.",
            MessageId::BattleOpeningTurn => "(turno inicial — los combatientes se estudian)",
            MessageId::BattleNoEncounters => "todavía no hay encuentros — Lanzar misión para combatir",
            MessageId::BattlePotionQueued => "poción encolada — se aplica en el próximo turno",
            MessageId::BattleFireballQueued => "bola de fuego encolada — se aplica en el próximo turno",
            MessageId::BattleMissed => "(fallado)",
            MessageId::MailboxEmpty => "(sin mensajes — pulsa el botón de arriba para hacer un ciclo de chat)",
            MessageId::MailboxKindChat => "chat",
            MessageId::MailboxKindGift => "regalo",
            MessageId::MailboxKindGuildInvite => "invitación",
            MessageId::MailboxKindTradeOffer => "intercambio",
            MessageId::CatchupClearsHint => "(El banner desaparece al lanzar una misión.)",
            MessageId::SettingsThemeDesc => "Elige una paleta. Se guarda en el almacenamiento local del navegador; se aplica al momento y persiste tras recargar.",
            MessageId::SettingsCadenceDesc => "Con qué frecuencia la webapp habla con tu nodo local. Agresivo = clasificación más viva, más tráfico al nodo. Suave = más ligero, pero el contrato te poda tras 60 s de silencio — no te pases.",
            MessageId::SettingsAutoMissionDesc => "Pausa el bucle auto cuando los PV bajan de esta fracción de tu máximo. 0% mantiene el comportamiento antiguo — solo parar a 0 PV. Valores más altos te salvan de perder PV/formas/consumibles por una mala racha.",
            MessageId::SettingsPublishCheckbox => " publicar inmediatamente tras una misión (además del heartbeat periódico)",
            MessageId::SettingsIdentityBody => "Exporta la seed Ed25519 para mover la identidad a otro nodo, o borra el inventario al estado de personaje nuevo. ",
            MessageId::SettingsIdentityBodyStrong => "Restablecer progreso es destructivo",
            MessageId::SettingsIdentityBodyTail => " — la clave pública sobrevive, pero todos los contadores, objetos, habilidades y finales vuelven a cero.",
            MessageId::SettingsAdvancedDesc => "Interruptores de tráfico / privacidad / debug. Los valores por defecto son adecuados para la mayoría.",
            MessageId::SettingsHidePubkey => " ocultar clave pública (panel Héroe + Ajustes)",
            MessageId::SettingsHideStale => " ocultar jugadores inactivos del marcador (visto hace > 30 s)",
            MessageId::SettingsWsOverride => "Sobreescritura de URL WS (vacío = usar ?ws= o por defecto; aplica al recargar):",
            MessageId::SettingsResetUiPrefsDesc => "Limpia tema + cadencia + auto-pausa + interruptores avanzados y recarga. No toca tu inventario — vive en el nodo.",
            MessageId::SettingsWhereStateBody => "La vista local es solo una caché de lo que vive en el nodo. Recarga la página — identidad e inventario vuelven del delegado. Para borrar realmente tu partida, limpia `~/.config/freenet/secrets/local/<delegate-key>/` en el nodo.",
            MessageId::SettingsSeedRevealWarn => "Cópiala solo una vez. Cualquiera con estos bytes puede suplantarte en el contrato.",
            MessageId::GuildsPanelDesc => "Grupos cooperativos — andamiaje temprano. Crea uno, otros pueden unirse por id. Cada jugador en como máximo un gremio; el liderazgo se traspasa automáticamente al salir.",
            MessageId::GuildsContractMissing => "Contrato de Gremios no configurado. Publica ",
            MessageId::GuildsContractMissingTail => " (extensión WIP) o anula las claves en ",
            MessageId::GuildsEmptyList => "(no hay gremios todavía — sé el primero)",
            MessageId::GuildsViaScript => " mediante ",
            MessageId::GuildNamePlaceholder => "nombre del gremio (≤ 32 bytes)",
            MessageId::MailboxNotConfiguredHead => "Contrato de Buzón no configurado. Publica ",
            MessageId::MailboxNotConfiguredVia => " mediante ",
            MessageId::MailboxNotConfiguredTail => " (extensión WIP) o define ",
            MessageId::MailboxNotConfiguredIn => " en ",
            MessageId::ShopStashDesc => "objetos agrupados por slot — equipa para llevar, vende al mercader por oro al precio del grado",
            MessageId::ShopBuyGearDesc => "equipo prefabricado en la herrería. Cada clic de Comprar añade una pieza del slot+grado solicitado a tu alijo. Legendario (T4) solo por forja o drop.",
            MessageId::ShopSageDesc => "el Sabio cambia conocimiento permanente por esencia. Veterano/Campeón siguen viniendo de hitos de nivel — no se venden.",
            MessageId::ShopFarmDesc => "ingreso seguro sin combate. Cada clic de Trabajar da +1 trigo; el mercader paga 1 oro por 10 trigos.",
            MessageId::ShopFarmDescPassive => "tus Braceros de la Finca ahora producen trigo pasivamente — el mercader sigue comprándolo a 1 oro por 10.",
            MessageId::TipFightInProgress => "combate en curso — espera a que termine la pelea actual",
            MessageId::TipAutoToggleMidFight => "el interruptor auto funciona también en combate — el nuevo valor aplica al terminar la pelea actual",
            MessageId::TipAutoEquipBest => "recorrer cada slot y equipar la pieza con la mayor suma de stats que tengas",
            MessageId::TipAutoEquipNothing => "nada en el alijo supera lo que ya llevas — consigue más botín o cambia de forma",
            MessageId::TipEstateBlocksCombat => "la Finca está activa — pausala desde el panel de la Finca para luchar",
            MessageId::TipPotionQueue => "cola: cura completa en el próximo turno",
            MessageId::TipPotionIdle => "cura los PV al máximo",
            MessageId::TipFireballQueue => "cola: daño extra en el próximo turno",
            MessageId::TipUnequipSlot => "quitar — enviar al alijo",
            MessageId::TipDisbandLeader => "solo líder: eliminar el gremio para todos",
            MessageId::PotionShopDesc => "cura tus PV al máximo",
            MessageId::TermCorrupt => "(corrupto)",
            MessageId::HelpTheLoop => "el bucle",
            MessageId::HelpStats => "stats",
            MessageId::HelpFormsTransformation => "formas y transformaciones",
            MessageId::HelpTabs => "pestañas",
            MessageId::HelpShopGear => "tienda y equipo",
            MessageId::HelpConsumables => "consumibles",
            MessageId::HelpWorldBoss => "jefe del mundo",
            MessageId::HelpDelegateWhat => "¿qué hace el delegado?",
            MessageId::HelpGuildsMailbox => "gremios y buzón (temprano)",
            MessageId::HelpEstate => "finca — bucle de ingresos pasivos",
            MessageId::HelpLegacy => "legado — prestigio personal",
            MessageId::HelpAreaGraph => "mapa del mundo — disposición en grafo",
            MessageId::PanelEstate => "Finca",
            MessageId::EstateBtnPause => "Pausar Finca",
            MessageId::EstateBtnRun => "Ejecutar Finca",
            MessageId::EstateColTier => "Grado",
            MessageId::EstateColOwned => "Contratados",
            MessageId::EstateColYield => "Rend./s",
            MessageId::EstateColNextPrice => "Próximo precio",
            MessageId::BtnHire => "Contratar",
            MessageId::EstateResWheat => "trigo",
            MessageId::EstateResGold => "oro",
            MessageId::EstateResEssence => "esencia",
            MessageId::PanelLegacy => "Legado",
            MessageId::LegacyColNode => "Nodo",
            MessageId::LegacyColLevel => "Nivel",
            MessageId::LegacyColMultiplier => "Multiplicador",
            MessageId::LegacyColNextCost => "Próximo coste",
            MessageId::BtnAscend => "Ascender",
            MessageId::LegacyAscendBlurb => "Soft-reset: mantiene estrellas, nivel, misiones, habilidades. Borra oro, equipo, Finca.",
            MessageId::LegacyAscendConfirm => "Ascender — ¿partida con soft-reset? Mantiene estrellas, nivel, contador de misiones y habilidades. Borra oro, equipo y Finca.",
            MessageId::CatchupModalTitle => "Bienvenido de vuelta",
            MessageId::BtnGotIt => "Entendido",
            MessageId::NewerBuildDesc => "Una build más reciente está en marcha — no hay changelog detallado esta vez.",
            MessageId::PanelFormsShop => "Formas",
            MessageId::FormsShopDesc => "Restablece tu forma (Humano es barato) o invierte oro en una de las otras cuatro. Cada forma rige la afinidad de la Finca — Caballo potencia Bracero + Recolector, Dragón potencia Mercader + Sabio, Gato potencia Recolector + Sabio, Limo da un +30% plano, Humano es neutral. La compra directa imita una transformación por derrota: la forma se añade a tu conjunto visitado y su habilidad se abre con el Sabio.",
            MessageId::FormsShopBaselineDesc => "base equilibrada — sin paquete de stats",
            MessageId::TipFormAlreadyActive => "ya estás en esta forma",
            MessageId::PanelActivities => "Actividades",
            MessageId::ActivitiesDesc => "Acciones no-combate ligadas a la zona actual. Elegir una la fija como tu acción pasiva — misión auto y Finca pausan hasta detenerla.",
            MessageId::ActivityStart => "Iniciar",
            MessageId::ActivityStop => "Detener",
            MessageId::PanelRoutine => "Rutina",
            MessageId::RoutineDesc => "Contrata automáticamente trabajadores de la Finca hasta el objetivo cuando el oro lo permita. Limitada a 50 contrataciones por tick de delegado para que una ventana grande de catch-up no vacíe la tesorería.",
            MessageId::RoutineColTier => "Grado",
            MessageId::RoutineColCurrent => "Contratados",
            MessageId::RoutineColTarget => "Objetivo",
            MessageId::PanelInsight => "Perspicacia",
            MessageId::InsightDesc => "Moneda rara. Ganada cada 25 misiones y por la actividad Astral «Descifrar sigilos». Se gasta en pequeños bonos permanentes.",
            MessageId::InsightColNode => "Nodo",
            MessageId::InsightColLevel => "Nivel",
            MessageId::InsightColNextCost => "Próximo coste",
            MessageId::PanelBossAttack => "Ataque personal al Jefe",
            MessageId::BossAttackBtn => "Atacar (-200 esencia, +50 daño jefe)",
            MessageId::BossAttackDesc => "Gasta esencia para mellar al Jefe del Mundo fuera de combate. Se desbloquea a misiones ≥ 100, nivel ≥ 10 y al menos un trabajador de la Finca.",
            MessageId::BossAttackLocked => "Bloqueado — hacen falta 100 misiones, nivel 10 y al menos un trabajador de la Finca.",
            MessageId::PanelTokens => "Fichas",
            MessageId::TokensDesc => "Una ficha cada 500 de daño personal al Jefe. Se gastan hoy en perks cosméticos; los perks de gameplay (slot de equipo, segundo preset de misión auto) llegarán cuando esté lista la fontanería.",
            MessageId::TokenColPerk => "Perk",
            MessageId::TokenColPrice => "Precio",
            MessageId::BtnUnlock => "Desbloquear",
            MessageId::ResInsight => "perspicacia",
            MessageId::ResTokens => "fichas",
            MessageId::MasteryIntro => "Mejoras permanentes — se compran una vez, se quedan para siempre. Las estrellas de Legado vienen de hitos de nivel; la Perspicacia de misiones y actividades Astrales; las Fichas del daño personal al Jefe. Los objetivos de Rutina y el Ataque al Jefe del Mundo también viven aquí.",
            MessageId::PanelWilds => "Tierras salvajes",
            MessageId::WildsDesc => "Mapa alternativo de fin de juego, generado proceduralmente desde tu seed de trama. Nombres y ruido de stats de enemigos son únicos para ti; la topología es fija (8 nodos, dos ramas desde la entrada, un nodo de confluencia). Sin contribución al Jefe del Mundo.",
            MessageId::MapViewLinear => "Lineal",
            MessageId::MapViewWilds => "Tierras salvajes",
        })
    }

    /// Japanese overrides. Exhaustive coverage.
    fn tr_ja(msg: MessageId) -> Option<&'static str> {
        Some(match msg {
            MessageId::BootLoading => "読み込み中…",
            MessageId::StatusAskingDelegate => "デリゲートに身元を要求中…",
            MessageId::StatusRegisteringDelegate => "デリゲートを登録中…",
            MessageId::StatusSubscribing => "購読中…",
            MessageId::TabFarm => "農場",
            MessageId::TabWorldMap => "ワールドマップ",
            MessageId::TabShop => "ショップ",
            MessageId::TabGuilds => "ギルド",
            MessageId::TabAchievements => "実績",
            MessageId::TabMastery => "熟練",
            MessageId::TabSettings => "設定",
            MessageId::TabHelp => "ヘルプ",
            MessageId::PillDefeated => "敗北",
            MessageId::PillAdventuring => "冒険中",
            MessageId::PillFocusing => "集中中",
            MessageId::PillRecovering => "回復中",
            MessageId::PillReady => "準備完了",
            MessageId::PillEstate => "領地",
            MessageId::SettingsTitle => "設定",
            MessageId::SettingsTheme => "テーマ",
            MessageId::SettingsLanguage => "言語",
            MessageId::SettingsSyncCadence => "同期頻度",
            MessageId::SettingsAutoMission => "自動ミッション",
            MessageId::SettingsPublishBehavior => "公開挙動",
            MessageId::SettingsIdentityBackup => "ID とバックアップ",
            MessageId::SettingsAdvanced => "詳細",
            MessageId::SettingsResetUiPrefs => "UI 設定をリセット",
            MessageId::SettingsMailbox => "メール (D2D テスト)",
            MessageId::SettingsWhereStateLives => "状態の所在",
            MessageId::LocaleEnglish => "English",
            MessageId::LocaleRussian => "Русский",
            MessageId::BtnExportSeed => "シードを書き出す",
            MessageId::BtnResetProgress => "進行リセット",
            MessageId::BtnHide => "隠す",
            MessageId::BtnResetDefaults => "デフォルトに戻す",
            MessageId::BtnSendTestSelf => "自分にテストメッセージ送信",
            MessageId::SourceLink => "ソース ↗",
            MessageId::PanelHero => "英雄",
            MessageId::PanelEquipment => "装備",
            MessageId::PanelConsumables => "消耗品",
            MessageId::PanelResources => "資源",
            MessageId::PanelShop => "ショップ",
            MessageId::PanelBuyGear => "装備を買う",
            MessageId::PanelSage => "賢者（スキル購入）",
            MessageId::PanelFarm => "農場",
            MessageId::PanelWorldMap => "ワールドマップ",
            MessageId::PanelWorldBoss => "ワールドボス",
            MessageId::PanelPlotSoFar => "ここまでの物語…",
            MessageId::PanelGuilds => "ギルド",
            MessageId::PanelCreateGuild => "ギルドを作成",
            MessageId::PanelTutorialWelcome => "ようこそ、放浪者よ",
            MessageId::PanelWhileAway => "離席中の出来事",
            MessageId::PanelEndings => "エンディング",
            MessageId::PanelSkillsLine => "スキル",
            MessageId::PanelFormsVisited => "訪れたフォーム",
            MessageId::PanelAchievementsLow => "実績",
            MessageId::PanelHowToPlay => "遊び方",
            MessageId::StatName => "名前",
            MessageId::StatForm => "フォーム",
            MessageId::StatLevel => "レベル",
            MessageId::StatXp => "XP",
            MessageId::StatHp => "HP",
            MessageId::StatAttack => "攻撃",
            MessageId::StatDefence => "防御",
            MessageId::StatSpeed => "素早さ",
            MessageId::StatEvasion => "回避",
            MessageId::ResGold => "金",
            MessageId::ResEssence => "精",
            MessageId::ResMissions => "ミッション",
            MessageId::ResBossDamage => "ボスダメージ",
            MessageId::ResPotions => "ポーション",
            MessageId::ResFireballs => "ファイアボール",
            MessageId::ColSlot => "スロット",
            MessageId::ColName => "名前",
            MessageId::ColDamage => "ダメージ",
            MessageId::ColArea => "エリア",
            MessageId::ColSeen => "確認",
            MessageId::BtnRunMission => "ミッション開始",
            MessageId::BtnAutoOn => "自動: オン",
            MessageId::BtnAutoOff => "自動: オフ",
            MessageId::BtnAutoEquipBest => "最良を自動装備",
            MessageId::BtnUse => "使用",
            MessageId::BtnBuy => "購入",
            MessageId::BtnWorkFarm => "農場で働く (+1 小麦)",
            MessageId::BtnSellAllWheat => "小麦を全部売る",
            MessageId::BtnCreate => "作成",
            MessageId::BtnLeaveGuild => "ギルドを抜ける",
            MessageId::BtnDisbandGuild => "ギルドを解散",
            MessageId::BtnJoin => "参加",
            MessageId::BtnEquip => "装備",
            MessageId::BtnNext => "次へ",
            MessageId::BtnStartPlaying => "ゲーム開始",
            MessageId::BtnSkipIntro => "イントロをスキップ",
            MessageId::ItemPotion => "ポーション",
            MessageId::ItemFireball => "ファイアボール",
            MessageId::TermYouBattle => "あなた",
            MessageId::TermYouBadge => "あなた",
            MessageId::TermYouLeader => "あなた",
            MessageId::TermLive => "オンライン",
            MessageId::TermActive => "アクティブ",
            MessageId::TermOwned => "所有",
            MessageId::TermMaxTier => "最高ティア",
            MessageId::TermEmpty => "空",
            MessageId::TermFormNa => "不可（フォーム）",
            MessageId::TermFormLocks => "フォームがこのスロットを封じています",
            MessageId::TermNever => "なし",
            MessageId::TermWin => "勝利",
            MessageId::TermDefeat => "敗北",
            MessageId::TermPubkeyHidden => "公開鍵は非表示（詳細から表示可能）",
            MessageId::TermPubkeyPending => "公開鍵: デリゲートの応答待ち",
            MessageId::TermPubkeyPendingShort => "公開鍵を取得中…",
            MessageId::OnbTitleWelcome => "Freenet Idle へようこそ",
            MessageId::OnbBodyWelcome1 => "英雄、インベントリ、ID はすべてローカルの Freenet ノードに存在し、このブラウザタブにはありません。Cookie の削除、ブラウザの変更、ページ再読込でも何も失われません。",
            MessageId::OnbBodyWelcome2 => "ノードを再構築する場合は、設定 → バックアップと移行 からシードを保存してください。",
            MessageId::OnbTitleLoop => "ゲームの流れ",
            MessageId::OnbBodyLoop1 => "ファームタブの「ミッション開始」をクリック。各ミッションは最大 5 戦の連戦で、勝利すると装備、ポーション、ファイアボールが固定頻度でドロップします。通常でない敵に負けると、その敵に永続的に変身します。",
            MessageId::OnbBodyLoop2 => "経験したフォームごとに恒久スキルが残ります — これがプレステージのループです。",
            MessageId::OnbTitleAuto => "自動ミッション",
            MessageId::OnbBodyAuto1 => "「自動: オン」にすると、英雄が自分で戦い続けます。タブを閉じて後で戻ると、デリゲートが逃したミッションを（一度に最大約 1 時間）シミュレートして要約を表示します。",
            MessageId::OnbBodyAuto2 => "自動敗北を避けたい場合は、設定で HP 一時停止しきい値を指定してください。",
            MessageId::OnbTitleTabs => "タブと設定",
            MessageId::OnbBodyTabs1 => "🗺 ワールドマップは現エリアを超えたら別の生物群系へ。🛒 ショップは装備とポーションの購入、ストック売却、重複の鍛造、小麦から金への両替。⚙ 設定はテーマ、同期頻度、ID バックアップ、詳細トグル。",
            MessageId::OnbBodyTabs2 => "詳細リファレンスは ❔ ヘルプ をいつでも参照できます。",
            MessageId::TutorialBody1 => "「ミッション開始」をクリックしてエリアの敵と戦いましょう。5 勝ごとに装備（ショップタブで管理）、13 勝ごとにポーション、19 勝ごとにファイアボールがドロップします。",
            MessageId::TutorialBody2 => "戦闘でダメージを受けた？ HP は時間で回復するか、ポーションで即時回復可能。現エリアを超えたら、ワールドマップから別の戦場を選んでください。",
            MessageId::BattleOpeningTurn => "（開始ターン — 戦闘者は互いを見定めています）",
            MessageId::BattleNoEncounters => "戦闘履歴なし — 「ミッション開始」で戦いましょう",
            MessageId::BattlePotionQueued => "ポーションをキュー — 次のターンに適用",
            MessageId::BattleFireballQueued => "ファイアボールをキュー — 次のターンに適用",
            MessageId::BattleMissed => "（ミス）",
            MessageId::MailboxEmpty => "（メッセージなし — 上のボタンでチャットの往復をテストできます）",
            MessageId::MailboxKindChat => "チャット",
            MessageId::MailboxKindGift => "ギフト",
            MessageId::MailboxKindGuildInvite => "ギルド招待",
            MessageId::MailboxKindTradeOffer => "トレード",
            MessageId::CatchupClearsHint => "（バナーは次のミッションでクリアされます。）",
            MessageId::SettingsThemeDesc => "パレットを選択。ブラウザのローカルストレージに保存され、即時適用、再読込後も保持されます。",
            MessageId::SettingsCadenceDesc => "ローカルノードへの問い合わせ頻度。頻繁 = リーダーボードが俊敏、トラフィック増。緩やか = 軽量だが、契約は 60 秒の沈黙であなたを除外するので超えないこと。",
            MessageId::SettingsAutoMissionDesc => "HP が最大値のこの割合を下回ったら自動ループを一時停止。0% は旧挙動 — HP 0 でのみ停止。値を上げるほど、悪い目の連続で HP/フォーム/消耗品を失うのを防げます。",
            MessageId::SettingsPublishCheckbox => " ミッション直後にも公開する（定期ハートビートに加えて）",
            MessageId::SettingsIdentityBody => "Ed25519 シードを書き出して別のノードに ID を移すか、インベントリを新規キャラ状態に戻します。",
            MessageId::SettingsIdentityBodyStrong => "進行リセットは破壊的です",
            MessageId::SettingsIdentityBodyTail => " — 公開鍵は残りますが、すべてのカウンター、アイテム、スキル、エンディングがゼロに戻ります。",
            MessageId::SettingsAdvancedDesc => "低トラフィック / プライバシー / デバッグ用トグル。多くのプレイヤーにはデフォルトで十分です。",
            MessageId::SettingsHidePubkey => " 公開鍵を非表示（英雄パネル + 設定）",
            MessageId::SettingsHideStale => " 非アクティブなプレイヤーをリーダーボードから隠す（最終確認 > 30 秒前）",
            MessageId::SettingsWsOverride => "WS URL の上書き（空 = ?ws= またはデフォルト、再読込で適用）:",
            MessageId::SettingsResetUiPrefsDesc => "テーマ + 頻度 + 自動一時停止 + 詳細トグルをクリアしてページを再読込。インベントリは触りません — ノードにあります。",
            MessageId::SettingsWhereStateBody => "ローカルビューはノードにある状態のキャッシュにすぎません。再読込すると、ID とインベントリはデリゲートからすぐ戻ります。セーブを実際に消すには、ノードで `~/.config/freenet/secrets/local/<delegate-key>/` を削除してください。",
            MessageId::SettingsSeedRevealWarn => "一度だけコピーしてください。これらのバイトを持つ者は誰でも契約上であなたになりすませます。",
            MessageId::GuildsPanelDesc => "協力グループ — 初期の足場。作成すると他者は id で参加できます。1 プレイヤーは最大 1 ギルド。リーダーは離脱時に自動で引き継がれます。",
            MessageId::GuildsContractMissing => "ギルド契約が未設定です。",
            MessageId::GuildsContractMissingTail => " を公開（拡張は作業中）するか、キーを上書きしてください: ",
            MessageId::GuildsEmptyList => "（ギルドはまだありません — 最初の一人になろう）",
            MessageId::GuildsViaScript => " 経由 ",
            MessageId::GuildNamePlaceholder => "ギルド名（≤ 32 バイト）",
            MessageId::MailboxNotConfiguredHead => "メール契約が未設定です。",
            MessageId::MailboxNotConfiguredVia => " 経由 ",
            MessageId::MailboxNotConfiguredTail => " を公開（拡張は作業中）するか、設定してください ",
            MessageId::MailboxNotConfiguredIn => " 内に ",
            MessageId::ShopStashDesc => "スロット別にまとめたアイテム — 装備するか、商人にティア価格で売却",
            MessageId::ShopBuyGearDesc => "鍛冶屋のプリロール装備。「購入」を 1 回押すごとに、指定スロット+ティアの装備がストックに 1 つ加わります。伝説 (T4) は鍛造かドロップのみ。",
            MessageId::ShopSageDesc => "賢者は精と引き換えに恒久スキルの叡智を授けます。ベテラン/チャンピオンはレベルのマイルストーン由来 — 売り物ではありません。",
            MessageId::ShopFarmDesc => "戦闘なしの安全な収入。「働く」を 1 回押すごとに +1 小麦。商人は小麦 10 で 1 金。",
            MessageId::ShopFarmDescPassive => "領地の農夫が小麦を受動的に生産します — 商人は依然として小麦 10 で 1 金で買い取ります。",
            MessageId::TipFightInProgress => "戦闘中 — 現在のバトル終了をお待ちください",
            MessageId::TipAutoToggleMidFight => "戦闘中も自動トグルは機能します — 新しい設定は現バトル終了後に適用されます",
            MessageId::TipAutoEquipBest => "各スロットを走査し、所持中で未装備のステータス合計最大のものを装備",
            MessageId::TipAutoEquipNothing => "ストックには装備中のものを超えるアイテムがありません — 戦利品を増やすかフォームを変えてください",
            MessageId::TipEstateBlocksCombat => "領地が稼働中 — 戦うには領地パネルから一時停止してください",
            MessageId::TipPotionQueue => "キュー: 次のターンに完全回復",
            MessageId::TipPotionIdle => "HP を全回復",
            MessageId::TipFireballQueue => "キュー: 次のターンに追加ダメージ",
            MessageId::TipUnequipSlot => "装備解除 — ストックへ戻す",
            MessageId::TipDisbandLeader => "リーダー専用: 全員のためにギルドを削除",
            MessageId::PotionShopDesc => "HP を全回復します",
            MessageId::TermCorrupt => "（破損）",
            MessageId::HelpTheLoop => "ループ",
            MessageId::HelpStats => "ステータス",
            MessageId::HelpFormsTransformation => "フォームと変身",
            MessageId::HelpTabs => "タブ",
            MessageId::HelpShopGear => "ショップと装備",
            MessageId::HelpConsumables => "消耗品",
            MessageId::HelpWorldBoss => "ワールドボス",
            MessageId::HelpDelegateWhat => "デリゲートの役割は？",
            MessageId::HelpGuildsMailbox => "ギルドとメール（初期）",
            MessageId::HelpEstate => "領地 — 放置収入ループ",
            MessageId::HelpLegacy => "レガシー — 個人プレステージ",
            MessageId::HelpAreaGraph => "ワールドマップ — グラフレイアウト",
            MessageId::PanelEstate => "領地",
            MessageId::EstateBtnPause => "領地を一時停止",
            MessageId::EstateBtnRun => "領地を稼働",
            MessageId::EstateColTier => "ティア",
            MessageId::EstateColOwned => "雇用数",
            MessageId::EstateColYield => "産出/秒",
            MessageId::EstateColNextPrice => "次の価格",
            MessageId::BtnHire => "雇う",
            MessageId::EstateResWheat => "小麦",
            MessageId::EstateResGold => "金",
            MessageId::EstateResEssence => "精",
            MessageId::PanelLegacy => "レガシー",
            MessageId::LegacyColNode => "ノード",
            MessageId::LegacyColLevel => "レベル",
            MessageId::LegacyColMultiplier => "倍率",
            MessageId::LegacyColNextCost => "次のコスト",
            MessageId::BtnAscend => "昇華",
            MessageId::LegacyAscendBlurb => "ソフトリセット: スター、レベル、ミッション、スキルは保持。金、装備、領地は消去。",
            MessageId::LegacyAscendConfirm => "昇華 — ソフトリセットを行いますか？ スター、レベル、ミッション数、スキルは保持され、金、装備、領地は消去されます。",
            MessageId::CatchupModalTitle => "おかえりなさい",
            MessageId::BtnGotIt => "了解",
            MessageId::NewerBuildDesc => "新しいビルドが公開中です — 今回は詳細な更新履歴はありません。",
            MessageId::PanelFormsShop => "フォーム",
            MessageId::FormsShopDesc => "姿をリセット（人間は安価）するか、他の 4 フォームのいずれかに金を投資します。各フォームは領地アフィニティも決定 — 馬は農夫+採取者を強化、ドラゴンは商人+賢者を強化、猫は採取者+賢者を強化、スライムは全段に +30%、人間は中立。直接購入は敗北による変身と同じ扱い: フォームが訪問済みに追加され、賢者でそのスキルが解放されます。",
            MessageId::FormsShopBaselineDesc => "バランス重視の基本 — ステータス追加なし",
            MessageId::TipFormAlreadyActive => "すでにこのフォームです",
            MessageId::PanelActivities => "アクティビティ",
            MessageId::ActivitiesDesc => "現在のエリアに結びついた非戦闘アクション。選択するとあなたの放置アクションになります — 自動ミッションと領地は停止するまで一時停止します。",
            MessageId::ActivityStart => "開始",
            MessageId::ActivityStop => "停止",
            MessageId::PanelRoutine => "ルーチン",
            MessageId::RoutineDesc => "金がある限り、目標人数まで領地の労働者を自動雇用します。デリゲートティックあたり 50 雇用に制限 — 巨大なキャッチアップで国庫が空にならないようにするためです。",
            MessageId::RoutineColTier => "ティア",
            MessageId::RoutineColCurrent => "雇用数",
            MessageId::RoutineColTarget => "目標",
            MessageId::PanelInsight => "洞察",
            MessageId::InsightDesc => "希少通貨。ミッション 25 回ごとと、アストラルの「印を解読」アクティビティで獲得。小さな恒久バフに消費。",
            MessageId::InsightColNode => "ノード",
            MessageId::InsightColLevel => "レベル",
            MessageId::InsightColNextCost => "次のコスト",
            MessageId::PanelBossAttack => "個人ボス攻撃",
            MessageId::BossAttackBtn => "攻撃 (-200 精、+50 ボスダメージ)",
            MessageId::BossAttackDesc => "精を消費して、戦闘外で共有ワールドボスを削る。ミッション数 ≥ 100、レベル ≥ 10、領地労働者が 1 体以上で解放。",
            MessageId::BossAttackLocked => "未解放 — ミッション 100、レベル 10、領地労働者 1 体以上が必要です。",
            MessageId::PanelTokens => "トークン",
            MessageId::TokensDesc => "個人ボスダメージ 500 ごとに 1 つ獲得。現在は装飾系特典に消費 — ゲームプレイ系特典（装備スロット、第 2 の自動ミッションプリセット）は配管が整い次第。",
            MessageId::TokenColPerk => "特典",
            MessageId::TokenColPrice => "価格",
            MessageId::BtnUnlock => "解放",
            MessageId::ResInsight => "洞察",
            MessageId::ResTokens => "トークン",
            MessageId::MasteryIntro => "恒久強化 — 一度購入すれば永続。レガシーのスターはレベルのマイルストーンから、洞察はミッションとアストラル活動から、トークンは個人ボスダメージから。ルーチンの目標とワールドボス攻撃もここに集まります。",
            MessageId::PanelWilds => "ワイルド",
            MessageId::WildsDesc => "後半戦の代替マップ。あなたのプロットシードから手続き生成されます。名前と敵ステータスのノイズはあなた固有、トポロジーは固定（8 ノード、入口から 2 つの分岐、合流ノード）。ワールドボスへの寄与はありません。",
            MessageId::MapViewLinear => "リニア",
            MessageId::MapViewWilds => "ワイルド",
        })
    }
}

/// Compound / parametric helpers — strings whose template needs to
/// differ between locales (different word order, different plural
/// forms, embedded arguments). The methods return `String` so each
/// call site is just a substitution.
impl Locale {
    /// Welcome banner body — appears on Farm tab before the first
    /// mission completes.
    pub fn fmt_tutorial_run_mission(self) -> String {
        // Static; no formatting args. Kept as a method for symmetry
        // with the other compound helpers.
        self.tr(MessageId::TutorialBody1).to_string()
    }

    /// "while you were away" body line 1 (missions run).
    pub fn fmt_catchup_summary(self, elapsed_human: &str, missions_won: u32, missions_lost: u32) -> String {
        let mut out = match self.fmt_locale() {
            Self::En => format!("Auto-mode ran for {elapsed_human} ({missions_won} missions)."),
            Self::Ru => format!("Авто-режим работал {elapsed_human} ({missions_won} миссий)."),
            Self::Fr => format!("Le mode auto a tourné pendant {elapsed_human} ({missions_won} missions)."),
            Self::Es => format!("El modo auto se ejecutó durante {elapsed_human} ({missions_won} misiones)."),
            Self::Ja => format!("自動モードは {elapsed_human} 動作しました（{missions_won} 件のミッション）。"),
            _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        };
        if missions_lost > 0 {
            let tail = match self.fmt_locale() {
                Self::En => format!(" {missions_lost} ended in defeat."),
                Self::Ru => format!(" {missions_lost} закончились поражением."),
                Self::Fr => format!(" {missions_lost} se sont soldées par une défaite."),
                Self::Es => format!(" {missions_lost} terminaron en derrota."),
                Self::Ja => format!(" {missions_lost} 件は敗北で終了しました。"),
                _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
            };
            out.push_str(&tail);
        }
        out
    }

    /// "rewards: …" line on the catch-up banner.
    pub fn fmt_catchup_rewards(self, gold: &str, essence: &str, xp: &str, dmg: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("rewards: +{gold}g · +{essence}e · +{xp} XP · +{dmg} boss damage"),
            Self::Ru => format!("награды: +{gold} зол · +{essence} эсс · +{xp} опыта · +{dmg} урона по боссу"),
            Self::Fr => format!("récompenses : +{gold} or · +{essence} ess · +{xp} XP · +{dmg} dégâts boss"),
            Self::Es => format!("recompensas: +{gold} oro · +{essence} ess · +{xp} XP · +{dmg} daño al jefe"),
            Self::Ja => format!("報酬: +{gold} 金 · +{essence} 精 · +{xp} XP · +{dmg} ボスダメージ"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Onboarding step counter ("step 1 / 4").
    pub fn fmt_onboarding_step(self, current: u8, total: u8) -> String {
        match self.fmt_locale() {
            Self::En => format!("step {current} / {total}"),
            Self::Ru => format!("шаг {current} / {total}"),
            Self::Fr => format!("étape {current} / {total}"),
            Self::Es => format!("paso {current} / {total}"),
            Self::Ja => format!("ステップ {current} / {total}"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Last-publish timestamp string ("3s ago" / "never").
    pub fn fmt_seconds_ago(self, seconds: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("{seconds}s ago"),
            Self::Ru => format!("{seconds} с назад"),
            Self::Fr => format!("il y a {seconds} s"),
            Self::Es => format!("hace {seconds} s"),
            Self::Ja => format!("{seconds} 秒前"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    pub fn term_never(self) -> &'static str {
        self.tr(MessageId::TermNever)
    }

    /// Estate hint line shown above the worker grid — embeds the
    /// active form name so the player can see at a glance which
    /// affinities will apply.
    pub fn fmt_estate_hint(self, form_name: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!(
                "Workers produce while Estate is the active idle action. Active form: {form_name}."
            ),
            Self::Ru => format!(
                "Работники приносят доход, пока активным простойным действием выбрано Поместье. Текущая форма: {form_name}."
            ),
            Self::Fr => format!(
                "Les travailleurs produisent tant que le Domaine est l'action passive active. Forme actuelle : {form_name}."
            ),
            Self::Es => format!(
                "Los trabajadores producen mientras la Finca sea la acción pasiva activa. Forma actual: {form_name}."
            ),
            Self::Ja => format!(
                "領地がアクティブな放置アクションのとき、労働者が生産します。現在のフォーム: {form_name}。"
            ),
            _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Legacy / Epoch header summary — stars, ascend count, and the
    /// level at which the next star milestone fires.
    pub fn fmt_legacy_header(
        self,
        stars: u64,
        ascensions: u64,
        next_star_level: u64,
    ) -> String {
        match self.fmt_locale() {
            Self::En => format!(
                "Stars: {stars}  ·  Ascensions: {ascensions}  ·  Next star at level {next_star_level}"
            ),
            Self::Ru => format!(
                "Звёзды: {stars}  ·  Вознесений: {ascensions}  ·  След. звезда на уровне {next_star_level}"
            ),
            Self::Fr => format!(
                "Étoiles : {stars}  ·  Ascensions : {ascensions}  ·  Prochaine étoile au niveau {next_star_level}"
            ),
            Self::Es => format!(
                "Estrellas: {stars}  ·  Ascensiones: {ascensions}  ·  Próxima estrella al nivel {next_star_level}"
            ),
            Self::Ja => format!(
                "スター: {stars}  ·  昇華回数: {ascensions}  ·  次のスターはレベル {next_star_level}"
            ),
            _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Catchup modal "What's new in v…" header — shown above the
    /// curated patchnotes block.
    pub fn fmt_whats_new(self, version: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("What's new in v{version}"),
            Self::Ru => format!("Что нового в v{version}"),
            Self::Fr => format!("Nouveautés de la v{version}"),
            Self::Es => format!("Novedades en la v{version}"),
            Self::Ja => format!("v{version} の新着情報"),
            _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Catchup modal fallback header — shown when the version
    /// changed but no curated notes were shipped for this build.
    pub fn fmt_now_running(self, version: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("Now running v{version}"),
            Self::Ru => format!("Сейчас работает v{version}"),
            Self::Fr => format!("Version actuelle : v{version}"),
            Self::Es => format!("Ejecutando v{version}"),
            Self::Ja => format!("現在のバージョン: v{version}"),
            _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Catchup modal Estate-subsection breakdown line for one tier.
    pub fn fmt_estate_worker_line(self, tier_name: &str, count: u64) -> String {
        format!("{tier_name}: {count}")
    }

    /// Inbox count line on the mailbox panel — "inbox: N messages".
    pub fn fmt_inbox_count(self, n: usize) -> String {
        match self.fmt_locale() {
            Self::En => format!("inbox: {n} message{}", if n == 1 { "" } else { "s" }),
            // Russian plural: 1, 2-4 → message/messages/messagesgen — we
            // collapse to the universal "сообщений" suffix which is
            // grammatical for 0, 5+ and any non-1 count when read as
            // an abbreviated count. For n=1 we use "1 сообщение".
            Self::Ru => {
                let suffix = ru_plural(n as u64, "сообщение", "сообщения", "сообщений");
                format!("ящик: {n} {suffix}")
            }
            // French has a binary plural (singular for 0 and 1, plural for
            // 2+) and Spanish has English-like plurals. Japanese has no
            // grammatical plural, so the suffix is the noun unchanged.
            Self::Fr => format!(
                "boîte : {n} message{}",
                if n <= 1 { "" } else { "s" }
            ),
            Self::Es => format!(
                "bandeja: {n} mensaje{}",
                if n == 1 { "" } else { "s" }
            ),
            Self::Ja => format!("受信箱: {n} 件のメッセージ"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Stash item count line — "N items in stash — manage at the Shop tab".
    pub fn fmt_stash_count(self, n: usize) -> String {
        match self.fmt_locale() {
            Self::En => format!(
                "{n} item{} in stash — manage at the Shop tab",
                if n == 1 { "" } else { "s" }
            ),
            Self::Ru => {
                let suffix = ru_plural(n as u64, "предмет", "предмета", "предметов");
                format!("{n} {suffix} в запасе — управляй на вкладке Магазин")
            }
            Self::Fr => format!(
                "{n} objet{} en réserve — gérez-les dans l'onglet Boutique",
                if n <= 1 { "" } else { "s" }
            ),
            Self::Es => format!(
                "{n} objeto{} en el alijo — gestiona en la pestaña Tienda",
                if n == 1 { "" } else { "s" }
            ),
            Self::Ja => format!("ストック: {n} 個 — ショップタブから管理"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "(N / M)" label suffix on panel headers — locale-agnostic, just
    /// renders the numbers; kept as a method so panel titles read uniformly.
    pub fn fmt_count_of(self, label: &str, n: usize, total: usize) -> String {
        format!("{label} ({n}/{total})")
    }

    /// "Era N · X / Y HP — Z total damage from W players".
    pub fn fmt_boss_summary(self, era: u64, hp: &str, max_hp: &str, total_dmg: &str, players: usize) -> String {
        match self.fmt_locale() {
            Self::En => format!(
                "Era {era} · {hp} / {max_hp} HP — {total_dmg} total damage from {players} players"
            ),
            Self::Ru => format!(
                "Эра {era} · {hp} / {max_hp} ОЗ — суммарно {total_dmg} урона от {players} игроков"
            ),
            Self::Fr => format!(
                "Ère {era} · {hp} / {max_hp} PV — {total_dmg} dégâts cumulés de {players} joueurs"
            ),
            Self::Es => format!(
                "Era {era} · {hp} / {max_hp} PV — {total_dmg} de daño total de {players} jugadores"
            ),
            Self::Ja => format!(
                "エラ {era} · {hp} / {max_hp} HP — {players} 人のプレイヤーから合計 {total_dmg} ダメージ"
            ),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "currently farming: X · level Y" on the world map.
    pub fn fmt_currently_farming(self, area: &str, lvl: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("currently farming: {area} · level {lvl}"),
            Self::Ru => format!("сейчас фармишь: {area} · уровень {lvl}"),
            Self::Fr => format!("zone actuelle : {area} · niveau {lvl}"),
            Self::Es => format!("farmeando ahora: {area} · nivel {lvl}"),
            Self::Ja => format!("現在の狩り場: {area} · レベル {lvl}"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "gold balance: X · potions: Y · fireballs: Z" — Shop top line.
    /// Quantities are passed as strings so any integer width works.
    pub fn fmt_shop_balance(self, gold: &str, potions: &str, fireballs: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("gold balance: {gold} · potions: {potions} · fireballs: {fireballs}"),
            Self::Ru => format!("золото: {gold} · зелья: {potions} · фаерболы: {fireballs}"),
            Self::Fr => format!("solde or : {gold} · potions : {potions} · boules de feu : {fireballs}"),
            Self::Es => format!("oro: {gold} · pociones: {potions} · bolas de fuego: {fireballs}"),
            Self::Ja => format!("所持金: {gold} · ポーション: {potions} · ファイアボール: {fireballs}"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "Buy (Xg)" — paid-by-gold button label.
    pub fn fmt_buy_gold(self, price: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("Buy ({price}g)"),
            Self::Ru => format!("Купить ({price} зол)"),
            Self::Fr => format!("Acheter ({price} or)"),
            Self::Es => format!("Comprar ({price} oro)"),
            Self::Ja => format!("購入 ({price} 金)"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "Buy (Xe)" — paid-by-essence button label.
    pub fn fmt_buy_essence(self, price: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("Buy ({price}e)"),
            Self::Ru => format!("Купить ({price} эсс)"),
            Self::Fr => format!("Acheter ({price} ess)"),
            Self::Es => format!("Comprar ({price} ess)"),
            Self::Ja => format!("購入 ({price} 精)"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "active players (N)" header.
    pub fn fmt_active_players(self, n: usize) -> String {
        match self.fmt_locale() {
            Self::En => format!("active players ({n})"),
            Self::Ru => format!("активные игроки ({n})"),
            Self::Fr => format!("joueurs actifs ({n})"),
            Self::Es => format!("jugadores activos ({n})"),
            Self::Ja => format!("アクティブなプレイヤー ({n})"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "you are in: <guild name>" — h3 on the Guilds tab.
    pub fn fmt_you_are_in_guild(self, name: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("you are in: {name}"),
            Self::Ru => format!("ты в гильдии: {name}"),
            Self::Fr => format!("vous êtes dans : {name}"),
            Self::Es => format!("estás en: {name}"),
            Self::Ja => format!("所属ギルド: {name}"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "members: N / M · leader: …" — guild meta line.
    pub fn fmt_guild_meta(self, members: usize, max_members: usize, leader_label: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("members: {members} / {max_members} · leader: {leader_label}"),
            Self::Ru => format!("участники: {members} / {max_members} · лидер: {leader_label}"),
            Self::Fr => format!("membres : {members} / {max_members} · chef : {leader_label}"),
            Self::Es => format!("miembros: {members} / {max_members} · líder: {leader_label}"),
            Self::Ja => format!("メンバー: {members} / {max_members} · リーダー: {leader_label}"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "directory (N)" header.
    pub fn fmt_directory(self, n: usize) -> String {
        match self.fmt_locale() {
            Self::En => format!("directory ({n})"),
            Self::Ru => format!("каталог ({n})"),
            Self::Fr => format!("annuaire ({n})"),
            Self::Es => format!("directorio ({n})"),
            Self::Ja => format!("ディレクトリ ({n})"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "stash (N)" header on the Shop tab.
    pub fn fmt_stash_header(self, n: usize) -> String {
        match self.fmt_locale() {
            Self::En => format!("stash ({n})"),
            Self::Ru => format!("запас ({n})"),
            Self::Fr => format!("réserve ({n})"),
            Self::Es => format!("alijo ({n})"),
            Self::Ja => format!("ストック ({n})"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Sync-cadence button labels — were `&'static str` on SyncCadence.
    /// Pass the cadence in and we route to the right pair.
    pub fn fmt_sync_cadence(self, cadence: crate::app::prefs::SyncCadence) -> &'static str {
        use crate::app::prefs::SyncCadence as C;
        match (self.fmt_locale(), cadence) {
            (Self::En, C::Aggressive) => "Aggressive (5s)",
            (Self::Ru, C::Aggressive) => "Агрессивно (5с)",
            (Self::Fr, C::Aggressive) => "Agressif (5s)",
            (Self::Es, C::Aggressive) => "Agresivo (5s)",
            (Self::Ja, C::Aggressive) => "頻繁 (5秒)",
            (Self::En, C::Normal) => "Normal (10s)",
            (Self::Ru, C::Normal) => "Обычно (10с)",
            (Self::Fr, C::Normal) => "Normal (10s)",
            (Self::Es, C::Normal) => "Normal (10s)",
            (Self::Ja, C::Normal) => "通常 (10秒)",
            (Self::En, C::Easy) => "Easy (30s)",
            (Self::Ru, C::Easy) => "Спокойно (30с)",
            (Self::Fr, C::Easy) => "Léger (30s)",
            (Self::Es, C::Easy) => "Suave (30s)",
            (Self::Ja, C::Easy) => "緩やか (30秒)",
            (_, _) => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Auto-pause HP-percent radio label.
    pub fn fmt_hp_pause_label(self, pct: u8) -> String {
        match (self, pct) {
            (Self::En, 0) => "0% (only at 0 HP)".to_string(),
            (Self::Ru, 0) => "0% (только при 0 ОЗ)".to_string(),
            (Self::Fr, 0) => "0 % (uniquement à 0 PV)".to_string(),
            (Self::Es, 0) => "0% (solo a 0 PV)".to_string(),
            (Self::Ja, 0) => "0%（HP 0 のときのみ）".to_string(),
            (_, p) => format!("{p}%"),
        }
    }

    /// World-map area card footer for the level-locked state — "lvl X required".
    pub fn fmt_lvl_required(self, min_level: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("lvl {min_level} required"),
            Self::Ru => format!("нужен ур. {min_level}"),
            Self::Fr => format!("niv. {min_level} requis"),
            Self::Es => format!("nv. {min_level} requerido"),
            Self::Ja => format!("Lv. {min_level} 必要"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// World-map area card footer for the clears-locked state —
    /// "clears N / M in prev". The level gate is OK but the
    /// predecessor zone hasn't been cleared enough times yet.
    /// Shows progress so the player knows how close they are.
    pub fn fmt_clears_required(self, have: u64, need: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("clears {have} / {need} in prev"),
            Self::Ru => format!("нужно {have} / {need} зачисток в предыд."),
            Self::Fr => format!("nettoyages {have} / {need} en préc."),
            Self::Es => format!("limpiezas {have} / {need} en prev."),
            Self::Ja => format!("前提エリア突破 {have} / {need}"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Compact "cleared N times" indicator on each area card —
    /// shown alongside the gold / essence / damage badges so the
    /// player sees their mastery progress per zone.
    pub fn fmt_cleared_count(self, n: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("cleared {n}"),
            Self::Ru => format!("зачищено {n}"),
            Self::Fr => format!("nettoyé {n}×"),
            Self::Es => format!("limpiada {n}×"),
            Self::Ja => format!("{n} 回クリア"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Encounter progress line during a battle.
    pub fn fmt_encounter_progress(self, idx: u32, total: u32) -> String {
        match self.fmt_locale() {
            Self::En => format!("encounter {idx} / {total}"),
            Self::Ru => format!("сражение {idx} / {total}"),
            Self::Fr => format!("rencontre {idx} / {total}"),
            Self::Es => format!("encuentro {idx} / {total}"),
            Self::Ja => format!("戦闘 {idx} / {total}"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Tutorial / no-encounters hint about gear drop cadence.
    pub fn fmt_no_spare_loot(self, every_n: u32) -> String {
        match self.fmt_locale() {
            Self::En => format!("no spare loot yet — gear drops every {every_n} missions"),
            Self::Ru => format!("свободного снаряжения нет — выпадает каждые {every_n} миссий"),
            Self::Fr => format!("pas encore d'équipement en surplus — un objet tombe toutes les {every_n} missions"),
            Self::Es => format!("aún no hay botín de sobra — el equipo cae cada {every_n} misiones"),
            Self::Ja => format!("予備の装備はまだなし — {every_n} ミッションごとにドロップ"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "Chapter N" caption on the plot panel.
    pub fn fmt_chapter(self, n: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("Chapter {n}"),
            Self::Ru => format!("Глава {n}"),
            Self::Fr => format!("Chapitre {n}"),
            Self::Es => format!("Capítulo {n}"),
            Self::Ja => format!("第 {n} 章"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Plot backstory paragraph — six-slot Mad Libs from a stable seed.
    /// The English template uses inline `{home}/{mac}/{vil}/{mthd}/{dest}`
    /// substitutions; Russian rewrites the sentence structure so the
    /// nouns land in the right grammatical positions.
    pub fn fmt_plot_backstory(self, home: &str, mac: &str, vil: &str, mthd: &str, dest: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!(
                "You were abandoned in the {home} as a baby. Then one day, the {mac} disappeared! Surely the {vil} used the {mthd} to take it! Now you must journey to the {dest} to confront them."
            ),
            // Word-list items from the shared crate are English nouns;
            // re-shaping the Russian sentence keeps it readable without
            // touching the shared word lists.
            Self::Ru => format!(
                "Тебя бросили младенцем в {home}. И вот однажды исчез {mac}! Это, конечно же, {vil} применил {mthd}, чтобы его забрать! Теперь твой путь — в {dest}, чтобы встретиться с ним лицом к лицу."
            ),
            Self::Fr => format!(
                "Tu as été abandonné(e) dans {home} alors que tu étais bébé. Puis un jour, {mac} a disparu ! Sans doute {vil} a-t-il utilisé {mthd} pour le prendre ! Tu dois maintenant te rendre à {dest} pour les affronter."
            ),
            Self::Es => format!(
                "Te abandonaron en {home} cuando eras un bebé. Un día, ¡{mac} desapareció! Seguro que {vil} usó {mthd} para llevárselo. Ahora debes viajar a {dest} para enfrentarlos."
            ),
            Self::Ja => format!(
                "あなたは赤子の頃 {home} に置き去りにされた。そしてある日、{mac} が消えた！ きっと {vil} が {mthd} を使って奪ったに違いない！ いまこそ {dest} へ向かい、彼らと対峙する時だ。"
            ),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "Mission in <area>: up to N encounters, ..." subtitle under the
    /// Run-Mission button when no battle is in flight.
    pub fn fmt_mission_summary(self, area: &str, encounters: u32, essence: u64, mission_damage: u64) -> String {
        match self.fmt_locale() {
            Self::En => {
                if mission_damage == 0 {
                    format!("Mission in {area}: up to {encounters} encounters, ~{essence} essence per win, no World Boss contribution from this area — gold scales by enemy")
                } else {
                    format!("Mission in {area}: up to {encounters} encounters, ~{essence} essence + ~{mission_damage} boss damage per win — gold scales by enemy")
                }
            }
            Self::Ru => {
                if mission_damage == 0 {
                    format!("Миссия в {area}: до {encounters} сражений, ~{essence} эссенции за победу, эта область не бьёт Мирового Босса — золото зависит от врага")
                } else {
                    format!("Миссия в {area}: до {encounters} сражений, ~{essence} эссенции + ~{mission_damage} урона по боссу за победу — золото зависит от врага")
                }
            }
            Self::Fr => {
                if mission_damage == 0 {
                    format!("Mission à {area} : jusqu'à {encounters} rencontres, ~{essence} essence par victoire, cette zone n'apporte aucun dégât au Boss du Monde — l'or dépend de l'ennemi")
                } else {
                    format!("Mission à {area} : jusqu'à {encounters} rencontres, ~{essence} essence + ~{mission_damage} dégâts au Boss par victoire — l'or dépend de l'ennemi")
                }
            }
            Self::Es => {
                if mission_damage == 0 {
                    format!("Misión en {area}: hasta {encounters} encuentros, ~{essence} de esencia por victoria, esta zona no aporta daño al Jefe del Mundo — el oro depende del enemigo")
                } else {
                    format!("Misión en {area}: hasta {encounters} encuentros, ~{essence} de esencia + ~{mission_damage} de daño al Jefe por victoria — el oro depende del enemigo")
                }
            }
            Self::Ja => {
                if mission_damage == 0 {
                    format!("{area} でのミッション: 最大 {encounters} 戦闘、勝利ごとに精 ~{essence}、このエリアはワールドボスに寄与しません — 金は敵に応じて変動")
                } else {
                    format!("{area} でのミッション: 最大 {encounters} 戦闘、勝利ごとに精 ~{essence} + ボスダメージ ~{mission_damage} — 金は敵に応じて変動")
                }
            }
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "last publish: <age> · published gold X · published damage Y"
    pub fn fmt_last_publish(self, age: &str, gold: &str, damage: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("last publish: {age} · published gold {gold} · published damage {damage}"),
            Self::Ru => format!("последняя публикация: {age} · золото {gold} · урон по боссу {damage}"),
            Self::Fr => format!("dernière publication : {age} · or publié {gold} · dégâts publiés {damage}"),
            Self::Es => format!("última publicación: {age} · oro publicado {gold} · daño publicado {damage}"),
            Self::Ja => format!("最終公開: {age} · 公開金 {gold} · 公開ダメージ {damage}"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "equipped bonus: +X atk · +Y def · +Z hp" — equipment-panel subtitle.
    pub fn fmt_equipped_bonus(self, atk: u64, def: u64, hp: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("equipped bonus: +{atk} atk · +{def} def · +{hp} hp"),
            Self::Ru => format!("бонус экипировки: +{atk} атк · +{def} защ · +{hp} ОЗ"),
            Self::Fr => format!("bonus d'équipement : +{atk} att · +{def} déf · +{hp} pv"),
            Self::Es => format!("bono de equipo: +{atk} atq · +{def} def · +{hp} pv"),
            Self::Ja => format!("装備ボーナス: +{atk} 攻 · +{def} 防 · +{hp} HP"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "deals {N} damage to the World Boss" — fireball idle tooltip.
    pub fn fmt_fireball_idle(self, dmg: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("deals {dmg} damage to the World Boss"),
            Self::Ru => format!("наносит {dmg} урона Мировому Боссу"),
            Self::Fr => format!("inflige {dmg} dégâts au Boss du Monde"),
            Self::Es => format!("inflige {dmg} de daño al Jefe del Mundo"),
            Self::Ja => format!("ワールドボスに {dmg} ダメージ"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "convert all wheat to gold at 1:N" — Sell All Wheat tooltip.
    pub fn fmt_sell_wheat_tooltip(self, ratio: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("convert all wheat to gold at 1:{ratio}"),
            Self::Ru => format!("обменять всю пшеницу на золото по курсу 1:{ratio}"),
            Self::Fr => format!("convertir tout le blé en or au taux 1:{ratio}"),
            Self::Es => format!("convertir todo el trigo en oro a 1:{ratio}"),
            Self::Ja => format!("小麦をすべて金に交換（レート 1:{ratio}）"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// "wheat: N · would sell for Mg" — farm panel running total.
    pub fn fmt_wheat_balance(self, wheat: &str, gold: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("wheat: {wheat} · would sell for {gold}g"),
            Self::Ru => format!("пшеница: {wheat} · принесёт {gold} зол"),
            Self::Fr => format!("blé : {wheat} · vaudrait {gold} or"),
            Self::Es => format!("trigo: {wheat} · se vendería por {gold} oro"),
            Self::Ja => format!("小麦: {wheat} · 売れば {gold} 金"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Confirm-dialog body: "Reset all inventory progress?…"
    pub fn confirm_reset_progress(self) -> &'static str {
        match self.fmt_locale() {
            Self::En => "Reset all inventory progress?\n\nYour identity (pubkey) stays the same — leaderboards keep recognizing you — but every counter, item, skill, ending, and achievement goes back to zero.",
            Self::Ru => "Сбросить весь прогресс инвентаря?\n\nЛичность (публичный ключ) остаётся той же — таблицы лидеров продолжат тебя узнавать — но все счётчики, предметы, навыки, финалы и достижения обнулятся.",
            Self::Fr => "Réinitialiser toute la progression d'inventaire ?\n\nVotre identité (clé publique) reste la même — les classements vous reconnaissent toujours — mais tous les compteurs, objets, compétences, fins et succès reviennent à zéro.",
            Self::Es => "¿Restablecer todo el progreso de inventario?\n\nTu identidad (clave pública) sigue siendo la misma — las clasificaciones siguen reconociéndote — pero todos los contadores, objetos, habilidades, finales y logros vuelven a cero.",
            Self::Ja => "インベントリの進行状況をすべてリセットしますか？\n\nID（公開鍵）はそのまま——リーダーボードはあなたを認識し続けます——が、すべてのカウンター、アイテム、スキル、エンディング、実績がゼロに戻ります。",
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Confirm-dialog body: "Reveal your Ed25519 seed?…"
    pub fn confirm_reveal_seed(self) -> &'static str {
        match self.fmt_locale() {
            Self::En => "Reveal your Ed25519 seed?\n\nAnyone holding it can impersonate you. Only paste it into trusted backup storage; never into chat or screenshots.",
            Self::Ru => "Показать Ed25519 seed?\n\nЛюбой, кто получит его, сможет выдать себя за тебя. Вставляй его только в надёжное хранилище резервных копий — никогда в чат или скриншоты.",
            Self::Fr => "Révéler votre seed Ed25519 ?\n\nQuiconque la détient peut se faire passer pour vous. Ne la copiez que dans un stockage de sauvegarde de confiance ; jamais dans un chat ou des captures d'écran.",
            Self::Es => "¿Mostrar tu seed Ed25519?\n\nCualquiera que la tenga puede suplantarte. Pégala solo en almacenamiento de respaldo confiable; nunca en chat ni capturas de pantalla.",
            Self::Ja => "Ed25519 シードを表示しますか？\n\nこれを持つ者は誰でもあなたになりすませます。信頼できるバックアップ先にのみ貼り付け、チャットやスクリーンショットには絶対に貼らないでください。",
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Confirm-dialog body: "Disband \"<name>\"?…"
    pub fn confirm_disband_guild(self, guild_name: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!(
                "Disband \"{guild_name}\"?\n\nThis removes the guild entirely and every member loses their membership immediately. Only you (the current leader) can do this; if you change your mind, just don't click OK.",
            ),
            Self::Ru => format!(
                "Распустить гильдию «{guild_name}»?\n\nЭто полностью удалит гильдию, и все участники потеряют членство сразу. Только ты (текущий лидер) можешь это сделать; если передумал — просто не нажимай ОК.",
            ),
            Self::Fr => format!(
                "Dissoudre « {guild_name} » ?\n\nCela supprime la guilde entièrement et chaque membre perd son adhésion immédiatement. Vous seul (chef actuel) pouvez faire cela ; si vous changez d'avis, il suffit de ne pas cliquer sur OK.",
            ),
            Self::Es => format!(
                "¿Disolver «{guild_name}»?\n\nEsto elimina el gremio por completo y todos los miembros pierden su pertenencia inmediatamente. Solo tú (el líder actual) puedes hacerlo; si cambias de opinión, simplemente no pulses OK.",
            ),
            Self::Ja => format!(
                "「{guild_name}」を解散しますか？\n\nギルドは完全に削除され、すべてのメンバーは直ちに脱退となります。これを実行できるのはあなた（現リーダー）だけです。気が変わったら OK を押さないでください。",
            ),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Status-bar line after exporting the seed — flips between
    /// success and error variants.
    pub fn status_seed_exported(self) -> &'static str {
        match self.fmt_locale() {
            Self::En => "seed exported — copy and hide promptly",
            Self::Ru => "seed экспортирован — скопируй и спрячь поскорее",
            Self::Fr => "seed exportée — copiez et masquez sans tarder",
            Self::Es => "seed exportada — copia y oculta cuanto antes",
            Self::Ja => "シードを書き出しました — 速やかにコピーして隠してください",
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    pub fn fmt_status_seed_export_failed(self, err: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("export failed: {err}"),
            Self::Ru => format!("экспорт не удался: {err}"),
            Self::Fr => format!("échec de l'export : {err}"),
            Self::Es => format!("falló la exportación: {err}"),
            Self::Ja => format!("書き出しに失敗: {err}"),
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }

    /// Help-tab body — kept as one big formatted blob per locale.
    /// Returns plain text (without inline `<strong>` markup); the
    /// help tab no longer needs the bolded keywords once translated.
    pub fn help_body(self) -> HelpBody {
        match self.fmt_locale() {
            Self::En => HelpBody::EN,
            Self::Ru => HelpBody::RU,
            Self::Fr => HelpBody::FR,
            Self::Es => HelpBody::ES,
            Self::Ja => HelpBody::JA,
                    _ => unreachable!("fmt_locale normalises non-En/Ru locales"),
        }
    }
}

/// Russian count-plural helper: maps a count → the right grammatical
/// suffix among (1, 2–4, others). Russian distinguishes three forms;
/// English collapses to two (s/no-s), handled separately in the
/// callers via `if n == 1`.
fn ru_plural(n: u64, one: &'static str, few: &'static str, many: &'static str) -> &'static str {
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

/// Help-tab body strings — paragraphs without inline `<strong>` markup
/// (the tag would have to ship via `dangerously_set_inner_html` to
/// survive translation, which isn't worth the security trade-off for
/// what's essentially a printed reference). Each paragraph is a single
/// `&'static str`; the help renderer threads them through `<p>` tags.
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
    pub const EN: Self = Self {
        loop_p1: "Click Run Mission on the Farm tab to start a chain of up to 5 encounters against the current area's enemies. Combat is tick-based: one turn fires every 700ms, and you can queue Use Potion or Use Fireball mid-fight to react to a bad streak. Each win grants gold, essence, XP, and chips the shared World Boss. Lose, and you transform into the enemy that beat you.",
        loop_p2: "Toggle auto: on and the node-side delegate keeps running missions even after you close the tab. You'll return to a while you were away banner summing up what happened (capped at ~1 hour of catch-up). Auto pauses when HP drops below the threshold you pick in Settings → auto-mission.",
        stats_p1: "Your Level comes from cumulative XP. XP per level rises 1.5× each step (100, 150, 225, 337, …). Base stats are static per level: HP = 20 + lvl×5, Attack = 5 + lvl×2, Defence = 5 + lvl×2. Equipment, form, and skills add on top; nothing else (no gold/essence bleed-through).",
        stats_p2: "HP depletes in combat and regenerates over time (full regen in 180s of real time). Use a Potion to instantly fill it.",
        forms_p1: "Losing a combat to a non-mundane enemy permanently transforms you into that monster. Each form has its own equipped-slot mask: a Slime can only wear Helm + Ring, a Cat keeps Helm/Cloak/Boots/Ring, etc. Stats shift to match the form. Every form you've touched leaves a permanent Skill — even after you change back to Human, those bonuses carry. This is the prestige loop.",
        forms_p2: "Forms also drive Estate affinity: while you're in a given form, your workers in matching tiers produce more, while non-matching tiers produce less. Horse buffs Farmhand + Forager, Dragon buffs Trader + Sage, Cat buffs Forager + Sage, Slime gives a flat +30% across the board, Human is neutral. The Shop tab has a paid Forms section so you can pick a shape directly without waiting for a defeat — Human is the cheap reset, the other four are a strategic commitment.",
        tabs: &[
            "🛡 Farm — your hero, the live combat scene (HP bars, queue-action buttons), plot, World Boss, raw resources.",
            "🗺 World Map — switch farming areas. Higher areas have a level gate but pay more (or differently — Forest is essence-rich, Mountain is gold-rich, Boss's Lair is damage-heavy).",
            "🛒 Shop — buy potions/fireballs, buy pre-rolled gear by slot+tier, sell stash items, forge 3-of-a-kind into the next tier, and Work the Farm (wheat → gold at 10:1).",
            "⚔ Guilds — create or join a cooperative group. Membership is exclusive (one pubkey, one guild); leader auto-passes when they leave.",
            "🏆 Achievements — milestones, skills you've unlocked, forms you've been, World Boss progress, leaderboard.",
            "⚙ Settings — themes, sync cadence, auto-mission HP threshold, identity export / progress reset, advanced toggles + mailbox D2D test + debug overlay.",
            "❔ Help — this page.",
        ],
        shop_p1: "Gear is grouped by 8 slots × 4 tiers. Tiers 1-3 are buyable (100g/250g/600g); Tier 4 (Legendary) only drops or forges. Forge needs 3 copies of one item + tier-scaled essence to combine into a single piece of the next tier — your duplicates aren't trash, they're future legendaries.",
        shop_p2: "Auto-Equip Best walks every form-allowed slot and equips the highest stat-sum unequipped piece you own.",
        consumables_p1: "Potions (cost 50g) heal your HP to full. Fireballs (cost 200g) deal 50 flat damage to the shared World Boss. Drop rates: a potion every 13 wins, a fireball every 19.",
        world_boss_p1: "The World Boss has 500 HP in era 0, shared across every player connected to the contract. Each win contributes 1× to area-dependent the base. Once cumulative damage exceeds the era's HP, the boss respawns in the next era — bigger. Era scaling makes the gauge keep moving once dozens of players have chipped at it forever.",
        delegate_p1: "Everything that matters lives on your Freenet node, not in this browser tab. The delegate stores your Ed25519 identity, your inventory, every gear piece, every skill, your XP, your wheat, your shop counter, and your achievements — plus the active battle state so closing the tab pauses (not aborts) a fight. The browser is just a thin view.",
        delegate_p2: "To move identity to another node, use Settings → Export seed — it returns the 32-byte secret key; copy it once, paste on the new node. Reset progress wipes the inventory (gold, gear, skills, achievements) but keeps the pubkey — leaderboards still recognize you. To actually destroy the identity, wipe `~/.config/freenet/secrets/local/<delegate-key>/`.",
        guilds_p1: "The Guilds tab is the first cross-player interaction beyond the leaderboard: cooperative groups, one pubkey per guild, 50 members per guild. Create a new one or join existing — leader auto-handoff on leave. Gameplay layers (shared boss, member contributions) come later.",
        guilds_p2: "The Mailbox section in Settings → Advanced is the signed-log substrate for player-to-player messaging — gifts, invites, trade offers will plug in on top. Send a self-test message to verify the round-trip is working.",
        estate_p1: "Estate is the long-game income loop on the Farm tab. Hire workers in four tiers — Farmhand (wheat), Forager (more wheat), Trader (gold), Sage (essence) — each with a 1.07ⁿ price curve. Workers accrue passively while Estate is your selected idle action, capped at ~1 hour of catchup. Running Estate pauses auto-mission and blocks Run Mission: you pick what to be doing right now, the strategic axis is which idle loop to commit to.",
        estate_p2: "Form affinity stacks on top: the active Form buffs or penalises specific tiers (see Forms above). Legacy multipliers, once you start collecting stars, also compound multiplicatively. The Estate panel itself shows the live affinity per row so you can read the current bonus at a glance.",
        legacy_p1: "Legacy is the personal-prestige loop, surfaced in Settings → Legacy once you've earned your first star. You get one star every 5 levels (idempotent — the milestone watermark prevents re-grinding from zero after an Ascend). Spend stars on permanent multipliers (Hero Attack, Estate Yield, Mission Gold); cost curve doubles each level. Ascend is the optional soft-reset that wipes gold, gear, Estate, and the active battle while keeping stars, level, mission count, skills, and achievements — opt-in, never forced.",
        area_graph_p1: "World Map is a graph, not a list — areas unlock if you meet the level requirement AND have the clear-count in any one predecessor. Branches off Forest Road (Deep Forest, Eastern paths) and Mountain Pass (Snowfields) let you choose specialisation routes that match your Form. Each card's '↑ Predecessor' label shows which upstream area unlocked it; the row layout grows downward as new zones ship.",
    };

    pub const RU: Self = Self {
        loop_p1: "Нажми «В миссию» на вкладке Ферма, чтобы запустить цепочку до 5 сражений с врагами текущей области. Бой пошаговый: ход срабатывает раз в 700 мс, и ты можешь поставить «Использовать зелье» или «Использовать фаербол» в очередь прямо в бою, чтобы отреагировать на неудачный заход. Каждая победа даёт золото, эссенцию, опыт и чуть-чуть откусывает от общего Мирового Босса. Проиграешь — превратишься в победившего тебя врага.",
        loop_p2: "Включи «авто: вкл» — и делегат на узле продолжит проходить миссии даже после закрытия вкладки. Когда вернёшься, увидишь баннер «пока тебя не было» со сводкой (до ~1 часа симуляции за раз). Авто-режим ставится на паузу, когда ОЗ падают ниже порога из Настройки → авто-миссия.",
        stats_p1: "Уровень вычисляется из накопленного опыта. Требуемый опыт растёт в 1.5 раза за ступень (100, 150, 225, 337…). Базовые характеристики статичны: ОЗ = 20 + ур×5, Атака = 5 + ур×2, Защита = 5 + ур×2. Снаряжение, форма и навыки складываются сверху; ничего другого не влияет (золото/эссенция не перетекают в боевые статы).",
        stats_p2: "ОЗ убывают в бою и восстанавливаются со временем (полное восстановление за 180 секунд реального времени). Зелье моментально доводит ОЗ до максимума.",
        forms_p1: "Поражение не-обычному врагу навсегда превращает тебя в этого монстра. У каждой формы своя маска слотов: Слизь носит только Шлем + Кольцо, Кот — Шлем/Плащ/Сапоги/Кольцо и так далее. Характеристики подстраиваются под форму. Каждая форма, в которой ты побывал, оставляет постоянный Навык — даже после возврата в Человека бонусы сохраняются. Это и есть петля престижа.",
        forms_p2: "Форма ещё и определяет аффинити Поместья: пока ты в данной форме, работники подходящих звеньев приносят больше, остальные — меньше. Конь усиливает Работника + Собирателя, Дракон — Торговца + Мудреца, Кот — Собирателя + Мудреца, Слизь даёт +30% по всем звеньям, Человек нейтрален. В Магазине есть платная секция Формы — можно купить нужный облик напрямую, не дожидаясь поражения. Человек — дешёвый сброс, остальные четыре — стратегическое вложение.",
        tabs: &[
            "🛡 Ферма — герой, живая сцена боя (полоски ОЗ, кнопки очереди действий), сюжет, Мировой Босс, ресурсы.",
            "🗺 Карта мира — смена области фарма. Высокие области требуют уровня, но платят больше (или иначе — Лес даёт эссенцию, Гора — золото, Логово Босса — урон).",
            "🛒 Магазин — зелья и фаерболы, готовое снаряжение по слоту и тиру, продажа запаса, ковка 3-х одинаковых в следующий тир и работа на ферме (пшеница → золото в соотношении 10:1).",
            "⚔ Гильдии — создание или вступление в кооперативную группу. Членство эксклюзивно (один ключ — одна гильдия); лидер автоматически передаётся при уходе.",
            "🏆 Достижения — вехи, открытые навыки, посещённые формы, прогресс Мирового Босса, таблица лидеров.",
            "⚙ Настройки — темы, частота синхронизации, порог авто-миссии, экспорт личности / сброс прогресса, продвинутые переключатели + тест почты D2D + диагностика.",
            "❔ Помощь — эта страница.",
        ],
        shop_p1: "Снаряжение делится на 8 слотов × 4 тира. Тиры 1–3 продаются (100/250/600 зол); тир 4 (Легендарный) выпадает или куётся. Ковка требует 3 копии одного предмета + эссенцию по тиру и собирает их в один предмет следующего тира — дубликаты не мусор, а будущие легендарки.",
        shop_p2: "«Лучшее снаряжение» проходит по всем разрешённым слотам формы и одевает предмет с наибольшей суммой характеристик.",
        consumables_p1: "Зелья (50 зол) полностью восстанавливают ОЗ. Фаерболы (200 зол) наносят 50 фиксированного урона общему Мировому Боссу. Дроп-рейты: зелье каждые 13 побед, фаербол каждые 19.",
        world_boss_p1: "У Мирового Босса 500 ОЗ в эре 0, общие для всех игроков, подключённых к контракту. Каждая победа добавляет урон с коэффициентом области. Когда суммарный урон превышает ОЗ эры, босс перерождается в следующей эре — крупнее прежнего. Масштабирование по эрам поддерживает движение шкалы, даже когда десятки игроков непрерывно лупят по боссу.",
        delegate_p1: "Всё значимое живёт на твоём узле Freenet, а не во вкладке браузера. Делегат хранит Ed25519-личность, инвентарь, каждое снаряжение, каждый навык, опыт, пшеницу, счётчик магазина и достижения — а также состояние активного боя, поэтому закрытие вкладки ставит бой на паузу, а не отменяет его. Браузер — лишь тонкий просмотрщик.",
        delegate_p2: "Чтобы перенести личность на другой узел, открой Настройки → Экспорт ключа — вернётся 32-байтовый секретный ключ; скопируй один раз и вставь на новом узле. «Сброс прогресса» обнуляет инвентарь (золото, снаряжение, навыки, достижения), но сохраняет публичный ключ — таблица лидеров продолжит тебя узнавать. Чтобы полностью уничтожить личность, удали `~/.config/freenet/secrets/local/<delegate-key>/`.",
        guilds_p1: "Вкладка Гильдии — первое взаимодействие игроков за пределами таблицы лидеров: кооперативные группы, один ключ на гильдию, до 50 участников. Создай новую или вступи в существующую — лидер передаётся автоматически при уходе. Игровые механики (общий босс, вклад участников) появятся позже.",
        guilds_p2: "Секция Почта в Настройки → Продвинутые — это лог с подписями, на котором будут строиться сообщения между игроками: подарки, приглашения, обменные предложения. Отправь тестовое сообщение себе, чтобы убедиться, что цикл работает.",
        estate_p1: "Поместье — это долгая петля пассивного дохода на вкладке Ферма. Нанимай работников в четырёх звеньях: Работник (пшеница), Собиратель (больше пшеницы), Торговец (золото), Мудрец (эссенция) — цена растёт как 1.07ⁿ. Доход капает, пока Поместье выбрано как активное простойное действие; кап — около часа симуляции при возврате. Поместье ставит на паузу авто-миссию и блокирует Run Mission: ты сам выбираешь, чем заниматься прямо сейчас. Стратегическая ось — на какую петлю ты сел.",
        estate_p2: "Аффинити формы накладывается сверху: текущая форма усиливает или ослабляет конкретные звенья (см. Формы выше). Множители Наследия, когда начнёшь собирать звёзды, тоже компонуются мультипликативно. На самой панели Поместья показано живое аффинити в каждой строке, чтобы текущий бонус читался с одного взгляда.",
        legacy_p1: "Наследие — личная петля престижа, появляется в Настройки → Наследие после первой полученной звезды. Звезда даётся каждые 5 уровней (идемпотентно — водяной знак не даёт переграбить звёзды повторно после Вознесения). Звёзды тратятся на постоянные множители (Атака, Доход Поместья, Золото за миссии); цена удваивается с каждым уровнем. Вознестись — необязательный мягкий сброс: золото, экипировка, Поместье и текущий бой обнуляются, но звёзды, уровень, счётчик миссий, навыки и достижения остаются. Чисто опциональная механика, никто не заставляет.",
        area_graph_p1: "Карта мира — граф, а не список: новая область открывается, если уровень соответствует требованию И в любой из предшествующих областей набран нужный счёт зачисток. От Лесной дороги ответвляются Глубокий лес и восточные тропы, от Горного перевала — Снежные равнины: можно выбрать маршрут под свою форму. Подпись «↑ Предшественник» под линией соединения указывает, откуда открыли область; ряд за рядом граф растёт вниз по мере добавления зон.",
    };

    pub const FR: Self = Self {
        loop_p1: "Cliquez sur Lancer la mission dans l'onglet Ferme pour démarrer une chaîne pouvant compter jusqu'à 5 rencontres contre les ennemis de la zone actuelle. Le combat est en tours : un tour se déclenche toutes les 700 ms, et vous pouvez mettre en file d'attente Utiliser une potion ou Utiliser une boule de feu en plein combat pour réagir à une mauvaise série. Chaque victoire octroie de l'or, de l'essence, de l'XP et entame le Boss du Monde partagé. Si vous perdez, vous vous transformez en l'ennemi qui vous a vaincu.",
        loop_p2: "Activez auto : le délégué côté nœud continue à lancer des missions même après la fermeture de l'onglet. Vous reviendrez sur une bannière pendant votre absence résumant les événements (plafonnée à ~1 h de rattrapage). L'auto se met en pause quand les PV passent sous le seuil défini dans Paramètres → mission auto.",
        stats_p1: "Votre Niveau découle de l'XP cumulée. L'XP par niveau augmente d'un facteur 1,5 à chaque palier (100, 150, 225, 337, …). Les stats de base sont statiques par niveau : PV = 20 + niv×5, Attaque = 5 + niv×2, Défense = 5 + niv×2. L'équipement, la forme et les compétences s'ajoutent par-dessus ; rien d'autre (pas de fuite d'or ou d'essence dans les stats).",
        stats_p2: "Les PV diminuent en combat et se régénèrent avec le temps (régen complète en 180 s en temps réel). Utilisez une Potion pour les remplir instantanément.",
        forms_p1: "Perdre un combat contre un ennemi non-ordinaire vous transforme définitivement en ce monstre. Chaque forme a son propre masque de slots équipables : un Slime ne peut porter que Casque + Anneau, un Chat garde Casque/Cape/Bottes/Anneau, etc. Les stats s'adaptent à la forme. Chaque forme que vous avez touchée laisse une Compétence permanente — même après être redevenu Humain, ces bonus restent. C'est la boucle de prestige.",
        forms_p2: "Les formes pilotent aussi l'affinité du Domaine : tant que vous êtes dans une forme donnée, vos travailleurs des paliers correspondants produisent plus, tandis que les paliers non correspondants produisent moins. Le Cheval favorise Ouvrier + Cueilleur, le Dragon Marchand + Sage, le Chat Cueilleur + Sage, le Slime offre un +30 % uniforme, l'Humain est neutre. L'onglet Boutique a une section Formes payante pour choisir une apparence directement sans attendre une défaite — l'Humain est le reset bon marché, les quatre autres sont un engagement stratégique.",
        tabs: &[
            "🛡 Ferme — votre héros, la scène de combat en direct (barres de PV, boutons de file d'actions), intrigue, Boss du Monde, ressources brutes.",
            "🗺 Carte du monde — changer de zone de farming. Les zones supérieures ont une condition de niveau mais paient davantage (ou différemment — la Forêt est riche en essence, la Montagne en or, l'Antre du Boss en dégâts).",
            "🛒 Boutique — acheter potions/boules de feu, acheter de l'équipement préfabriqué par slot+palier, vendre la réserve, forger 3 doubles dans le palier supérieur, et Travailler à la Ferme (blé → or à 10:1).",
            "⚔ Guildes — créer ou rejoindre un groupe coopératif. L'adhésion est exclusive (une clé publique, une guilde) ; le chef se transmet automatiquement à son départ.",
            "🏆 Succès — jalons, compétences débloquées, formes traversées, progression du Boss du Monde, classement.",
            "⚙ Paramètres — thèmes, cadence de synchronisation, seuil de mission auto, export d'identité / reset de progression, options avancées + test de boîte D2D + diagnostic.",
            "❔ Aide — cette page.",
        ],
        shop_p1: "L'équipement est groupé par 8 slots × 4 paliers. Les paliers 1-3 sont achetables (100/250/600 or) ; le palier 4 (Légendaire) ne tombe ou ne se forge. La forge demande 3 copies d'un objet + essence proportionnelle au palier pour fusionner en une pièce du palier supérieur — vos doublons ne sont pas déchet, ce sont de futurs légendaires.",
        shop_p2: "Auto-Équiper Meilleur parcourt chaque slot autorisé par la forme et équipe la pièce non équipée à la plus haute somme de stats que vous possédez.",
        consumables_p1: "Les Potions (50 or) restaurent vos PV au maximum. Les Boules de feu (200 or) infligent 50 dégâts fixes au Boss du Monde partagé. Taux de drop : une potion toutes les 13 victoires, une boule de feu toutes les 19.",
        world_boss_p1: "Le Boss du Monde a 500 PV en ère 0, partagés entre tous les joueurs connectés au contrat. Chaque victoire contribue à hauteur de la base dépendante de la zone. Une fois que les dégâts cumulés dépassent les PV de l'ère, le boss renaît dans l'ère suivante — plus gros. La mise à l'échelle par ère maintient le mouvement de la jauge même quand des dizaines de joueurs frappent en continu.",
        delegate_p1: "Tout ce qui compte vit sur votre nœud Freenet, pas dans cet onglet. Le délégué stocke votre identité Ed25519, votre inventaire, chaque pièce d'équipement, chaque compétence, votre XP, votre blé, votre compteur de boutique et vos succès — ainsi que l'état du combat actif, si bien que fermer l'onglet met en pause (pas en abandon) un combat. Le navigateur n'est qu'une vue mince.",
        delegate_p2: "Pour transférer l'identité vers un autre nœud, utilisez Paramètres → Exporter la seed — elle renvoie la clé secrète de 32 octets ; copiez-la une fois, collez-la sur le nouveau nœud. Réinitialiser la progression efface l'inventaire (or, équipement, compétences, succès) mais conserve la clé publique — les classements vous reconnaissent toujours. Pour détruire l'identité elle-même, effacez `~/.config/freenet/secrets/local/<delegate-key>/`.",
        guilds_p1: "L'onglet Guildes est la première interaction inter-joueurs au-delà du classement : groupes coopératifs, une clé par guilde, 50 membres par guilde. Créez-en une nouvelle ou rejoignez une existante — passation automatique de chef au départ. Les couches de gameplay (boss partagé, contributions des membres) arrivent plus tard.",
        guilds_p2: "La section Boîte aux lettres dans Paramètres → Avancé est le substrat de log signé pour la messagerie entre joueurs — cadeaux, invitations, offres d'échange se grefferont dessus. Envoyez un message de test à vous-même pour vérifier que l'aller-retour fonctionne.",
        estate_p1: "Le Domaine est la boucle de revenu à long terme dans l'onglet Ferme. Embauchez des travailleurs en quatre paliers — Ouvrier (blé), Cueilleur (plus de blé), Marchand (or), Sage (essence) — chacun avec une courbe de prix en 1,07ⁿ. Les travailleurs accumulent passivement tant que Domaine est votre action passive sélectionnée, plafonné à ~1 h de rattrapage. Lancer Domaine met en pause la mission auto et bloque Lancer la mission : vous choisissez quoi faire maintenant, l'axe stratégique étant à quelle boucle passive vous engager.",
        estate_p2: "L'affinité de forme se cumule par-dessus : la Forme active favorise ou pénalise des paliers spécifiques (voir Formes ci-dessus). Les multiplicateurs d'Héritage, une fois que vous commencez à collecter des étoiles, se composent aussi multiplicativement. Le panneau Domaine lui-même affiche l'affinité vivante par ligne, pour lire le bonus actuel d'un coup d'œil.",
        legacy_p1: "L'Héritage est la boucle de prestige personnelle, visible dans Paramètres → Héritage une fois votre première étoile gagnée. Vous obtenez une étoile tous les 5 niveaux (idempotent — la marque haute du palier empêche de re-grinder à zéro après une Ascension). Dépensez les étoiles en multiplicateurs permanents (Attaque, Rendement du Domaine, Or de Mission) ; le coût double à chaque niveau. Ascension est le soft-reset optionnel qui efface or, équipement, Domaine et combat actif tout en gardant étoiles, niveau, compteur de missions, compétences et succès — opt-in, jamais forcé.",
        area_graph_p1: "La Carte du monde est un graphe, pas une liste — les zones se débloquent si vous atteignez le niveau requis ET avez le nombre de nettoyages dans n'importe quel prédécesseur. Les embranchements depuis la Route de la Forêt (Forêt profonde, chemins de l'Est) et le Col de la Montagne (Plaines enneigées) vous laissent choisir des spécialisations adaptées à votre Forme. L'étiquette « ↑ Prédécesseur » de chaque carte indique quelle zone en amont l'a débloquée ; la disposition des rangées s'étend vers le bas à mesure que de nouvelles zones arrivent.",
    };

    pub const ES: Self = Self {
        loop_p1: "Pulsa Lanzar misión en la pestaña Granja para iniciar una cadena de hasta 5 encuentros contra los enemigos de la zona actual. El combate es por turnos: un turno se dispara cada 700 ms, y puedes encolar Usar poción o Usar bola de fuego en mitad de la pelea para reaccionar a una mala racha. Cada victoria otorga oro, esencia, XP y mella al Jefe del Mundo compartido. Si pierdes, te transformas en el enemigo que te venció.",
        loop_p2: "Activa auto: el delegado del nodo sigue lanzando misiones incluso después de cerrar la pestaña. Volverás a un banner mientras estabas fuera que resume lo ocurrido (limitado a ~1 h de catch-up). El auto se pausa cuando los PV bajan del umbral elegido en Ajustes → misión auto.",
        stats_p1: "Tu Nivel viene del XP acumulado. El XP por nivel sube 1,5× en cada paso (100, 150, 225, 337, …). Las estadísticas base son estáticas por nivel: PV = 20 + niv×5, Ataque = 5 + niv×2, Defensa = 5 + niv×2. Equipo, forma y habilidades suman encima; nada más (sin filtración de oro/esencia).",
        stats_p2: "Los PV bajan en combate y se regeneran con el tiempo (regen total en 180 s reales). Usa una Poción para rellenarlos al instante.",
        forms_p1: "Perder un combate contra un enemigo no-mundano te transforma permanentemente en ese monstruo. Cada forma tiene su máscara de slots: el Limo solo lleva Casco + Anillo, el Gato conserva Casco/Capa/Botas/Anillo, etc. Las estadísticas se ajustan a la forma. Cada forma por la que pasaste deja una Habilidad permanente — incluso al volver a Humano, esos bonos se mantienen. Es el bucle de prestigio.",
        forms_p2: "Las formas también guían la afinidad de la Finca: mientras estás en una forma, tus trabajadores de los grados correspondientes producen más y los no correspondientes producen menos. Caballo potencia Bracero + Recolector, Dragón potencia Mercader + Sabio, Gato potencia Recolector + Sabio, Limo da un +30 % plano y Humano es neutral. La pestaña Tienda tiene una sección Formas de pago para escoger un cuerpo directamente sin esperar a una derrota — Humano es el reset barato, los otros cuatro son una decisión estratégica.",
        tabs: &[
            "🛡 Granja — tu héroe, la escena de combate en directo (barras de PV, botones de cola de acciones), trama, Jefe del Mundo, recursos brutos.",
            "🗺 Mapa del mundo — cambia de zona de farmeo. Las zonas superiores tienen un umbral de nivel pero pagan más (o distinto — Bosque rinde esencia, Montaña rinde oro, Guarida del Jefe rinde daño).",
            "🛒 Tienda — compra pociones/bolas de fuego, equipo prefabricado por slot+grado, vende el alijo, forja 3 iguales al siguiente grado, y Trabaja la Granja (trigo → oro en 10:1).",
            "⚔ Gremios — crea o únete a un grupo cooperativo. La pertenencia es exclusiva (una clave, un gremio); el liderazgo se traspasa al irse.",
            "🏆 Logros — hitos, habilidades desbloqueadas, formas visitadas, progreso del Jefe del Mundo, clasificación.",
            "⚙ Ajustes — temas, cadencia de sincronización, umbral de misión auto, exportar identidad / resetear progreso, opciones avanzadas + test de buzón D2D + diagnóstico.",
            "❔ Ayuda — esta página.",
        ],
        shop_p1: "El equipo se agrupa en 8 slots × 4 grados. Los grados 1-3 son comprables (100/250/600 oro); el grado 4 (Legendario) solo cae o se forja. La forja necesita 3 copias de un objeto + esencia proporcional al grado para combinarlos en una pieza del siguiente grado — tus duplicados no son basura, son legendarios en potencia.",
        shop_p2: "Auto-Equipar Lo Mejor recorre cada slot permitido por la forma y equipa la pieza no equipada con la mayor suma de estadísticas que poseas.",
        consumables_p1: "Las Pociones (50 oro) curan los PV al máximo. Las Bolas de fuego (200 oro) infligen 50 de daño plano al Jefe del Mundo compartido. Tasas de drop: una poción cada 13 victorias, una bola de fuego cada 19.",
        world_boss_p1: "El Jefe del Mundo tiene 500 PV en la era 0, compartidos entre todos los jugadores conectados al contrato. Cada victoria contribuye a la base dependiente de la zona. Cuando el daño acumulado supera los PV de la era, el jefe renace en la siguiente era — más grande. El escalado por era mantiene el medidor en movimiento aunque docenas de jugadores le golpeen sin parar.",
        delegate_p1: "Todo lo que importa vive en tu nodo Freenet, no en esta pestaña. El delegado guarda tu identidad Ed25519, tu inventario, cada pieza de equipo, cada habilidad, tu XP, tu trigo, tu contador de tienda y tus logros — más el estado del combate activo, así que cerrar la pestaña pausa (no aborta) la pelea. El navegador solo es una vista delgada.",
        delegate_p2: "Para mover la identidad a otro nodo, usa Ajustes → Exportar seed — devuelve la clave secreta de 32 bytes; cópiala una vez y pégala en el nuevo nodo. Resetear progreso borra el inventario (oro, equipo, habilidades, logros) pero mantiene la clave pública — las clasificaciones siguen reconociéndote. Para destruir la identidad de verdad, borra `~/.config/freenet/secrets/local/<delegate-key>/`.",
        guilds_p1: "La pestaña Gremios es la primera interacción entre jugadores más allá de la clasificación: grupos cooperativos, una clave por gremio, 50 miembros por gremio. Crea uno nuevo o únete a uno existente — traspaso automático del líder al irse. Las capas de juego (jefe compartido, contribuciones de miembros) llegarán después.",
        guilds_p2: "La sección Buzón en Ajustes → Avanzado es el sustrato de log firmado para mensajería entre jugadores — regalos, invitaciones, ofertas de intercambio se enchufarán encima. Envíate un mensaje de prueba a ti mismo para verificar que el ciclo funciona.",
        estate_p1: "La Finca es el bucle de ingresos a largo plazo en la pestaña Granja. Contrata trabajadores en cuatro grados — Bracero (trigo), Recolector (más trigo), Mercader (oro), Sabio (esencia) — cada uno con una curva de precio 1,07ⁿ. Los trabajadores acumulan pasivamente mientras la Finca sea tu acción pasiva seleccionada, con tope ~1 h de catch-up. Ejecutar la Finca pausa la misión auto y bloquea Lanzar misión: eliges qué hacer ahora, el eje estratégico es qué bucle pasivo tomar.",
        estate_p2: "La afinidad de forma se apila encima: la Forma activa potencia o penaliza grados específicos (ver Formas arriba). Los multiplicadores de Legado, una vez empieces a coleccionar estrellas, también se componen multiplicativamente. El panel de Finca muestra la afinidad viva por fila, para leer el bono actual de un vistazo.",
        legacy_p1: "El Legado es el bucle de prestigio personal, en Ajustes → Legado una vez ganada la primera estrella. Recibes una estrella cada 5 niveles (idempotente — la marca de agua del hito impide re-grindear desde cero tras una Ascensión). Gasta estrellas en multiplicadores permanentes (Ataque, Rendimiento de la Finca, Oro de Misión); el coste se duplica con cada nivel. Ascender es el soft-reset opcional que borra oro, equipo, Finca y combate activo manteniendo estrellas, nivel, contador de misiones, habilidades y logros — opt-in, nunca forzado.",
        area_graph_p1: "El Mapa del mundo es un grafo, no una lista — las zonas se desbloquean si cumples el nivel Y tienes el conteo de limpiezas en algún predecesor. Las ramificaciones desde la Senda del Bosque (Bosque Profundo, sendas del Este) y el Paso de Montaña (Llanuras Nevadas) te dejan elegir rutas de especialización que casan con tu Forma. La etiqueta «↑ Predecesor» en cada carta indica qué zona la desbloqueó; la disposición de filas crece hacia abajo según se añaden zonas.",
    };

    pub const JA: Self = Self {
        loop_p1: "ファームタブの「ミッション開始」をクリックすると、現在のエリアの敵と最大 5 戦の連戦が始まります。戦闘はティック制で、700ms ごとに 1 ターン進行し、戦闘中に「ポーション使用」や「ファイアボール使用」をキューに入れて運の悪い展開に対応できます。勝利ごとに金、精、XP を獲得し、共有のワールドボスを少しずつ削ります。負けると、倒した敵にあなたが変身します。",
        loop_p2: "「自動: オン」を切り替えると、タブを閉じた後もノード側のデリゲートがミッションを実行し続けます。戻ると「離席中の出来事」バナーで概要が見られます（最大約 1 時間まで巻き戻し）。HP が「設定 → 自動ミッション」で指定したしきい値を下回ると、自動は一時停止します。",
        stats_p1: "レベルは累積 XP から決まります。レベルごとの必要 XP は段階ごとに 1.5 倍ずつ増えます（100, 150, 225, 337, …）。基本ステータスはレベルごとに静的です: HP = 20 + Lv×5、攻撃 = 5 + Lv×2、防御 = 5 + Lv×2。装備、フォーム、スキルが上乗せされ、それ以外（金/精から漏れたボーナス）は加算されません。",
        stats_p2: "HP は戦闘で減り、時間経過で回復します（実時間 180 秒で全回復）。ポーションを使うと即座に満タンになります。",
        forms_p1: "通常でない敵に戦闘で負けると、その敵に永続的に変身します。各フォームには独自の装備スロットマスクがあります: スライムは兜と指輪のみ、猫は兜・マント・ブーツ・指輪を保持、など。ステータスもフォームに合わせて変化します。一度でも経験したフォームは恒久スキルを残し、人間に戻ってもボーナスは継続します。これがプレステージのループです。",
        forms_p2: "フォームは領地のアフィニティも決めます: そのフォームでいる間、一致するティアの労働者は増産し、一致しないティアは減産します。馬は農夫＋採取者を強化、ドラゴンは商人＋賢者を強化、猫は採取者＋賢者を強化、スライムは全段に一律 +30%、人間は中立です。ショップのフォーム（有料）から、敗北を待たずに直接フォームを選べます。人間は安価なリセット、他の 4 つは戦略的なコミットメントです。",
        tabs: &[
            "🛡 ファーム — あなたの英雄、ライブ戦闘画面（HP バー、アクションキュー）、ストーリー、ワールドボス、原材料。",
            "🗺 ワールドマップ — 狩り場を変更。上位エリアはレベル要件があるが報酬が多い（あるいは異なる — 森は精、山は金、ボスの巣はダメージが豊富）。",
            "🛒 ショップ — ポーション/ファイアボール購入、スロット+ティア別の既製装備の購入、ストック売却、3 個同種からの上位ティアへの鍛造、農場作業（小麦 → 金 10:1）。",
            "⚔ ギルド — 協力グループを作成または参加。所属は排他的（1 公開鍵に 1 ギルド）、リーダーは離脱時に自動で引き継がれます。",
            "🏆 実績 — マイルストーン、解放スキル、訪れたフォーム、ワールドボスの進捗、リーダーボード。",
            "⚙ 設定 — テーマ、同期頻度、自動ミッション HP しきい値、ID 書き出し / 進行リセット、詳細トグル + D2D メールテスト + 診断オーバーレイ。",
            "❔ ヘルプ — このページ。",
        ],
        shop_p1: "装備は 8 スロット × 4 ティアに分かれます。ティア 1〜3 は購入可能（100/250/600 金）。ティア 4（伝説）はドロップか鍛造のみ。鍛造は同じアイテム 3 個 + ティアに応じた精 で次のティアの 1 個に統合します — 重複は無駄ではなく、未来の伝説です。",
        shop_p2: "「最良を自動装備」はフォーム許可のスロットをすべて走査し、所持中で未装備のステータス合計最大のものを装備します。",
        consumables_p1: "ポーション（50 金）は HP を全回復します。ファイアボール（200 金）は共有ワールドボスに固定 50 ダメージを与えます。ドロップ率: ポーションは 13 勝ごと、ファイアボールは 19 勝ごと。",
        world_boss_p1: "ワールドボスはエラ 0 で 500 HP、契約に接続したすべてのプレイヤーで共有されます。各勝利はエリア依存のベースで寄与します。累積ダメージがエラの HP を超えると、ボスは次のエラに再出現し、より強くなります。エラのスケーリングにより、数十人のプレイヤーが叩き続けてもゲージは動き続けます。",
        delegate_p1: "重要なものはすべてあなたの Freenet ノードに存在し、このブラウザタブにはありません。デリゲートは Ed25519 ID、インベントリ、装備、スキル、XP、小麦、ショップカウンター、実績 — そして現在の戦闘状態を保持するので、タブを閉じても戦闘は中断されず一時停止になります。ブラウザは薄いビューに過ぎません。",
        delegate_p2: "ID を別のノードに移すには、設定 → シード書き出し を使ってください — 32 バイトの秘密鍵が返るので、一度コピーし新しいノードに貼り付けます。「進行リセット」はインベントリ（金、装備、スキル、実績）を消去しますが公開鍵は残ります — リーダーボードはあなたを認識し続けます。ID を完全に破棄するには `~/.config/freenet/secrets/local/<delegate-key>/` を削除してください。",
        guilds_p1: "ギルドタブはリーダーボード以外で初めての対人インタラクションです: 協力グループ、1 公開鍵につき 1 ギルド、最大 50 名。新規作成または既存への参加が可能 — リーダーは離脱時に自動で引き継がれます。ゲームプレイ層（共有ボス、メンバー貢献）は今後追加されます。",
        guilds_p2: "設定 → 詳細 内のメールボックスは、プレイヤー間メッセージ（ギフト、招待、トレード提案）の基盤となる署名付きログです。自分宛にテストメッセージを送って往復を確認できます。",
        estate_p1: "領地はファームタブの長期収入ループです。4 ティアの労働者を雇います — 農夫（小麦）、採取者（小麦増）、商人（金）、賢者（精） — 価格は 1.07ⁿ で上昇。領地が選択された放置アクションのとき、労働者は受動的に蓄積し、復帰時は最大約 1 時間まで巻き戻ります。領地稼働中は自動ミッションが一時停止し「ミッション開始」もブロックされます: 今何をするかを選ぶ、放置ループの戦略軸です。",
        estate_p2: "フォームのアフィニティが上乗せされます: 現在のフォームが特定のティアを強化または弱体化します（上の「フォーム」参照）。スターを集め始めるとレガシー乗数も乗算で積み重なります。領地パネルは行ごとに現アフィニティを表示するので、現在のボーナスを一目で読めます。",
        legacy_p1: "レガシーは個人プレステージのループで、最初のスターを得てから 設定 → レガシー に表示されます。スターは 5 レベルごとに獲得します（冪等 — 高水位マークが昇華後のゼロ再ファームを防ぎます）。スターは永続乗数（攻撃、領地生産、ミッション金）に消費でき、コストはレベルごとに倍増します。昇華は任意のソフトリセットで、金・装備・領地・現戦闘を消去しつつスター、レベル、ミッション数、スキル、実績を保持します — オプトイン、強制ではありません。",
        area_graph_p1: "ワールドマップはリストではなくグラフです — 各エリアは、レベル要件を満たし、かつ任意の前提エリアでの突破回数を満たすと解放されます。森の道（深い森、東の道）と山道（雪原）の分岐により、フォームに合わせた特化ルートを選べます。各カードの「↑ 前提」ラベルは、どの上流エリアから解放されたかを示します — 新しいエリアが追加されるにつれて段は下方向に伸びていきます。",
    };
}

/// Every translatable string in the UI. Adding a variant forces the
/// `tr` match to grow both arms — that's the compile-time check that
/// keeps the translation matrix complete.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
pub enum MessageId {
    // Boot-time loader shown while the delegate's `LoadUiPrefs` reply is
    // in flight; once that lands the main UI takes over.
    BootLoading,

    // Connection status.
    StatusAskingDelegate,
    StatusRegisteringDelegate,
    StatusSubscribing,

    // Tabs.
    TabFarm,
    TabWorldMap,
    TabShop,
    TabGuilds,
    TabAchievements,
    TabMastery,
    TabSettings,
    TabHelp,

    // Status pills (rendered in uppercase by the existing CSS).
    PillDefeated,
    PillAdventuring,
    PillFocusing,
    PillRecovering,
    PillReady,
    PillEstate,

    // Settings tab section headers.
    SettingsTitle,
    SettingsTheme,
    SettingsLanguage,
    SettingsSyncCadence,
    SettingsAutoMission,
    SettingsPublishBehavior,
    SettingsIdentityBackup,
    SettingsAdvanced,
    SettingsResetUiPrefs,
    SettingsMailbox,
    SettingsWhereStateLives,

    // Locale picker buttons.
    LocaleEnglish,
    LocaleRussian,

    // Action buttons.
    BtnExportSeed,
    BtnResetProgress,
    BtnHide,
    BtnResetDefaults,
    BtnSendTestSelf,

    // Header chrome.
    SourceLink,

    // Panel / section headers.
    PanelHero,
    PanelEquipment,
    PanelConsumables,
    PanelResources,
    PanelShop,
    PanelBuyGear,
    PanelSage,
    PanelFarm,
    PanelWorldMap,
    PanelWorldBoss,
    PanelPlotSoFar,
    PanelGuilds,
    PanelCreateGuild,
    PanelTutorialWelcome,
    PanelWhileAway,
    PanelEndings,
    PanelSkillsLine,
    PanelFormsVisited,
    PanelAchievementsLow,
    PanelHowToPlay,

    // Stat / table column names.
    StatName,
    StatForm,
    StatLevel,
    StatXp,
    StatHp,
    StatAttack,
    StatDefence,
    StatSpeed,
    StatEvasion,
    ResGold,
    ResEssence,
    ResMissions,
    ResBossDamage,
    ResPotions,
    ResFireballs,
    ColSlot,
    ColName,
    ColDamage,
    ColArea,
    ColSeen,

    // Action buttons (main UI).
    BtnRunMission,
    BtnAutoOn,
    BtnAutoOff,
    BtnAutoEquipBest,
    BtnUse,
    BtnBuy,
    BtnWorkFarm,
    BtnSellAllWheat,
    BtnCreate,
    BtnLeaveGuild,
    BtnDisbandGuild,
    BtnJoin,
    BtnEquip,
    BtnNext,
    BtnStartPlaying,
    BtnSkipIntro,

    // Consumable / item names.
    ItemPotion,
    ItemFireball,

    // Common micro-strings.
    TermYouBattle,
    TermYouBadge,
    TermYouLeader,
    TermLive,
    TermActive,
    TermOwned,
    TermMaxTier,
    TermEmpty,
    TermFormNa,
    TermFormLocks,
    TermNever,
    TermWin,
    TermDefeat,
    TermPubkeyHidden,
    TermPubkeyPending,
    TermPubkeyPendingShort,

    // Onboarding wizard (4 steps × {title, body lines}).
    OnbTitleWelcome,
    OnbBodyWelcome1,
    OnbBodyWelcome2,
    OnbTitleLoop,
    OnbBodyLoop1,
    OnbBodyLoop2,
    OnbTitleAuto,
    OnbBodyAuto1,
    OnbBodyAuto2,
    OnbTitleTabs,
    OnbBodyTabs1,
    OnbBodyTabs2,

    // Welcome tutorial banner on Farm tab.
    TutorialBody1,
    TutorialBody2,

    // Battle-queue / combat-history hints.
    BattleOpeningTurn,
    BattleNoEncounters,
    BattlePotionQueued,
    BattleFireballQueued,
    BattleMissed,

    // Mailbox panel labels.
    MailboxEmpty,
    MailboxKindChat,
    MailboxKindGift,
    MailboxKindGuildInvite,
    MailboxKindTradeOffer,

    // Catch-up banner.
    CatchupClearsHint,

    // Help-tab subheaders (h3).
    HelpTheLoop,
    HelpStats,
    HelpFormsTransformation,
    HelpTabs,
    HelpShopGear,
    HelpConsumables,
    HelpWorldBoss,
    HelpDelegateWhat,
    HelpGuildsMailbox,
    HelpEstate,
    HelpLegacy,
    HelpAreaGraph,

    // Estate panel (B2).
    PanelEstate,
    EstateBtnPause,
    EstateBtnRun,
    EstateColTier,
    EstateColOwned,
    EstateColYield,
    EstateColNextPrice,
    BtnHire,
    EstateResWheat,
    EstateResGold,
    EstateResEssence,

    // Legacy / Epoch panel (C1). `BtnBuy` is defined earlier in
    // the enum (shared with shop usages) so it isn't re-declared
    // here.
    PanelLegacy,
    LegacyColNode,
    LegacyColLevel,
    LegacyColMultiplier,
    LegacyColNextCost,
    BtnAscend,
    LegacyAscendBlurb,
    LegacyAscendConfirm,

    // Catchup / patchnotes modal (B4).
    CatchupModalTitle,
    BtnGotIt,
    NewerBuildDesc,

    // Forms shop (#40).
    PanelFormsShop,
    FormsShopDesc,
    FormsShopBaselineDesc,
    TipFormAlreadyActive,

    // Per-zone activities (A1) + Routine (B1) + Insight (B5) +
    // Boss attack (C1) + Tokens (C2).
    PanelActivities,
    ActivitiesDesc,
    ActivityStart,
    ActivityStop,
    PanelRoutine,
    RoutineDesc,
    RoutineColTier,
    RoutineColCurrent,
    RoutineColTarget,
    PanelInsight,
    InsightDesc,
    InsightColNode,
    InsightColLevel,
    InsightColNextCost,
    PanelBossAttack,
    BossAttackBtn,
    BossAttackDesc,
    BossAttackLocked,
    PanelTokens,
    TokensDesc,
    TokenColPerk,
    TokenColPrice,
    BtnUnlock,
    ResInsight,
    ResTokens,
    MasteryIntro,
    PanelWilds,
    WildsDesc,
    MapViewLinear,
    MapViewWilds,

    // Settings descriptive paragraphs (long copy that lives next to
    // each h3). Several read as inline fragments next to a <strong>
    // (BodyStrong, BodyTail) so they can be reassembled in Yew.
    SettingsThemeDesc,
    SettingsCadenceDesc,
    SettingsAutoMissionDesc,
    SettingsPublishCheckbox,
    SettingsIdentityBody,
    SettingsIdentityBodyStrong,
    SettingsIdentityBodyTail,
    SettingsAdvancedDesc,
    SettingsHidePubkey,
    SettingsHideStale,
    SettingsWsOverride,
    SettingsResetUiPrefsDesc,
    SettingsWhereStateBody,
    SettingsSeedRevealWarn,

    // Guilds descriptive copy.
    GuildsPanelDesc,
    GuildsContractMissing,
    GuildsContractMissingTail,
    GuildsEmptyList,
    GuildsViaScript,
    GuildNamePlaceholder,
    MailboxNotConfiguredHead,
    MailboxNotConfiguredVia,
    MailboxNotConfiguredTail,
    MailboxNotConfiguredIn,

    // Shop descriptive copy.
    ShopStashDesc,
    ShopBuyGearDesc,
    ShopSageDesc,
    ShopFarmDesc,
    ShopFarmDescPassive,

    // Hover tooltips on buttons / icons.
    TipFightInProgress,
    TipAutoToggleMidFight,
    TipAutoEquipBest,
    TipAutoEquipNothing,
    TipEstateBlocksCombat,
    TipPotionQueue,
    TipPotionIdle,
    TipFireballQueue,
    TipUnequipSlot,
    TipDisbandLeader,
    PotionShopDesc,
    TermCorrupt,
}

/// Map a `Locale` to the lowercase short code used by the locale
/// picker `data-*` attribute (matches the `Themes` pattern). Useful
/// when emitting onclick callbacks keyed by string id.
pub fn locale_code(l: Locale) -> &'static str {
    match l {
        Locale::En => "en",
        Locale::Ru => "ru",
        Locale::De => "de",
        Locale::Fr => "fr",
        Locale::Es => "es",
        Locale::Ja => "ja",
    }
}

/// Parse a short code back to `Locale`. Unknown codes fall through
/// to `En` — same defensiveness as `apply_theme` on a missing
/// `data-theme` attribute.
pub fn locale_from_code(code: &str) -> Locale {
    match code {
        "ru" => Locale::Ru,
        "de" => Locale::De,
        "fr" => Locale::Fr,
        "es" => Locale::Es,
        "ja" => Locale::Ja,
        _ => Locale::En,
    }
}

/// Detect a sensible default locale on first load by reading
/// `navigator.language`. Returns the first `Locale` whose short
/// code is a prefix of the browser-reported tag — covers regional
/// variants (`ru-RU`, `de-AT`) without enumerating each. The
/// detected value is only used the first time UserPrefs is built;
/// once persisted the picker decides.
pub fn detect_browser_locale() -> Locale {
    let Some(win) = web_sys::window() else {
        return Locale::En;
    };
    let lang = win.navigator().language().unwrap_or_default().to_lowercase();
    for loc in [Locale::Ru, Locale::De, Locale::Fr, Locale::Es, Locale::Ja] {
        if lang.starts_with(locale_code(loc)) {
            return loc;
        }
    }
    Locale::En
}
