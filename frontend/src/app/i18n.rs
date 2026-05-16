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

/// Languages the UI knows how to render. Default is `En` — `Ru` is
/// only selected if either `UserPrefs` already has it stored or the
/// browser advertises a Russian-family locale at first load.
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
}

impl Locale {
    /// Normalise compound-format dispatch: any locale without a
    /// curated `fmt_*` override falls back to English. Methods in
    /// the `impl Locale` block below call this on `self` at the
    /// top of each match so the existing two-arm matches stay
    /// exhaustive without adding a `Self::De` arm everywhere.
    #[inline]
    pub fn fmt_locale(self) -> Self {
        match self {
            Self::De => Self::En,
            other => other,
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
            (Self::En, MessageId::TipAutoToggleMidFight) => "auto toggle still works during a fight — the new setting takes effect once the current battle ends",
            (Self::Ru, MessageId::TipAutoToggleMidFight) => "переключатель авто работает и в бою — новое значение применится после окончания текущей схватки",
            (Self::En, MessageId::TipAutoEquipBest) => "walk every slot and equip the highest stat-sum piece you own",
            (Self::Ru, MessageId::TipAutoEquipBest) => "пройти по слотам и надеть лучшие предметы с наибольшей суммой характеристик",
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

            // German (C5). Selective overrides for the highest-impact
            // surface area (tabs, status pills) — anything not listed
            // falls through to English via this catch-all arm.
            (Self::De, m) => Self::tr_de(m).unwrap_or_else(|| Self::En.tr(m)),
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
            _ => return None,
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
            Self::De => unreachable!("fmt_locale normalises De → En/Ru"),
        };
        if missions_lost > 0 {
            let tail = match self.fmt_locale() {
                Self::En => format!(" {missions_lost} ended in defeat."),
                Self::Ru => format!(" {missions_lost} закончились поражением."),
                Self::De => unreachable!("fmt_locale normalises De → En/Ru"),
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
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Onboarding step counter ("step 1 / 4").
    pub fn fmt_onboarding_step(self, current: u8, total: u8) -> String {
        match self.fmt_locale() {
            Self::En => format!("step {current} / {total}"),
            Self::Ru => format!("шаг {current} / {total}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Last-publish timestamp string ("3s ago" / "never").
    pub fn fmt_seconds_ago(self, seconds: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("{seconds}s ago"),
            Self::Ru => format!("{seconds} с назад"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    pub fn term_never(self) -> &'static str {
        self.tr(MessageId::TermNever)
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
                    Self::De => unreachable!("fmt_locale normalises De"),
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
                    Self::De => unreachable!("fmt_locale normalises De"),
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
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "currently farming: X · level Y" on the world map.
    pub fn fmt_currently_farming(self, area: &str, lvl: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("currently farming: {area} · level {lvl}"),
            Self::Ru => format!("сейчас фармишь: {area} · уровень {lvl}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "gold balance: X · potions: Y · fireballs: Z" — Shop top line.
    /// Quantities are passed as strings so any integer width works.
    pub fn fmt_shop_balance(self, gold: &str, potions: &str, fireballs: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("gold balance: {gold} · potions: {potions} · fireballs: {fireballs}"),
            Self::Ru => format!("золото: {gold} · зелья: {potions} · фаерболы: {fireballs}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "Buy (Xg)" — paid-by-gold button label.
    pub fn fmt_buy_gold(self, price: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("Buy ({price}g)"),
            Self::Ru => format!("Купить ({price} зол)"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "Buy (Xe)" — paid-by-essence button label.
    pub fn fmt_buy_essence(self, price: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("Buy ({price}e)"),
            Self::Ru => format!("Купить ({price} эсс)"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "active players (N)" header.
    pub fn fmt_active_players(self, n: usize) -> String {
        match self.fmt_locale() {
            Self::En => format!("active players ({n})"),
            Self::Ru => format!("активные игроки ({n})"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "you are in: <guild name>" — h3 on the Guilds tab.
    pub fn fmt_you_are_in_guild(self, name: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("you are in: {name}"),
            Self::Ru => format!("ты в гильдии: {name}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "members: N / M · leader: …" — guild meta line.
    pub fn fmt_guild_meta(self, members: usize, max_members: usize, leader_label: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("members: {members} / {max_members} · leader: {leader_label}"),
            Self::Ru => format!("участники: {members} / {max_members} · лидер: {leader_label}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "directory (N)" header.
    pub fn fmt_directory(self, n: usize) -> String {
        match self.fmt_locale() {
            Self::En => format!("directory ({n})"),
            Self::Ru => format!("каталог ({n})"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "stash (N)" header on the Shop tab.
    pub fn fmt_stash_header(self, n: usize) -> String {
        match self.fmt_locale() {
            Self::En => format!("stash ({n})"),
            Self::Ru => format!("запас ({n})"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Sync-cadence button labels — were `&'static str` on SyncCadence.
    /// Pass the cadence in and we route to the right pair.
    pub fn fmt_sync_cadence(self, cadence: crate::app::prefs::SyncCadence) -> &'static str {
        use crate::app::prefs::SyncCadence as C;
        match (self.fmt_locale(), cadence) {
            (Self::En, C::Aggressive) => "Aggressive (5s)",
            (Self::Ru, C::Aggressive) => "Агрессивно (5с)",
            (Self::En, C::Normal) => "Normal (10s)",
            (Self::Ru, C::Normal) => "Обычно (10с)",
            (Self::En, C::Easy) => "Easy (30s)",
            (Self::Ru, C::Easy) => "Спокойно (30с)",
            (Self::De, _) => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Auto-pause HP-percent radio label.
    pub fn fmt_hp_pause_label(self, pct: u8) -> String {
        match (self, pct) {
            (Self::En, 0) => "0% (only at 0 HP)".to_string(),
            (Self::Ru, 0) => "0% (только при 0 ОЗ)".to_string(),
            (_, p) => format!("{p}%"),
        }
    }

    /// World-map area card footer for the level-locked state — "lvl X required".
    pub fn fmt_lvl_required(self, min_level: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("lvl {min_level} required"),
            Self::Ru => format!("нужен ур. {min_level}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
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
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Compact "cleared N times" indicator on each area card —
    /// shown alongside the gold / essence / damage badges so the
    /// player sees their mastery progress per zone.
    pub fn fmt_cleared_count(self, n: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("cleared {n}"),
            Self::Ru => format!("зачищено {n}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Encounter progress line during a battle.
    pub fn fmt_encounter_progress(self, idx: u32, total: u32) -> String {
        match self.fmt_locale() {
            Self::En => format!("encounter {idx} / {total}"),
            Self::Ru => format!("сражение {idx} / {total}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Tutorial / no-encounters hint about gear drop cadence.
    pub fn fmt_no_spare_loot(self, every_n: u32) -> String {
        match self.fmt_locale() {
            Self::En => format!("no spare loot yet — gear drops every {every_n} missions"),
            Self::Ru => format!("свободного снаряжения нет — выпадает каждые {every_n} миссий"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "Chapter N" caption on the plot panel.
    pub fn fmt_chapter(self, n: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("Chapter {n}"),
            Self::Ru => format!("Глава {n}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
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
                    Self::De => unreachable!("fmt_locale normalises De"),
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
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "last publish: <age> · published gold X · published damage Y"
    pub fn fmt_last_publish(self, age: &str, gold: &str, damage: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("last publish: {age} · published gold {gold} · published damage {damage}"),
            Self::Ru => format!("последняя публикация: {age} · золото {gold} · урон по боссу {damage}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "equipped bonus: +X atk · +Y def · +Z hp" — equipment-panel subtitle.
    pub fn fmt_equipped_bonus(self, atk: u64, def: u64, hp: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("equipped bonus: +{atk} atk · +{def} def · +{hp} hp"),
            Self::Ru => format!("бонус экипировки: +{atk} атк · +{def} защ · +{hp} ОЗ"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "deals {N} damage to the World Boss" — fireball idle tooltip.
    pub fn fmt_fireball_idle(self, dmg: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("deals {dmg} damage to the World Boss"),
            Self::Ru => format!("наносит {dmg} урона Мировому Боссу"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "convert all wheat to gold at 1:N" — Sell All Wheat tooltip.
    pub fn fmt_sell_wheat_tooltip(self, ratio: u64) -> String {
        match self.fmt_locale() {
            Self::En => format!("convert all wheat to gold at 1:{ratio}"),
            Self::Ru => format!("обменять всю пшеницу на золото по курсу 1:{ratio}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// "wheat: N · would sell for Mg" — farm panel running total.
    pub fn fmt_wheat_balance(self, wheat: &str, gold: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("wheat: {wheat} · would sell for {gold}g"),
            Self::Ru => format!("пшеница: {wheat} · принесёт {gold} зол"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Confirm-dialog body: "Reset all inventory progress?…"
    pub fn confirm_reset_progress(self) -> &'static str {
        match self.fmt_locale() {
            Self::En => "Reset all inventory progress?\n\nYour identity (pubkey) stays the same — leaderboards keep recognizing you — but every counter, item, skill, ending, and achievement goes back to zero.",
            Self::Ru => "Сбросить весь прогресс инвентаря?\n\nЛичность (публичный ключ) остаётся той же — таблицы лидеров продолжат тебя узнавать — но все счётчики, предметы, навыки, финалы и достижения обнулятся.",
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Confirm-dialog body: "Reveal your Ed25519 seed?…"
    pub fn confirm_reveal_seed(self) -> &'static str {
        match self.fmt_locale() {
            Self::En => "Reveal your Ed25519 seed?\n\nAnyone holding it can impersonate you. Only paste it into trusted backup storage; never into chat or screenshots.",
            Self::Ru => "Показать Ed25519 seed?\n\nЛюбой, кто получит его, сможет выдать себя за тебя. Вставляй его только в надёжное хранилище резервных копий — никогда в чат или скриншоты.",
                    Self::De => unreachable!("fmt_locale normalises De"),
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
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Status-bar line after exporting the seed — flips between
    /// success and error variants.
    pub fn status_seed_exported(self) -> &'static str {
        match self.fmt_locale() {
            Self::En => "seed exported — copy and hide promptly",
            Self::Ru => "seed экспортирован — скопируй и спрячь поскорее",
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    pub fn fmt_status_seed_export_failed(self, err: &str) -> String {
        match self.fmt_locale() {
            Self::En => format!("export failed: {err}"),
            Self::Ru => format!("экспорт не удался: {err}"),
                    Self::De => unreachable!("fmt_locale normalises De"),
        }
    }

    /// Help-tab body — kept as one big formatted blob per locale.
    /// Returns plain text (without inline `<strong>` markup); the
    /// help tab no longer needs the bolded keywords once translated.
    pub fn help_body(self) -> HelpBody {
        match self.fmt_locale() {
            Self::En => HelpBody::EN,
            Self::Ru => HelpBody::RU,
                    Self::De => unreachable!("fmt_locale normalises De"),
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
    pub tabs: &'static [&'static str],
    pub shop_p1: &'static str,
    pub shop_p2: &'static str,
    pub consumables_p1: &'static str,
    pub world_boss_p1: &'static str,
    pub delegate_p1: &'static str,
    pub delegate_p2: &'static str,
    pub guilds_p1: &'static str,
    pub guilds_p2: &'static str,
}

impl HelpBody {
    pub const EN: Self = Self {
        loop_p1: "Click Run Mission on the Farm tab to start a chain of up to 5 encounters against the current area's enemies. Combat is tick-based: one turn fires every 700ms, and you can queue Use Potion or Use Fireball mid-fight to react to a bad streak. Each win grants gold, essence, XP, and chips the shared World Boss. Lose, and you transform into the enemy that beat you.",
        loop_p2: "Toggle auto: on and the node-side delegate keeps running missions even after you close the tab. You'll return to a while you were away banner summing up what happened (capped at ~1 hour of catch-up). Auto pauses when HP drops below the threshold you pick in Settings → auto-mission.",
        stats_p1: "Your Level comes from cumulative XP. XP per level rises 1.5× each step (100, 150, 225, 337, …). Base stats are static per level: HP = 20 + lvl×5, Attack = 5 + lvl×2, Defence = 5 + lvl×2. Equipment, form, and skills add on top; nothing else (no gold/essence bleed-through).",
        stats_p2: "HP depletes in combat and regenerates over time (full regen in 180s of real time). Use a Potion to instantly fill it.",
        forms_p1: "Losing a combat to a non-mundane enemy permanently transforms you into that monster. Each form has its own equipped-slot mask: a Slime can only wear Helm + Ring, a Cat keeps Helm/Cloak/Boots/Ring, etc. Stats shift to match the form. Every form you've touched leaves a permanent Skill — even after you change back to Human, those bonuses carry. This is the prestige loop.",
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
    };

    pub const RU: Self = Self {
        loop_p1: "Нажми «В миссию» на вкладке Ферма, чтобы запустить цепочку до 5 сражений с врагами текущей области. Бой пошаговый: ход срабатывает раз в 700 мс, и ты можешь поставить «Использовать зелье» или «Использовать фаербол» в очередь прямо в бою, чтобы отреагировать на неудачный заход. Каждая победа даёт золото, эссенцию, опыт и чуть-чуть откусывает от общего Мирового Босса. Проиграешь — превратишься в победившего тебя врага.",
        loop_p2: "Включи «авто: вкл» — и делегат на узле продолжит проходить миссии даже после закрытия вкладки. Когда вернёшься, увидишь баннер «пока тебя не было» со сводкой (до ~1 часа симуляции за раз). Авто-режим ставится на паузу, когда ОЗ падают ниже порога из Настройки → авто-миссия.",
        stats_p1: "Уровень вычисляется из накопленного опыта. Требуемый опыт растёт в 1.5 раза за ступень (100, 150, 225, 337…). Базовые характеристики статичны: ОЗ = 20 + ур×5, Атака = 5 + ур×2, Защита = 5 + ур×2. Снаряжение, форма и навыки складываются сверху; ничего другого не влияет (золото/эссенция не перетекают в боевые статы).",
        stats_p2: "ОЗ убывают в бою и восстанавливаются со временем (полное восстановление за 180 секунд реального времени). Зелье моментально доводит ОЗ до максимума.",
        forms_p1: "Поражение не-обычному врагу навсегда превращает тебя в этого монстра. У каждой формы своя маска слотов: Слизь носит только Шлем + Кольцо, Кот — Шлем/Плащ/Сапоги/Кольцо и так далее. Характеристики подстраиваются под форму. Каждая форма, в которой ты побывал, оставляет постоянный Навык — даже после возврата в Человека бонусы сохраняются. Это и есть петля престижа.",
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
    TabSettings,
    TabHelp,

    // Status pills (rendered in uppercase by the existing CSS).
    PillDefeated,
    PillAdventuring,
    PillFocusing,
    PillRecovering,
    PillReady,

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

    // Hover tooltips on buttons / icons.
    TipFightInProgress,
    TipAutoToggleMidFight,
    TipAutoEquipBest,
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
    }
}

/// Parse a short code back to `Locale`. Unknown codes fall through
/// to `En` — same defensiveness as `apply_theme` on a missing
/// `data-theme` attribute.
pub fn locale_from_code(code: &str) -> Locale {
    match code {
        "ru" => Locale::Ru,
        "de" => Locale::De,
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
    for loc in [Locale::Ru, Locale::De] {
        if lang.starts_with(locale_code(loc)) {
            return loc;
        }
    }
    Locale::En
}
