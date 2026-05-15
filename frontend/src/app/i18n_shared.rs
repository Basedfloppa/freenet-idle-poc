//! Locale-aware wrappers around the static name/blurb tables that
//! live in the `shared` crate (form names, area names, enemy names,
//! skill names, ending names, achievement labels, slot/tier names,
//! chapter copy, plot word lists).
//!
//! Why mirror them here instead of teaching `shared` about `Locale`?
//! `shared` is consumed by the delegate, contracts, and any non-UI
//! tooling we may add later — adding a per-string locale parameter
//! would either (a) leak UI concerns into the delegate or (b) force
//! every backend caller to thread a "no-locale" default. Mirroring
//! the lookups on the webapp side keeps `shared` text-pure and
//! gives the UI a single place to grow translations.
//!
//! Conventions:
//! - All `*_name` / `*_blurb` functions return `&'static str` so
//!   they're zero-allocation pointer copies.
//! - `gear_name` and `chapter` return `String` because they combine
//!   slot + tier (or area + mission count) into one string.
//! - `plot_tuple_l10n` returns five owned strings; the source nouns
//!   are arrays of `&str` so we hand back `&'static str` after
//!   indexing.

use shared::{
    AreaDef, EnemyDef, GearTemplate, Inventory, ACH_BRONZE_GRINDER, ACH_CAPTAIN,
    ACH_FIRST_BLOOD, ACH_FIRST_KILL, ACH_FIRST_LEGENDARY, ACH_FIRST_MISSION, ACH_GOLD_GRINDER,
    ACH_LIEUTENANT, ACH_SILVER_GRINDER, ACH_SOUL_BOUND, ACH_TREASURER, AchievementCheck,
    ACHIEVEMENT_TABLE, ENDING_DRAGON_LORD, ENDING_PILGRIM, ENDING_QUIET_FARMER, ENDING_VICTORY,
    FORM_CAT, FORM_DRAGON, FORM_HORSE, FORM_HUMAN, FORM_SLIME, SKILL_CHAMPION,
    SKILL_DRAGON_SCALES, SKILL_FELINE_GRACE, SKILL_SLIME_BODY, SKILL_STEED_HEART, SKILL_VETERAN,
    SLOT_COUNT,
};

use super::i18n::Locale;

/// Localized form name. Falls through to "Unknown"/"Неизвестно" for
/// out-of-table ids — matches the shared crate's `"Unknown"` fallback.
pub fn form_name(locale: Locale, form: u8) -> &'static str {
    match (locale, form) {
        (Locale::En, FORM_HUMAN) => "Human",
        (Locale::Ru, FORM_HUMAN) => "Человек",
        (Locale::En, FORM_SLIME) => "Slime",
        (Locale::Ru, FORM_SLIME) => "Слизь",
        (Locale::En, FORM_CAT) => "Cat",
        (Locale::Ru, FORM_CAT) => "Кот",
        (Locale::En, FORM_DRAGON) => "Dragon",
        (Locale::Ru, FORM_DRAGON) => "Дракон",
        (Locale::En, FORM_HORSE) => "Horse",
        (Locale::Ru, FORM_HORSE) => "Конь",
        (Locale::En, _) => "Unknown",
        (Locale::Ru, _) => "Неизвестно",
    }
}

/// Localized area name. Falls back to the English `AreaDef.name` for
/// out-of-table ids so the UI never shows an empty cell.
pub fn area_name(locale: Locale, area: &AreaDef) -> &'static str {
    match (locale, area.id) {
        (Locale::En, _) => area.name,
        (Locale::Ru, 0) => "Деревенские поля",
        (Locale::Ru, 1) => "Лесная дорога",
        (Locale::Ru, 2) => "Горный перевал",
        (Locale::Ru, 3) => "Логово Босса",
        (Locale::Ru, _) => area.name,
    }
}

pub fn area_blurb(locale: Locale, area: &AreaDef) -> &'static str {
    match (locale, area.id) {
        (Locale::En, _) => area.blurb,
        (Locale::Ru, 0) => "лёгкая работа — сбалансированные награды (без босса)",
        (Locale::Ru, 1) => "много эссенции, мало риска (без босса)",
        (Locale::Ru, 2) => "купцы платят щедро; меньше эссенции (без босса)",
        (Locale::Ru, 3) => "тяжёлый урон; единственная область, бьющая Мирового Босса",
        (Locale::Ru, _) => area.blurb,
    }
}

/// Localized enemy display name. Uses the enemy id for routing so
/// the table stays compact even as new enemies get added.
pub fn enemy_name(locale: Locale, enemy: &EnemyDef) -> &'static str {
    match (locale, enemy.id) {
        (Locale::En, _) => enemy.name,
        (Locale::Ru, 0) => "злой эльф",
        (Locale::Ru, 1) => "средневековый юрист",
        (Locale::Ru, 2) => "тревожная слизь",
        (Locale::Ru, 10) => "одичавший кот",
        (Locale::Ru, 11) => "тёрновый призрак",
        (Locale::Ru, 20) => "каменный голем",
        (Locale::Ru, 21) => "дух боевого коня",
        (Locale::Ru, 30) => "молодой дракон",
        (Locale::Ru, 31) => "повелитель теней",
        (Locale::Ru, _) => enemy.name,
    }
}

pub fn enemy_death_blurb(locale: Locale, enemy: &EnemyDef) -> &'static str {
    match (locale, enemy.id) {
        (Locale::En, _) => enemy.death_blurb,
        (Locale::Ru, 0) => "Эльф одолевает тебя и оставляет истекать кровью у дороги. Ты доползаешь домой, в синяках, но всё ещё собой.",
        (Locale::Ru, 1) => "Юрист вручает тебе предписание, которое сплющивает твоё эго. Ты ковыляешь домой, всё такой же обычный.",
        (Locale::Ru, 2) => "Слизь делится надвое, а новая половина бросается на тебя. Тебя засасывает, тело плавится и сочится, и ты становишься зелёным сияющим комом тупой слизи.",
        (Locale::Ru, 10) => "Кот прыгает и прокусывает тебе душу. Когда зрение меркнет, у тебя пробивается шерсть, усы и глубокая мудрость зверя, что сбивает вещи со столов.",
        (Locale::Ru, 11) => "Шипы призрака неделю заставляют твои вены светиться зелёным, но ты доковыливаешь домой целиком.",
        (Locale::Ru, 20) => "Голем расплющивает тебя в лепёшку. Ты просыпаешься у начала тропы, помят, но не окристалл.",
        (Locale::Ru, 21) => "Боевой конь встаёт на дыбы, и пока его копыта опускаются, ты чувствуешь, как удлиняется хребет, сливаются руки, отступает достоинство. Теперь ты прочный четвероногий.",
        (Locale::Ru, 30) => "Огонь дракона спекает твои кости в чешуйки. Когда всё кончается, ты не помнишь, как быть маленьким. Теперь ты дракон.",
        (Locale::Ru, 31) => "Повелитель теней высасывает тебя до оболочки, но кожа выдерживает. Ты возвращаешься в деревню, всё ещё человек, всё ещё жив — едва.",
        (Locale::Ru, _) => enemy.death_blurb,
    }
}

/// Localized skill name.
pub fn skill_name(locale: Locale, id: u8) -> &'static str {
    match (locale, id) {
        (Locale::En, SKILL_SLIME_BODY) => "Slime Body",
        (Locale::Ru, SKILL_SLIME_BODY) => "Тело Слизи",
        (Locale::En, SKILL_FELINE_GRACE) => "Feline Grace",
        (Locale::Ru, SKILL_FELINE_GRACE) => "Кошачья грация",
        (Locale::En, SKILL_DRAGON_SCALES) => "Dragon Scales",
        (Locale::Ru, SKILL_DRAGON_SCALES) => "Драконья чешуя",
        (Locale::En, SKILL_STEED_HEART) => "Steed Heart",
        (Locale::Ru, SKILL_STEED_HEART) => "Сердце скакуна",
        (Locale::En, SKILL_VETERAN) => "Veteran",
        (Locale::Ru, SKILL_VETERAN) => "Ветеран",
        (Locale::En, SKILL_CHAMPION) => "Champion",
        (Locale::Ru, SKILL_CHAMPION) => "Чемпион",
        _ => "?",
    }
}

pub fn skill_blurb(locale: Locale, id: u8) -> &'static str {
    match (locale, id) {
        (Locale::En, SKILL_SLIME_BODY) => "You've been gooey once. The membrane carries over: +20 HP, +5 defence.",
        (Locale::Ru, SKILL_SLIME_BODY) => "Ты уже бывал желеобразным. Мембрана остаётся: +20 ОЗ, +5 защиты.",
        (Locale::En, SKILL_FELINE_GRACE) => "Your reflexes remember the cat: +6 attack.",
        (Locale::Ru, SKILL_FELINE_GRACE) => "Рефлексы помнят кошку: +6 атаки.",
        (Locale::En, SKILL_DRAGON_SCALES) => "Stray scales still cling to your skin: +8 attack, +6 defence.",
        (Locale::Ru, SKILL_DRAGON_SCALES) => "Оставшиеся чешуйки прирастают к коже: +8 атаки, +6 защиты.",
        (Locale::En, SKILL_STEED_HEART) => "A horse's lung capacity outlasts the form: +25 HP, +4 defence.",
        (Locale::Ru, SKILL_STEED_HEART) => "Лошадиная ёмкость лёгких переживает форму: +25 ОЗ, +4 защиты.",
        (Locale::En, SKILL_VETERAN) => "Ten levels of combat experience: +5 attack, +5 defence.",
        (Locale::Ru, SKILL_VETERAN) => "Десять уровней боевого опыта: +5 атаки, +5 защиты.",
        (Locale::En, SKILL_CHAMPION) => "Twenty levels in, you've earned the title: +10 atk, +10 def, +30 HP.",
        (Locale::Ru, SKILL_CHAMPION) => "За двадцать уровней ты заслужил титул: +10 атк, +10 защ, +30 ОЗ.",
        _ => "",
    }
}

/// Localized ending name.
pub fn ending_name(locale: Locale, id: u8) -> &'static str {
    match (locale, id) {
        (Locale::En, ENDING_VICTORY) => "Hero's Victory",
        (Locale::Ru, ENDING_VICTORY) => "Победа Героя",
        (Locale::En, ENDING_DRAGON_LORD) => "Dragon Ascendant",
        (Locale::Ru, ENDING_DRAGON_LORD) => "Восхождение Дракона",
        (Locale::En, ENDING_PILGRIM) => "Pilgrim of Forms",
        (Locale::Ru, ENDING_PILGRIM) => "Странник Форм",
        (Locale::En, ENDING_QUIET_FARMER) => "Quiet Farmer",
        (Locale::Ru, ENDING_QUIET_FARMER) => "Тихий фермер",
        _ => "?",
    }
}

pub fn ending_blurb(locale: Locale, id: u8) -> &'static str {
    match (locale, id) {
        (Locale::En, ENDING_VICTORY) => "Felled the Shadow Lord with your bare human hands. The kingdom remembers your name.",
        (Locale::Ru, ENDING_VICTORY) => "Сразил Повелителя Теней голыми человеческими руками. Королевство помнит твоё имя.",
        (Locale::En, ENDING_DRAGON_LORD) => "You came as dragon and left as dragon, but the Shadow Lord's keep is your eyrie now.",
        (Locale::Ru, ENDING_DRAGON_LORD) => "Ты пришёл драконом и ушёл драконом, но крепость Повелителя Теней — теперь твоё гнездо.",
        (Locale::En, ENDING_PILGRIM) => "You've worn every shape on the map and decided each one was, technically, also you.",
        (Locale::Ru, ENDING_PILGRIM) => "Ты примерил каждую форму на карте и решил, что любая из них тоже, формально, — ты.",
        (Locale::En, ENDING_QUIET_FARMER) => "Ten thousand bushels of wheat. The Shadow Lord still lurks somewhere, but the harvest is good.",
        (Locale::Ru, ENDING_QUIET_FARMER) => "Десять тысяч мер пшеницы. Повелитель Теней где-то ещё прячется, но урожай хорош.",
        _ => "",
    }
}

/// Localized achievement label (chip text).
pub fn achievement_label(locale: Locale, id: u8) -> &'static str {
    match (locale, id) {
        (Locale::En, ACH_FIRST_MISSION) => "first mission",
        (Locale::Ru, ACH_FIRST_MISSION) => "первая миссия",
        (Locale::En, ACH_BRONZE_GRINDER) => "bronze grinder",
        (Locale::Ru, ACH_BRONZE_GRINDER) => "бронзовый труженик",
        (Locale::En, ACH_SILVER_GRINDER) => "silver grinder",
        (Locale::Ru, ACH_SILVER_GRINDER) => "серебряный труженик",
        (Locale::En, ACH_GOLD_GRINDER) => "gold grinder",
        (Locale::Ru, ACH_GOLD_GRINDER) => "золотой труженик",
        (Locale::En, ACH_FIRST_BLOOD) => "first blood",
        (Locale::Ru, ACH_FIRST_BLOOD) => "первая кровь",
        (Locale::En, ACH_LIEUTENANT) => "lieutenant",
        (Locale::Ru, ACH_LIEUTENANT) => "лейтенант",
        (Locale::En, ACH_CAPTAIN) => "captain",
        (Locale::Ru, ACH_CAPTAIN) => "капитан",
        (Locale::En, ACH_TREASURER) => "treasurer",
        (Locale::Ru, ACH_TREASURER) => "казначей",
        (Locale::En, ACH_SOUL_BOUND) => "soul-bound",
        (Locale::Ru, ACH_SOUL_BOUND) => "связан душой",
        (Locale::En, ACH_FIRST_KILL) => "first kill",
        (Locale::Ru, ACH_FIRST_KILL) => "первое убийство",
        (Locale::En, ACH_FIRST_LEGENDARY) => "first legendary",
        (Locale::Ru, ACH_FIRST_LEGENDARY) => "первая легендарка",
        _ => "?",
    }
}

/// Localized achievement unlock criterion (tooltip body / toast body).
/// Mirrors `shared::achievement_reason` but routes through the
/// `Locale`-aware label formatter.
pub fn achievement_reason(locale: Locale, id: u8) -> String {
    for (aid, check) in ACHIEVEMENT_TABLE {
        if *aid == id {
            return match (locale, *check) {
                (Locale::En, AchievementCheck::Missions(n)) => format!("Run {n} missions"),
                (Locale::Ru, AchievementCheck::Missions(n)) => format!("Пройди {n} миссий"),
                (Locale::En, AchievementCheck::BossDamage(n)) => format!("Deal {n} damage to the World Boss"),
                (Locale::Ru, AchievementCheck::BossDamage(n)) => format!("Нанеси {n} урона Мировому Боссу"),
                (Locale::En, AchievementCheck::Gold(n)) => format!("Accumulate {n} gold"),
                (Locale::Ru, AchievementCheck::Gold(n)) => format!("Накопи {n} золота"),
                (Locale::En, AchievementCheck::Essence(n)) => format!("Accumulate {n} essence"),
                (Locale::Ru, AchievementCheck::Essence(n)) => format!("Накопи {n} эссенции"),
                (Locale::En, AchievementCheck::WinCount(n)) => format!("Win {n} encounters"),
                (Locale::Ru, AchievementCheck::WinCount(n)) => format!("Выиграй {n} сражений"),
                (Locale::En, AchievementCheck::LegendaryEquipped) => "Equip a Legendary (T4) item".into(),
                (Locale::Ru, AchievementCheck::LegendaryEquipped) => "Надень Легендарный (T4) предмет".into(),
            };
        }
    }
    match locale {
        Locale::En => "unknown achievement".into(),
        Locale::Ru => "неизвестное достижение".into(),
    }
}

/// Localized gear slot label (Helm / Шлем / etc.). Idx is the slot
/// index used by `SLOT_NAMES`. Falls back to the shared crate's
/// English label for out-of-range indices — defensive only, real
/// callers always pass a valid 0..SLOT_COUNT index.
pub fn slot_name(locale: Locale, idx: usize) -> &'static str {
    const RU_SLOTS: [&str; SLOT_COUNT] =
        ["Шлем", "Плащ", "Нагрудник", "Штаны", "Щит", "Меч", "Сапоги", "Кольцо"];
    match locale {
        Locale::En => shared::SLOT_NAMES.get(idx).copied().unwrap_or("?"),
        Locale::Ru => RU_SLOTS.get(idx).copied().unwrap_or("?"),
    }
}

/// Localized gear-tier prefix (Worn / Изношенный / etc.).
pub fn tier_prefix(locale: Locale, tier: u8) -> &'static str {
    let idx = tier.saturating_sub(1) as usize;
    const RU_TIERS: [&str; 4] = ["Изношенный", "Полированный", "Рунный", "Легендарный"];
    match locale {
        Locale::En => shared::TIER_PREFIXES.get(idx).copied().unwrap_or("?"),
        Locale::Ru => RU_TIERS.get(idx).copied().unwrap_or("?"),
    }
}

/// Localized gear name — tier prefix + slot. Mirrors
/// `GearTemplate::name()` but routes both halves through the locale.
pub fn gear_name(locale: Locale, t: &GearTemplate) -> String {
    format!("{} {}", tier_prefix(locale, t.tier), slot_name(locale, t.slot as usize))
}

/// Chapter copy — returns `(chapter number, title, body)` for the
/// player's currently selected area. Mirrors
/// `frontend::game::derived::current_chapter` but adds locale
/// routing. Falls through to chapter 4 / boss's lair for area ids
/// beyond the table.
pub fn chapter(locale: Locale, inv: &Inventory) -> (u8, String, String) {
    let area_id = inv.current_area;
    match (locale, area_id) {
        (Locale::En, 0) => (
            1,
            "Chapter 1 · The Village Fields".into(),
            if inv.mission_count == 0 {
                "Your father points east. \"Be strong, and bring the boss down.\" The fields outside the village are quiet — for now. Run a mission to begin.".into()
            } else {
                "You're running errands at the edge of the fields. Each mission trickles gold and essence into the lockbox the delegate keeps for you on the node.".into()
            },
        ),
        (Locale::Ru, 0) => (
            1,
            "Глава 1 · Деревенские поля".into(),
            if inv.mission_count == 0 {
                "Отец указывает на восток. «Будь сильным и одолей босса». Поля за деревней пока тихи — запусти миссию, чтобы начать.".into()
            } else {
                "Ты разбираешь мелочи на краю полей. Каждая миссия по чуть-чуть капает золотом и эссенцией в сейф, который делегат держит на узле.".into()
            },
        ),
        (Locale::En, 1) => (
            2,
            "Chapter 2 · The Forest Road".into(),
            "Word of your exploits has reached the next biome. The forest paths yield more essence, but the World Boss begins to stir as every player chips at its HP.".into(),
        ),
        (Locale::Ru, 1) => (
            2,
            "Глава 2 · Лесная дорога".into(),
            "Слухи о твоих подвигах добрались до следующего биома. Лесные тропы дают больше эссенции, но Мировой Босс начинает шевелиться, пока каждый игрок откусывает кусочки от его ОЗ.".into(),
        ),
        (Locale::En, 2) => (
            3,
            "Chapter 3 · The Mountain Pass".into(),
            "Merchants pay handsomely at the pass, and the loot scales. Other adventurers across the network are converging on the same foe — every hit is mirrored in the global HP gauge.".into(),
        ),
        (Locale::Ru, 2) => (
            3,
            "Глава 3 · Горный перевал".into(),
            "Купцы на перевале платят щедро, и добыча масштабируется. Другие искатели приключений по всей сети сходятся к одному и тому же врагу — каждый удар отражается в общей шкале ОЗ.".into(),
        ),
        (Locale::En, _) => (
            4,
            "Chapter 4 · The Boss's Lair".into(),
            "You've reached the inner sanctum. Damage-heavy work — every blow you land is mirrored in the World Boss HP gauge that every connected player sees in real time.".into(),
        ),
        (Locale::Ru, _) => (
            4,
            "Глава 4 · Логово Босса".into(),
            "Ты добрался до внутреннего святилища. Тяжёлая работа по урону — каждый твой удар отражается в шкале ОЗ Мирового Босса, которую каждый подключённый игрок видит в реальном времени.".into(),
        ),
    }
}

/// Plot word-list expansion — six-slot Mad Libs source. Returns the
/// (home, macguffin, villain, method, destination) tuple already
/// localized; the consumer passes these into
/// `Locale::fmt_plot_backstory`.
pub fn plot_tuple_l10n(locale: Locale, seed: u32) -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
    let s = seed as u64;
    let home = WORDS_HOMES[locale_idx(locale)][(s % WORDS_HOMES[0].len() as u64) as usize];
    let mac = WORDS_MACGUFFINS[locale_idx(locale)][((s / 7) % WORDS_MACGUFFINS[0].len() as u64) as usize];
    let vil = WORDS_VILLAINS[locale_idx(locale)][((s / 53) % WORDS_VILLAINS[0].len() as u64) as usize];
    let mthd = WORDS_METHODS[locale_idx(locale)][((s / 211) % WORDS_METHODS[0].len() as u64) as usize];
    let dest = WORDS_DESTINATIONS[locale_idx(locale)][((s / 1009) % WORDS_DESTINATIONS[0].len() as u64) as usize];
    (home, mac, vil, mthd, dest)
}

fn locale_idx(locale: Locale) -> usize {
    match locale {
        Locale::En => 0,
        Locale::Ru => 1,
    }
}

// Word lists mirror `shared::PLOT_*` for English and add a Russian
// parallel set. Index 0 = English, index 1 = Russian. The Russian
// nouns are inflected so they read grammatically inside
// `Locale::fmt_plot_backstory`'s sentence template.
const WORDS_HOMES: [&[&str]; 2] = [
    &[
        "floating castle of Bloodpool",
        "hamlet of Kirkwent",
        "village of Greenmoor",
        "port of Saltreach",
        "mire of Thornveil",
        "drowned town of Felgrave",
    ],
    &[
        "летающем замке Кровавая Заводь",
        "хуторе Киркуэнт",
        "деревне Зелёный Вереск",
        "порту Соленый Берег",
        "болоте Тёрновая Завеса",
        "затопленном городке Фелгрейв",
    ],
];

const WORDS_MACGUFFINS: [&[&str]; 2] = [
    &[
        "Chest of Cats",
        "sacred amulet of Sundered Light",
        "Last Egg",
        "world's only working watch",
        "Heart of the Mountain",
        "name of your mother",
    ],
    &[
        "Сундук с Котами",
        "священный амулет Расколотого Света",
        "Последнее Яйцо",
        "единственные в мире рабочие часы",
        "Сердце Горы",
        "имя твоей матери",
    ],
];

const WORDS_VILLAINS: [&[&str]; 2] = [
    &[
        "Dark Lord",
        "Whispering King",
        "Lich of the Salt Plain",
        "Shadow Council",
        "Wandering Hunger",
        "Crowned Glutton",
    ],
    &[
        "Тёмный Властелин",
        "Шепчущий Король",
        "Лич Солёной Равнины",
        "Совет Теней",
        "Бродячий Голод",
        "Венчанный Обжора",
    ],
];

const WORDS_METHODS: [&[&str]; 2] = [
    &[
        "Rain of Destruction",
        "midnight raid",
        "hex of forgetting",
        "terrible bargain at midnight",
        "summons from the deep",
        "ledger of broken oaths",
    ],
    &[
        "Дождь Разрушения",
        "полуночный набег",
        "заклятие забвения",
        "ужасную сделку в полночь",
        "призыв из глубин",
        "реестр нарушенных клятв",
    ],
];

const WORDS_DESTINATIONS: [&[&str]; 2] = [
    &[
        "Island in the Sky",
        "Forest of Doors",
        "Tower of Spires",
        "Abyss Below",
        "Mirror Pavilion",
        "city of locked rooms",
    ],
    &[
        "Небесный Остров",
        "Лес Дверей",
        "Башню Шпилей",
        "Бездну Внизу",
        "Зеркальный Павильон",
        "город запертых комнат",
    ],
];
