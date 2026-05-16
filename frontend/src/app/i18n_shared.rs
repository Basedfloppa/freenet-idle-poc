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

/// Localized form name. Falls through to "Unknown"/equivalent for
/// out-of-table ids — matches the shared crate's `"Unknown"` fallback.
pub fn form_name(locale: Locale, form: u8) -> &'static str {
    match (locale.fmt_locale(), form) {
        (Locale::En, FORM_HUMAN) => "Human",
        (Locale::Ru, FORM_HUMAN) => "Человек",
        (Locale::Fr, FORM_HUMAN) => "Humain",
        (Locale::Es, FORM_HUMAN) => "Humano",
        (Locale::Ja, FORM_HUMAN) => "人間",
        (Locale::En, FORM_SLIME) => "Slime",
        (Locale::Ru, FORM_SLIME) => "Слизь",
        (Locale::Fr, FORM_SLIME) => "Slime",
        (Locale::Es, FORM_SLIME) => "Limo",
        (Locale::Ja, FORM_SLIME) => "スライム",
        (Locale::En, FORM_CAT) => "Cat",
        (Locale::Ru, FORM_CAT) => "Кот",
        (Locale::Fr, FORM_CAT) => "Chat",
        (Locale::Es, FORM_CAT) => "Gato",
        (Locale::Ja, FORM_CAT) => "猫",
        (Locale::En, FORM_DRAGON) => "Dragon",
        (Locale::Ru, FORM_DRAGON) => "Дракон",
        (Locale::Fr, FORM_DRAGON) => "Dragon",
        (Locale::Es, FORM_DRAGON) => "Dragón",
        (Locale::Ja, FORM_DRAGON) => "竜",
        (Locale::En, FORM_HORSE) => "Horse",
        (Locale::Ru, FORM_HORSE) => "Конь",
        (Locale::Fr, FORM_HORSE) => "Cheval",
        (Locale::Es, FORM_HORSE) => "Caballo",
        (Locale::Ja, FORM_HORSE) => "馬",
        (Locale::En, _) => "Unknown",
        (Locale::Ru, _) => "Неизвестно",
        (Locale::Fr, _) => "Inconnu",
        (Locale::Es, _) => "Desconocido",
        (Locale::Ja, _) => "不明",
        (_, _) => unreachable!("fmt_locale normalises non-curated locales"),
    }
}

/// Localized area name. Falls back to the English `AreaDef.name` for
/// out-of-table ids so the UI never shows an empty cell.
pub fn area_name(locale: Locale, area: &AreaDef) -> &'static str {
    match (locale.fmt_locale(), area.id) {
        (Locale::En, _) => area.name,
        (Locale::Ru, 0) => "Деревенские поля",
        (Locale::Ru, 1) => "Лесная дорога",
        (Locale::Ru, 2) => "Горный перевал",
        (Locale::Ru, 3) => "Логово Босса",
        (Locale::Ru, 4) => "Глубокий лес",
        (Locale::Ru, 5) => "Снежные равнины",
        (Locale::Ru, _) => area.name,
        (Locale::Fr, 0) => "Champs du village",
        (Locale::Fr, 1) => "Route de la forêt",
        (Locale::Fr, 2) => "Col de la montagne",
        (Locale::Fr, 3) => "Antre du Boss",
        (Locale::Fr, 4) => "Forêt profonde",
        (Locale::Fr, 5) => "Plaines enneigées",
        (Locale::Fr, _) => area.name,
        (Locale::Es, 0) => "Campos del pueblo",
        (Locale::Es, 1) => "Senda del bosque",
        (Locale::Es, 2) => "Paso de la montaña",
        (Locale::Es, 3) => "Guarida del Jefe",
        (Locale::Es, 4) => "Bosque profundo",
        (Locale::Es, 5) => "Llanuras nevadas",
        (Locale::Es, _) => area.name,
        (Locale::Ja, 0) => "村の畑",
        (Locale::Ja, 1) => "森の道",
        (Locale::Ja, 2) => "山の峠",
        (Locale::Ja, 3) => "ボスの巣",
        (Locale::Ja, 4) => "深い森",
        (Locale::Ja, 5) => "雪原",
        (Locale::Ja, _) => area.name,
        (_, _) => unreachable!("fmt_locale normalises non-curated locales"),
    }
}

pub fn area_blurb(locale: Locale, area: &AreaDef) -> &'static str {
    match (locale.fmt_locale(), area.id) {
        (Locale::En, _) => area.blurb,
        (Locale::Ru, 0) => "лёгкая работа — сбалансированные награды (без босса)",
        (Locale::Ru, 1) => "много эссенции, мало риска (без босса)",
        (Locale::Ru, 2) => "купцы платят щедро; меньше эссенции (без босса)",
        (Locale::Ru, 3) => "тяжёлый урон; единственная область, бьющая Мирового Босса",
        (Locale::Ru, 4) => "густая чаща — больше эссенции, враги опаснее",
        (Locale::Ru, 5) => "продуваемые ветром плато — много золота, тяжёлые потери",
        (Locale::Ru, _) => area.blurb,
        (Locale::Fr, 0) => "travail léger — récompenses équilibrées (sans boss)",
        (Locale::Fr, 1) => "essence abondante, peu de risque (sans boss)",
        (Locale::Fr, 2) => "les marchands paient bien ; moins d'essence (sans boss)",
        (Locale::Fr, 3) => "lourds dégâts ; la seule zone qui blesse le Boss du Monde",
        (Locale::Fr, 4) => "futaie dense — plus d'essence, ennemis plus coriaces",
        (Locale::Fr, 5) => "plateaux balayés par le vent — beaucoup d'or, lourdes pertes",
        (Locale::Fr, _) => area.blurb,
        (Locale::Es, 0) => "trabajo ligero — recompensas equilibradas (sin jefe)",
        (Locale::Es, 1) => "mucha esencia, poco riesgo (sin jefe)",
        (Locale::Es, 2) => "los mercaderes pagan bien; menos esencia (sin jefe)",
        (Locale::Es, 3) => "daño pesado; la única zona que hiere al Jefe del Mundo",
        (Locale::Es, 4) => "fronda densa — más esencia, enemigos más duros",
        (Locale::Es, 5) => "mesetas barridas por el viento — mucho oro, bajas duras",
        (Locale::Es, _) => area.blurb,
        (Locale::Ja, 0) => "軽労働 — バランスの取れた報酬（ボスなし）",
        (Locale::Ja, 1) => "精が豊富、リスクは少ない（ボスなし）",
        (Locale::Ja, 2) => "商人は気前よく払う、精は控えめ（ボスなし）",
        (Locale::Ja, 3) => "重ダメージ、ワールドボスを削れる唯一のエリア",
        (Locale::Ja, 4) => "深い森 — 精が増えるが敵は危険",
        (Locale::Ja, 5) => "風吹きすさぶ高原 — 金は多いが損害も大きい",
        (Locale::Ja, _) => area.blurb,
        (_, _) => unreachable!("fmt_locale normalises non-curated locales"),
    }
}

/// Localized enemy display name. Uses the enemy id for routing so
/// the table stays compact even as new enemies get added.
pub fn enemy_name(locale: Locale, enemy: &EnemyDef) -> &'static str {
    match (locale.fmt_locale(), enemy.id) {
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
        (Locale::Fr, 0) => "elfe maléfique",
        (Locale::Fr, 1) => "juriste médiéval",
        (Locale::Fr, 2) => "slime inquiet",
        (Locale::Fr, 10) => "chat sauvage",
        (Locale::Fr, 11) => "spectre épineux",
        (Locale::Fr, 20) => "golem de pierre",
        (Locale::Fr, 21) => "esprit du destrier",
        (Locale::Fr, 30) => "jeune dragon",
        (Locale::Fr, 31) => "seigneur des ombres",
        (Locale::Fr, _) => enemy.name,
        (Locale::Es, 0) => "elfo maligno",
        (Locale::Es, 1) => "abogado medieval",
        (Locale::Es, 2) => "limo inquieto",
        (Locale::Es, 10) => "gato salvaje",
        (Locale::Es, 11) => "espectro de espino",
        (Locale::Es, 20) => "gólem de piedra",
        (Locale::Es, 21) => "espíritu del corcel",
        (Locale::Es, 30) => "dragón joven",
        (Locale::Es, 31) => "señor de las sombras",
        (Locale::Es, _) => enemy.name,
        (Locale::Ja, 0) => "邪悪なエルフ",
        (Locale::Ja, 1) => "中世の法律家",
        (Locale::Ja, 2) => "不穏なスライム",
        (Locale::Ja, 10) => "野良猫",
        (Locale::Ja, 11) => "茨の亡霊",
        (Locale::Ja, 20) => "石のゴーレム",
        (Locale::Ja, 21) => "軍馬の霊",
        (Locale::Ja, 30) => "若き竜",
        (Locale::Ja, 31) => "影の主",
        (Locale::Ja, _) => enemy.name,
        (_, _) => unreachable!("fmt_locale normalises non-curated locales"),
    }
}

pub fn enemy_death_blurb(locale: Locale, enemy: &EnemyDef) -> &'static str {
    match (locale.fmt_locale(), enemy.id) {
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
        (Locale::Fr, 0) => "L'elfe te bat et te laisse saignant au bord de la route. Tu rampes jusque chez toi, couvert de bleus mais toujours toi-même.",
        (Locale::Fr, 1) => "Le juriste te tend une assignation qui aplatit ton ego. Tu rentres en boitant, aussi banal qu'avant.",
        (Locale::Fr, 2) => "Le slime se scinde en deux, et la nouvelle moitié se jette sur toi. Tu es aspiré, ton corps fond et suinte, et tu deviens un amas vert luisant de slime stupide.",
        (Locale::Fr, 10) => "Le chat bondit et mord ton âme. Quand ta vue se brouille, du poil, des moustaches et une profonde sagesse féline pour faire tomber les objets des étagères poussent.",
        (Locale::Fr, 11) => "Les épines du spectre font luire tes veines en vert pendant une semaine, mais tu rentres entier.",
        (Locale::Fr, 20) => "Le golem t'écrase comme une crêpe. Tu te réveilles au début du sentier, cabossé mais pas cristallisé.",
        (Locale::Fr, 21) => "Le destrier se cabre, et pendant que ses sabots descendent, tu sens ta colonne s'allonger, tes bras fusionner, ta dignité reculer. Te voilà solide quadrupède.",
        (Locale::Fr, 30) => "Le feu du dragon cuit tes os en écailles. Quand tout est fini, tu ne sais plus comment être petit. Tu es un dragon désormais.",
        (Locale::Fr, 31) => "Le seigneur des ombres te vide jusqu'à la coquille, mais la peau tient. Tu retournes au village, encore humain, encore en vie — de justesse.",
        (Locale::Fr, _) => enemy.death_blurb,
        (Locale::Es, 0) => "El elfo te derrota y te deja sangrando al borde del camino. Te arrastras a casa, magullado pero aún tú.",
        (Locale::Es, 1) => "El abogado te entrega una citación que aplasta tu ego. Vuelves cojeando, tan corriente como antes.",
        (Locale::Es, 2) => "El limo se parte en dos y la nueva mitad se abalanza sobre ti. Te absorbe, tu cuerpo se funde y rezuma, y te conviertes en un grumo verde y brillante de limo bobo.",
        (Locale::Es, 10) => "El gato salta y muerde tu alma. Cuando se te nubla la vista, te brotan pelo, bigotes y la sabiduría profunda de tirar cosas de las mesas.",
        (Locale::Es, 11) => "Las espinas del espectro hacen brillar tus venas en verde durante una semana, pero llegas a casa entero.",
        (Locale::Es, 20) => "El gólem te aplasta como una tortita. Despiertas al inicio del sendero, magullado pero no cristalizado.",
        (Locale::Es, 21) => "El corcel se encabrita, y mientras sus cascos bajan, sientes tu columna alargarse, tus brazos fusionarse, tu dignidad retroceder. Ahora eres un sólido cuadrúpedo.",
        (Locale::Es, 30) => "El fuego del dragón cuece tus huesos en escamas. Cuando todo acaba, no recuerdas cómo ser pequeño. Ahora eres un dragón.",
        (Locale::Es, 31) => "El señor de las sombras te vacía hasta el cascarón, pero la piel aguanta. Vuelves al pueblo, todavía humano, todavía vivo — por los pelos.",
        (Locale::Es, _) => enemy.death_blurb,
        (Locale::Ja, 0) => "エルフはあなたを打ち倒し、道端で血を流させる。傷だらけながら、まだ自分のままで家まで這って帰る。",
        (Locale::Ja, 1) => "法律家がエゴをぺしゃんこにする召喚状を手渡す。あなたは相変わらず平凡なまま、足を引きずって帰る。",
        (Locale::Ja, 2) => "スライムが二つに割れ、新しい半身が襲いかかる。吸い込まれ、体は溶けて滲み、緑色に光るぼんやりしたスライムの塊になる。",
        (Locale::Ja, 10) => "猫が飛びかかり、魂をかむ。視界が霞むと、毛、ヒゲ、そして物を机から落とす獣の深い知恵が生えてくる。",
        (Locale::Ja, 11) => "亡霊の棘で一週間ほど血管が緑に光るが、無事に家へたどり着く。",
        (Locale::Ja, 20) => "ゴーレムにぺしゃんこにされる。道の始点で目覚め、ボロボロだが結晶化はしていない。",
        (Locale::Ja, 21) => "軍馬が後脚で立ち上がる。蹄が下りる間に、背骨は伸び、腕は融合し、尊厳は後退するのを感じる。あなたは堅実な四足獣になった。",
        (Locale::Ja, 30) => "竜の炎で骨が鱗に焼き固められる。すべてが終わるころには、小さくいる方法を思い出せない。あなたは竜である。",
        (Locale::Ja, 31) => "影の主はあなたを殻まで吸い尽くすが、皮は持ちこたえる。村に戻る——まだ人間、まだ生きている——かろうじて。",
        (Locale::Ja, _) => enemy.death_blurb,
        (_, _) => unreachable!("fmt_locale normalises non-curated locales"),
    }
}

/// Localized skill name.
pub fn skill_name(locale: Locale, id: u8) -> &'static str {
    match (locale.fmt_locale(), id) {
        (Locale::En, SKILL_SLIME_BODY) => "Slime Body",
        (Locale::Ru, SKILL_SLIME_BODY) => "Тело Слизи",
        (Locale::Fr, SKILL_SLIME_BODY) => "Corps de Slime",
        (Locale::Es, SKILL_SLIME_BODY) => "Cuerpo de Limo",
        (Locale::Ja, SKILL_SLIME_BODY) => "スライムの肉体",
        (Locale::En, SKILL_FELINE_GRACE) => "Feline Grace",
        (Locale::Ru, SKILL_FELINE_GRACE) => "Кошачья грация",
        (Locale::Fr, SKILL_FELINE_GRACE) => "Grâce féline",
        (Locale::Es, SKILL_FELINE_GRACE) => "Gracia felina",
        (Locale::Ja, SKILL_FELINE_GRACE) => "猫のしなやかさ",
        (Locale::En, SKILL_DRAGON_SCALES) => "Dragon Scales",
        (Locale::Ru, SKILL_DRAGON_SCALES) => "Драконья чешуя",
        (Locale::Fr, SKILL_DRAGON_SCALES) => "Écailles de dragon",
        (Locale::Es, SKILL_DRAGON_SCALES) => "Escamas de dragón",
        (Locale::Ja, SKILL_DRAGON_SCALES) => "竜の鱗",
        (Locale::En, SKILL_STEED_HEART) => "Steed Heart",
        (Locale::Ru, SKILL_STEED_HEART) => "Сердце скакуна",
        (Locale::Fr, SKILL_STEED_HEART) => "Cœur de destrier",
        (Locale::Es, SKILL_STEED_HEART) => "Corazón de corcel",
        (Locale::Ja, SKILL_STEED_HEART) => "駿馬の心臓",
        (Locale::En, SKILL_VETERAN) => "Veteran",
        (Locale::Ru, SKILL_VETERAN) => "Ветеран",
        (Locale::Fr, SKILL_VETERAN) => "Vétéran",
        (Locale::Es, SKILL_VETERAN) => "Veterano",
        (Locale::Ja, SKILL_VETERAN) => "ベテラン",
        (Locale::En, SKILL_CHAMPION) => "Champion",
        (Locale::Ru, SKILL_CHAMPION) => "Чемпион",
        (Locale::Fr, SKILL_CHAMPION) => "Champion",
        (Locale::Es, SKILL_CHAMPION) => "Campeón",
        (Locale::Ja, SKILL_CHAMPION) => "チャンピオン",
        _ => "?",
    }
}

pub fn skill_blurb(locale: Locale, id: u8) -> &'static str {
    match (locale.fmt_locale(), id) {
        (Locale::En, SKILL_SLIME_BODY) => "You've been gooey once. The membrane carries over: +10 HP, +3 defence.",
        (Locale::Ru, SKILL_SLIME_BODY) => "Ты уже бывал желеобразным. Мембрана остаётся: +10 ОЗ, +3 защиты.",
        (Locale::Fr, SKILL_SLIME_BODY) => "Tu as déjà été gluant. La membrane reste : +10 PV, +3 défense.",
        (Locale::Es, SKILL_SLIME_BODY) => "Ya fuiste gelatinoso una vez. La membrana queda: +10 PV, +3 de defensa.",
        (Locale::Ja, SKILL_SLIME_BODY) => "かつて粘体だった経験が残る。膜は健在: HP +10、防御 +3。",
        (Locale::En, SKILL_FELINE_GRACE) => "Your reflexes remember the cat: +3 attack.",
        (Locale::Ru, SKILL_FELINE_GRACE) => "Рефлексы помнят кошку: +3 атаки.",
        (Locale::Fr, SKILL_FELINE_GRACE) => "Tes réflexes se souviennent du chat : +3 attaque.",
        (Locale::Es, SKILL_FELINE_GRACE) => "Tus reflejos recuerdan al gato: +3 de ataque.",
        (Locale::Ja, SKILL_FELINE_GRACE) => "猫の反射神経が残る: 攻撃 +3。",
        (Locale::En, SKILL_DRAGON_SCALES) => "Stray scales still cling to your skin: +4 attack, +3 defence.",
        (Locale::Ru, SKILL_DRAGON_SCALES) => "Оставшиеся чешуйки прирастают к коже: +4 атаки, +3 защиты.",
        (Locale::Fr, SKILL_DRAGON_SCALES) => "Quelques écailles s'accrochent encore à ta peau : +4 attaque, +3 défense.",
        (Locale::Es, SKILL_DRAGON_SCALES) => "Algunas escamas siguen pegadas a tu piel: +4 de ataque, +3 de defensa.",
        (Locale::Ja, SKILL_DRAGON_SCALES) => "鱗のかけらが肌に張りついたまま: 攻撃 +4、防御 +3。",
        (Locale::En, SKILL_STEED_HEART) => "A horse's lung capacity outlasts the form: +12 HP, +2 defence.",
        (Locale::Ru, SKILL_STEED_HEART) => "Лошадиная ёмкость лёгких переживает форму: +12 ОЗ, +2 защиты.",
        (Locale::Fr, SKILL_STEED_HEART) => "La capacité pulmonaire du cheval survit à la forme : +12 PV, +2 défense.",
        (Locale::Es, SKILL_STEED_HEART) => "La capacidad pulmonar del caballo sobrevive a la forma: +12 PV, +2 de defensa.",
        (Locale::Ja, SKILL_STEED_HEART) => "馬の肺活量はフォームを超えて残る: HP +12、防御 +2。",
        (Locale::En, SKILL_VETERAN) => "Ten levels of combat experience: +3 attack, +3 defence.",
        (Locale::Ru, SKILL_VETERAN) => "Десять уровней боевого опыта: +3 атаки, +3 защиты.",
        (Locale::Fr, SKILL_VETERAN) => "Dix niveaux d'expérience au combat : +3 attaque, +3 défense.",
        (Locale::Es, SKILL_VETERAN) => "Diez niveles de experiencia en combate: +3 de ataque, +3 de defensa.",
        (Locale::Ja, SKILL_VETERAN) => "10 レベル分の戦闘経験: 攻撃 +3、防御 +3。",
        (Locale::En, SKILL_CHAMPION) => "Twenty levels in, you've earned the title: +5 atk, +5 def, +15 HP.",
        (Locale::Ru, SKILL_CHAMPION) => "За двадцать уровней ты заслужил титул: +5 атк, +5 защ, +15 ОЗ.",
        (Locale::Fr, SKILL_CHAMPION) => "Vingt niveaux plus tard, le titre est mérité : +5 att, +5 déf, +15 PV.",
        (Locale::Es, SKILL_CHAMPION) => "Veinte niveles después, te has ganado el título: +5 atq, +5 def, +15 PV.",
        (Locale::Ja, SKILL_CHAMPION) => "20 レベルでこの称号を獲得: 攻撃 +5、防御 +5、HP +15。",
        _ => "",
    }
}

/// Localized ending name.
pub fn ending_name(locale: Locale, id: u8) -> &'static str {
    match (locale.fmt_locale(), id) {
        (Locale::En, ENDING_VICTORY) => "Hero's Victory",
        (Locale::Ru, ENDING_VICTORY) => "Победа Героя",
        (Locale::Fr, ENDING_VICTORY) => "Victoire du Héros",
        (Locale::Es, ENDING_VICTORY) => "Victoria del Héroe",
        (Locale::Ja, ENDING_VICTORY) => "英雄の勝利",
        (Locale::En, ENDING_DRAGON_LORD) => "Dragon Ascendant",
        (Locale::Ru, ENDING_DRAGON_LORD) => "Восхождение Дракона",
        (Locale::Fr, ENDING_DRAGON_LORD) => "Ascension du Dragon",
        (Locale::Es, ENDING_DRAGON_LORD) => "Ascenso del Dragón",
        (Locale::Ja, ENDING_DRAGON_LORD) => "竜の昇華",
        (Locale::En, ENDING_PILGRIM) => "Pilgrim of Forms",
        (Locale::Ru, ENDING_PILGRIM) => "Странник Форм",
        (Locale::Fr, ENDING_PILGRIM) => "Pèlerin des Formes",
        (Locale::Es, ENDING_PILGRIM) => "Peregrino de las Formas",
        (Locale::Ja, ENDING_PILGRIM) => "フォームの巡礼者",
        (Locale::En, ENDING_QUIET_FARMER) => "Quiet Farmer",
        (Locale::Ru, ENDING_QUIET_FARMER) => "Тихий фермер",
        (Locale::Fr, ENDING_QUIET_FARMER) => "Paysan tranquille",
        (Locale::Es, ENDING_QUIET_FARMER) => "Granjero tranquilo",
        (Locale::Ja, ENDING_QUIET_FARMER) => "静かな農夫",
        _ => "?",
    }
}

pub fn ending_blurb(locale: Locale, id: u8) -> &'static str {
    match (locale.fmt_locale(), id) {
        (Locale::En, ENDING_VICTORY) => "Felled the Shadow Lord with your bare human hands. The kingdom remembers your name.",
        (Locale::Ru, ENDING_VICTORY) => "Сразил Повелителя Теней голыми человеческими руками. Королевство помнит твоё имя.",
        (Locale::Fr, ENDING_VICTORY) => "Tu as abattu le Seigneur des Ombres de tes propres mains humaines. Le royaume retient ton nom.",
        (Locale::Es, ENDING_VICTORY) => "Derribaste al Señor de las Sombras con tus manos humanas desnudas. El reino recuerda tu nombre.",
        (Locale::Ja, ENDING_VICTORY) => "影の主を素手で打ち倒した。王国はあなたの名を覚えている。",
        (Locale::En, ENDING_DRAGON_LORD) => "You came as dragon and left as dragon, but the Shadow Lord's keep is your eyrie now.",
        (Locale::Ru, ENDING_DRAGON_LORD) => "Ты пришёл драконом и ушёл драконом, но крепость Повелителя Теней — теперь твоё гнездо.",
        (Locale::Fr, ENDING_DRAGON_LORD) => "Tu es venu en dragon et reparti en dragon, mais le donjon du Seigneur des Ombres est désormais ton aire.",
        (Locale::Es, ENDING_DRAGON_LORD) => "Llegaste como dragón y partiste como dragón, pero la fortaleza del Señor de las Sombras es ahora tu nido.",
        (Locale::Ja, ENDING_DRAGON_LORD) => "竜として来て竜として去った。影の主の砦はいまやあなたの巣だ。",
        (Locale::En, ENDING_PILGRIM) => "You've worn every shape on the map and decided each one was, technically, also you.",
        (Locale::Ru, ENDING_PILGRIM) => "Ты примерил каждую форму на карте и решил, что любая из них тоже, формально, — ты.",
        (Locale::Fr, ENDING_PILGRIM) => "Tu as porté chaque forme de la carte et décidé que chacune était, techniquement, toi aussi.",
        (Locale::Es, ENDING_PILGRIM) => "Has llevado todas las formas del mapa y decidiste que cada una era, técnicamente, también tú.",
        (Locale::Ja, ENDING_PILGRIM) => "地図にあるすべてのフォームをまとい、どれもが厳密には自分でもあると認めた。",
        (Locale::En, ENDING_QUIET_FARMER) => "Ten thousand bushels of wheat. The Shadow Lord still lurks somewhere, but the harvest is good.",
        (Locale::Ru, ENDING_QUIET_FARMER) => "Десять тысяч мер пшеницы. Повелитель Теней где-то ещё прячется, но урожай хорош.",
        (Locale::Fr, ENDING_QUIET_FARMER) => "Dix mille boisseaux de blé. Le Seigneur des Ombres rôde encore quelque part, mais la récolte est bonne.",
        (Locale::Es, ENDING_QUIET_FARMER) => "Diez mil fanegas de trigo. El Señor de las Sombras sigue acechando en algún lugar, pero la cosecha es buena.",
        (Locale::Ja, ENDING_QUIET_FARMER) => "小麦 1 万ブッシェル。影の主はどこかでまだ潜むが、収穫はよい。",
        _ => "",
    }
}

/// Localized achievement label (chip text).
pub fn achievement_label(locale: Locale, id: u8) -> &'static str {
    match (locale.fmt_locale(), id) {
        (Locale::En, ACH_FIRST_MISSION) => "first mission",
        (Locale::Ru, ACH_FIRST_MISSION) => "первая миссия",
        (Locale::Fr, ACH_FIRST_MISSION) => "première mission",
        (Locale::Es, ACH_FIRST_MISSION) => "primera misión",
        (Locale::Ja, ACH_FIRST_MISSION) => "初ミッション",
        (Locale::En, ACH_BRONZE_GRINDER) => "bronze grinder",
        (Locale::Ru, ACH_BRONZE_GRINDER) => "бронзовый труженик",
        (Locale::Fr, ACH_BRONZE_GRINDER) => "bûcheur de bronze",
        (Locale::Es, ACH_BRONZE_GRINDER) => "currante de bronce",
        (Locale::Ja, ACH_BRONZE_GRINDER) => "ブロンズ周回者",
        (Locale::En, ACH_SILVER_GRINDER) => "silver grinder",
        (Locale::Ru, ACH_SILVER_GRINDER) => "серебряный труженик",
        (Locale::Fr, ACH_SILVER_GRINDER) => "bûcheur d'argent",
        (Locale::Es, ACH_SILVER_GRINDER) => "currante de plata",
        (Locale::Ja, ACH_SILVER_GRINDER) => "シルバー周回者",
        (Locale::En, ACH_GOLD_GRINDER) => "gold grinder",
        (Locale::Ru, ACH_GOLD_GRINDER) => "золотой труженик",
        (Locale::Fr, ACH_GOLD_GRINDER) => "bûcheur d'or",
        (Locale::Es, ACH_GOLD_GRINDER) => "currante de oro",
        (Locale::Ja, ACH_GOLD_GRINDER) => "ゴールド周回者",
        (Locale::En, ACH_FIRST_BLOOD) => "first blood",
        (Locale::Ru, ACH_FIRST_BLOOD) => "первая кровь",
        (Locale::Fr, ACH_FIRST_BLOOD) => "premier sang",
        (Locale::Es, ACH_FIRST_BLOOD) => "primera sangre",
        (Locale::Ja, ACH_FIRST_BLOOD) => "ファーストブラッド",
        (Locale::En, ACH_LIEUTENANT) => "lieutenant",
        (Locale::Ru, ACH_LIEUTENANT) => "лейтенант",
        (Locale::Fr, ACH_LIEUTENANT) => "lieutenant",
        (Locale::Es, ACH_LIEUTENANT) => "teniente",
        (Locale::Ja, ACH_LIEUTENANT) => "中尉",
        (Locale::En, ACH_CAPTAIN) => "captain",
        (Locale::Ru, ACH_CAPTAIN) => "капитан",
        (Locale::Fr, ACH_CAPTAIN) => "capitaine",
        (Locale::Es, ACH_CAPTAIN) => "capitán",
        (Locale::Ja, ACH_CAPTAIN) => "大尉",
        (Locale::En, ACH_TREASURER) => "treasurer",
        (Locale::Ru, ACH_TREASURER) => "казначей",
        (Locale::Fr, ACH_TREASURER) => "trésorier",
        (Locale::Es, ACH_TREASURER) => "tesorero",
        (Locale::Ja, ACH_TREASURER) => "会計係",
        (Locale::En, ACH_SOUL_BOUND) => "soul-bound",
        (Locale::Ru, ACH_SOUL_BOUND) => "связан душой",
        (Locale::Fr, ACH_SOUL_BOUND) => "lié par l'âme",
        (Locale::Es, ACH_SOUL_BOUND) => "ligado por el alma",
        (Locale::Ja, ACH_SOUL_BOUND) => "魂縛り",
        (Locale::En, ACH_FIRST_KILL) => "first kill",
        (Locale::Ru, ACH_FIRST_KILL) => "первое убийство",
        (Locale::Fr, ACH_FIRST_KILL) => "première mise à mort",
        (Locale::Es, ACH_FIRST_KILL) => "primera baja",
        (Locale::Ja, ACH_FIRST_KILL) => "初撃破",
        (Locale::En, ACH_FIRST_LEGENDARY) => "first legendary",
        (Locale::Ru, ACH_FIRST_LEGENDARY) => "первая легендарка",
        (Locale::Fr, ACH_FIRST_LEGENDARY) => "premier légendaire",
        (Locale::Es, ACH_FIRST_LEGENDARY) => "primer legendario",
        (Locale::Ja, ACH_FIRST_LEGENDARY) => "初の伝説",
        _ => "?",
    }
}

/// Localized achievement unlock criterion (tooltip body / toast body).
/// Mirrors `shared::achievement_reason` but routes through the
/// `Locale`-aware label formatter.
pub fn achievement_reason(locale: Locale, id: u8) -> String {
    for (aid, check) in ACHIEVEMENT_TABLE {
        if *aid == id {
            return match (locale.fmt_locale(), *check) {
                (Locale::En, AchievementCheck::Missions(n)) => format!("Run {n} missions"),
                (Locale::Ru, AchievementCheck::Missions(n)) => format!("Пройди {n} миссий"),
                (Locale::Fr, AchievementCheck::Missions(n)) => format!("Effectuez {n} missions"),
                (Locale::Es, AchievementCheck::Missions(n)) => format!("Completa {n} misiones"),
                (Locale::Ja, AchievementCheck::Missions(n)) => format!("ミッションを {n} 回こなす"),
                (Locale::En, AchievementCheck::BossDamage(n)) => format!("Deal {n} damage to the World Boss"),
                (Locale::Ru, AchievementCheck::BossDamage(n)) => format!("Нанеси {n} урона Мировому Боссу"),
                (Locale::Fr, AchievementCheck::BossDamage(n)) => format!("Infligez {n} dégâts au Boss du Monde"),
                (Locale::Es, AchievementCheck::BossDamage(n)) => format!("Inflige {n} de daño al Jefe del Mundo"),
                (Locale::Ja, AchievementCheck::BossDamage(n)) => format!("ワールドボスに {n} ダメージを与える"),
                (Locale::En, AchievementCheck::Gold(n)) => format!("Accumulate {n} gold"),
                (Locale::Ru, AchievementCheck::Gold(n)) => format!("Накопи {n} золота"),
                (Locale::Fr, AchievementCheck::Gold(n)) => format!("Accumulez {n} or"),
                (Locale::Es, AchievementCheck::Gold(n)) => format!("Acumula {n} de oro"),
                (Locale::Ja, AchievementCheck::Gold(n)) => format!("金を {n} 貯める"),
                (Locale::En, AchievementCheck::Essence(n)) => format!("Accumulate {n} essence"),
                (Locale::Ru, AchievementCheck::Essence(n)) => format!("Накопи {n} эссенции"),
                (Locale::Fr, AchievementCheck::Essence(n)) => format!("Accumulez {n} essence"),
                (Locale::Es, AchievementCheck::Essence(n)) => format!("Acumula {n} de esencia"),
                (Locale::Ja, AchievementCheck::Essence(n)) => format!("精を {n} 貯める"),
                (Locale::En, AchievementCheck::WinCount(n)) => format!("Win {n} encounters"),
                (Locale::Ru, AchievementCheck::WinCount(n)) => format!("Выиграй {n} сражений"),
                (Locale::Fr, AchievementCheck::WinCount(n)) => format!("Remportez {n} rencontres"),
                (Locale::Es, AchievementCheck::WinCount(n)) => format!("Gana {n} encuentros"),
                (Locale::Ja, AchievementCheck::WinCount(n)) => format!("戦闘で {n} 勝する"),
                (Locale::En, AchievementCheck::LegendaryEquipped) => "Equip a Legendary (T4) item".into(),
                (Locale::Ru, AchievementCheck::LegendaryEquipped) => "Надень Легендарный (T4) предмет".into(),
                (Locale::Fr, AchievementCheck::LegendaryEquipped) => "Équipez un objet Légendaire (T4)".into(),
                (Locale::Es, AchievementCheck::LegendaryEquipped) => "Equípate un objeto Legendario (T4)".into(),
                (Locale::Ja, AchievementCheck::LegendaryEquipped) => "伝説 (T4) のアイテムを装備する".into(),
                (_, _) => unreachable!("fmt_locale normalises non-curated locales"),
            };
        }
    }
    match locale.fmt_locale() {
        Locale::En => "unknown achievement".into(),
        Locale::Ru => "неизвестное достижение".into(),
        Locale::Fr => "succès inconnu".into(),
        Locale::Es => "logro desconocido".into(),
        Locale::Ja => "不明な実績".into(),
        _ => unreachable!("fmt_locale normalises non-curated locales"),
    }
}

/// Localized gear slot label (Helm / Шлем / etc.). Idx is the slot
/// index used by `SLOT_NAMES`. Falls back to the shared crate's
/// English label for out-of-range indices — defensive only, real
/// callers always pass a valid 0..SLOT_COUNT index.
pub fn slot_name(locale: Locale, idx: usize) -> &'static str {
    const RU_SLOTS: [&str; SLOT_COUNT] =
        ["Шлем", "Плащ", "Нагрудник", "Штаны", "Щит", "Меч", "Сапоги", "Кольцо"];
    const FR_SLOTS: [&str; SLOT_COUNT] =
        ["Casque", "Cape", "Plastron", "Pantalon", "Bouclier", "Épée", "Bottes", "Anneau"];
    const ES_SLOTS: [&str; SLOT_COUNT] =
        ["Casco", "Capa", "Pechera", "Pantalón", "Escudo", "Espada", "Botas", "Anillo"];
    const JA_SLOTS: [&str; SLOT_COUNT] =
        ["兜", "マント", "胸当て", "ズボン", "盾", "剣", "ブーツ", "指輪"];
    match locale.fmt_locale() {
        Locale::En => shared::SLOT_NAMES.get(idx).copied().unwrap_or("?"),
        Locale::Ru => RU_SLOTS.get(idx).copied().unwrap_or("?"),
        Locale::Fr => FR_SLOTS.get(idx).copied().unwrap_or("?"),
        Locale::Es => ES_SLOTS.get(idx).copied().unwrap_or("?"),
        Locale::Ja => JA_SLOTS.get(idx).copied().unwrap_or("?"),
        _ => unreachable!("fmt_locale normalises non-curated locales"),
    }
}

/// Localized gear-tier prefix (Worn / Изношенный / etc.).
pub fn tier_prefix(locale: Locale, tier: u8) -> &'static str {
    let idx = tier.saturating_sub(1) as usize;
    const RU_TIERS: [&str; 4] = ["Изношенный", "Полированный", "Рунный", "Легендарный"];
    const FR_TIERS: [&str; 4] = ["Usé", "Poli", "Runique", "Légendaire"];
    const ES_TIERS: [&str; 4] = ["Desgastado", "Pulido", "Rúnico", "Legendario"];
    const JA_TIERS: [&str; 4] = ["古びた", "磨かれた", "ルーンの", "伝説の"];
    match locale.fmt_locale() {
        Locale::En => shared::TIER_PREFIXES.get(idx).copied().unwrap_or("?"),
        Locale::Ru => RU_TIERS.get(idx).copied().unwrap_or("?"),
        Locale::Fr => FR_TIERS.get(idx).copied().unwrap_or("?"),
        Locale::Es => ES_TIERS.get(idx).copied().unwrap_or("?"),
        Locale::Ja => JA_TIERS.get(idx).copied().unwrap_or("?"),
        _ => unreachable!("fmt_locale normalises non-curated locales"),
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
    let area = shared::current_area_def(inv);
    let name_l = area_name(locale, &area);
    let is_wilds = area_id >= shared::WILDS_AREA_BASE;
    let chap_no = if is_wilds {
        area_id
            .saturating_sub(shared::WILDS_AREA_BASE)
            .saturating_add(1)
    } else {
        area_id.saturating_add(1)
    };
    let title = match (locale.fmt_locale(), is_wilds) {
        (Locale::En, false) => format!("Chapter {chap_no} · {name_l}"),
        (Locale::Ru, false) => format!("Глава {chap_no} · {name_l}"),
        (Locale::Fr, false) => format!("Chapitre {chap_no} · {name_l}"),
        (Locale::Es, false) => format!("Capítulo {chap_no} · {name_l}"),
        (Locale::Ja, false) => format!("第 {chap_no} 章 · {name_l}"),
        (Locale::En, true) => format!("Wilds {chap_no} · {name_l}"),
        (Locale::Ru, true) => format!("Дикие земли {chap_no} · {name_l}"),
        (Locale::Fr, true) => format!("Terres sauvages {chap_no} · {name_l}"),
        (Locale::Es, true) => format!("Tierras salvajes {chap_no} · {name_l}"),
        (Locale::Ja, true) => format!("ワイルド {chap_no} · {name_l}"),
        (_, _) => unreachable!("fmt_locale normalises non-curated locales"),
    };
    let body: String = match (locale.fmt_locale(), area_id) {
        (Locale::En, 0) if inv.mission_count == 0 =>
            "Your father points east. \"Be strong, and bring the boss down.\" The fields outside the village are quiet — for now. Run a mission to begin.".into(),
        (Locale::En, 0) =>
            "You're running errands at the edge of the fields. Each mission trickles gold and essence into the lockbox the delegate keeps for you on the node.".into(),
        (Locale::Ru, 0) if inv.mission_count == 0 =>
            "Отец указывает на восток. «Будь сильным и одолей босса». Поля за деревней пока тихи — запусти миссию, чтобы начать.".into(),
        (Locale::Ru, 0) =>
            "Ты разбираешь мелочи на краю полей. Каждая миссия по чуть-чуть капает золотом и эссенцией в сейф, который делегат держит на узле.".into(),
        (Locale::Fr, 0) if inv.mission_count == 0 =>
            "Ton père pointe vers l'est. « Sois fort, et abats le boss. » Les champs hors du village sont calmes — pour l'instant. Lance une mission pour commencer.".into(),
        (Locale::Fr, 0) =>
            "Tu fais des courses à la lisière des champs. Chaque mission fait couler un peu d'or et d'essence dans le coffre que le délégué garde pour toi sur le nœud.".into(),
        (Locale::Es, 0) if inv.mission_count == 0 =>
            "Tu padre señala al este. «Sé fuerte y derriba al jefe.» Los campos fuera del pueblo están en calma — por ahora. Lanza una misión para empezar.".into(),
        (Locale::Es, 0) =>
            "Haces recados al borde de los campos. Cada misión gotea oro y esencia en la caja fuerte que el delegado guarda para ti en el nodo.".into(),
        (Locale::Ja, 0) if inv.mission_count == 0 =>
            "父が東を指す。「強くなれ、ボスを倒せ。」 村の外の畑は静かだ — 今のところ。ミッションを開始して旅立とう。".into(),
        (Locale::Ja, 0) =>
            "畑の端で雑用をこなす。各ミッションは、デリゲートがノード上の金庫に少しずつ金と精を貯めていく。".into(),
        (Locale::En, 1) =>
            "Word of your exploits has reached the next biome. The forest paths yield more essence, but the World Boss begins to stir as every player chips at its HP.".into(),
        (Locale::Ru, 1) =>
            "Слухи о твоих подвигах добрались до следующего биома. Лесные тропы дают больше эссенции, но Мировой Босс начинает шевелиться, пока каждый игрок откусывает кусочки от его ОЗ.".into(),
        (Locale::Fr, 1) =>
            "Le bruit de tes exploits a atteint le biome suivant. Les sentiers de la forêt rendent plus d'essence, mais le Boss du Monde commence à s'agiter à mesure que chaque joueur entame ses PV.".into(),
        (Locale::Es, 1) =>
            "El rumor de tus hazañas ha llegado al siguiente bioma. Las sendas del bosque rinden más esencia, pero el Jefe del Mundo empieza a removerse mientras cada jugador mella sus PV.".into(),
        (Locale::Ja, 1) =>
            "あなたの活躍の噂が次の生物群系に届いた。森の道はより多くの精を生むが、各プレイヤーが HP を削るにつれ、ワールドボスは身じろぎ始める。".into(),
        (Locale::En, 2) =>
            "Merchants pay handsomely at the pass, and the loot scales. Other adventurers across the network are converging on the same foe — every hit is mirrored in the global HP gauge.".into(),
        (Locale::Ru, 2) =>
            "Купцы на перевале платят щедро, и добыча масштабируется. Другие искатели приключений по всей сети сходятся к одному и тому же врагу — каждый удар отражается в общей шкале ОЗ.".into(),
        (Locale::Fr, 2) =>
            "Les marchands paient grassement au col, et le butin évolue. D'autres aventuriers à travers le réseau convergent sur le même ennemi — chaque coup est répercuté sur la jauge de PV globale.".into(),
        (Locale::Es, 2) =>
            "Los mercaderes pagan generosamente en el paso, y el botín escala. Otros aventureros por toda la red convergen en el mismo enemigo — cada golpe se refleja en el medidor de PV global.".into(),
        (Locale::Ja, 2) =>
            "峠の商人は気前よく払い、戦利品もスケールする。ネット越しの他の冒険者たちが同じ敵に集まり — 一撃ごとに共有 HP ゲージに反映される。".into(),
        (Locale::En, 3) =>
            "You've reached the inner sanctum. Damage-heavy work — every blow you land is mirrored in the World Boss HP gauge that every connected player sees in real time.".into(),
        (Locale::Ru, 3) =>
            "Ты добрался до внутреннего святилища. Тяжёлая работа по урону — каждый твой удар отражается в шкале ОЗ Мирового Босса, которую каждый подключённый игрок видит в реальном времени.".into(),
        (Locale::Fr, 3) =>
            "Tu as atteint le sanctuaire intérieur. Travail axé sur les dégâts — chaque coup porté est répercuté sur la jauge de PV du Boss du Monde, que chaque joueur connecté voit en temps réel.".into(),
        (Locale::Es, 3) =>
            "Has llegado al santuario interior. Trabajo de daño pesado — cada golpe que asestas se refleja en el medidor de PV del Jefe del Mundo, que ve cada jugador conectado en tiempo real.".into(),
        (Locale::Ja, 3) =>
            "内陣にたどり着いた。ダメージ重視の戦い — あなたの一撃ごとに、すべての接続プレイヤーがリアルタイムで見るワールドボス HP ゲージが動く。".into(),
        _ => area_blurb(locale, &area).to_string(),
    };
    (chap_no, title, body)
}

/// Plot word-list expansion — six-slot Mad Libs source. Returns the
/// (home, macguffin, villain, method, destination) tuple already
/// localized; the consumer passes these into
/// `Locale::fmt_plot_backstory`.
pub fn plot_tuple_l10n(locale: Locale, seed: u32) -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
    let s = seed as u64;
    let idx = locale_idx(locale);
    let home = WORDS_HOMES[idx][(s % WORDS_HOMES[0].len() as u64) as usize];
    let mac = WORDS_MACGUFFINS[idx][((s / 7) % WORDS_MACGUFFINS[0].len() as u64) as usize];
    let vil = WORDS_VILLAINS[idx][((s / 53) % WORDS_VILLAINS[0].len() as u64) as usize];
    let mthd = WORDS_METHODS[idx][((s / 211) % WORDS_METHODS[0].len() as u64) as usize];
    let dest = WORDS_DESTINATIONS[idx][((s / 1009) % WORDS_DESTINATIONS[0].len() as u64) as usize];
    (home, mac, vil, mthd, dest)
}

fn locale_idx(locale: Locale) -> usize {
    match locale.fmt_locale() {
        Locale::En => 0,
        Locale::Ru => 1,
        Locale::Fr => 2,
        Locale::Es => 3,
        Locale::Ja => 4,
        _ => unreachable!("fmt_locale normalises non-curated locales"),
    }
}

// Word lists mirror `shared::PLOT_*` for English and add per-locale
// parallel sets. Index order: En, Ru, Fr, Es, Ja. Each entry is
// inflected so it reads grammatically inside the sentence template
// in `Locale::fmt_plot_backstory`.
const WORDS_HOMES: [&[&str]; 5] = [
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
    &[
        "le château flottant de Bassang",
        "le hameau de Kirkwent",
        "le village de Verlande",
        "le port de Selrive",
        "la fagne de Voile-d'Épines",
        "la ville engloutie de Felgrave",
    ],
    &[
        "el castillo flotante de Sangrelago",
        "la aldea de Kirkwent",
        "el pueblo de Verde Brezal",
        "el puerto de Salalcance",
        "la ciénaga de Velo de Espino",
        "el pueblo ahogado de Felgrave",
    ],
    &[
        "ブラッドプールの浮遊城",
        "カークウェントの集落",
        "グリーンムーア村",
        "ソルトリーチ港",
        "ソーンヴェイルの泥地",
        "水没都市フェルグレイヴ",
    ],
];

const WORDS_MACGUFFINS: [&[&str]; 5] = [
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
    &[
        "Coffre aux Chats",
        "amulette sacrée de la Lumière Brisée",
        "Dernier Œuf",
        "seule montre encore en marche au monde",
        "Cœur de la Montagne",
        "nom de ta mère",
    ],
    &[
        "Cofre de los Gatos",
        "amuleto sagrado de la Luz Quebrada",
        "Último Huevo",
        "único reloj del mundo que aún funciona",
        "Corazón de la Montaña",
        "nombre de tu madre",
    ],
    &[
        "猫の小箱",
        "砕け光の聖アミュレット",
        "最後の卵",
        "世界で唯一動く時計",
        "山の心臓",
        "あなたの母の名",
    ],
];

const WORDS_VILLAINS: [&[&str]; 5] = [
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
    &[
        "Seigneur des Ténèbres",
        "Roi Murmurant",
        "Liche de la Plaine de Sel",
        "Conseil des Ombres",
        "Faim Errante",
        "Glouton Couronné",
    ],
    &[
        "Señor Oscuro",
        "Rey Susurrante",
        "Liche de la Llanura de Sal",
        "Consejo de Sombras",
        "Hambre Errante",
        "Glotón Coronado",
    ],
    &[
        "闇の主",
        "ささやきの王",
        "塩平原のリッチ",
        "影の評議会",
        "彷徨う飢え",
        "戴冠の暴食者",
    ],
];

const WORDS_METHODS: [&[&str]; 5] = [
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
    &[
        "Pluie de Destruction",
        "razzia de minuit",
        "sortilège d'oubli",
        "marché terrible à minuit",
        "appel venu des abysses",
        "registre des serments rompus",
    ],
    &[
        "Lluvia de Destrucción",
        "incursión de medianoche",
        "hechizo del olvido",
        "trato terrible a medianoche",
        "llamada de las profundidades",
        "libro de juramentos rotos",
    ],
    &[
        "破壊の雨",
        "真夜中の襲撃",
        "忘却の呪い",
        "深夜の恐ろしい取引",
        "深淵からの召喚",
        "破られた誓いの台帳",
    ],
];

const WORDS_DESTINATIONS: [&[&str]; 5] = [
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
    &[
        "l'Île dans le Ciel",
        "la Forêt des Portes",
        "la Tour aux Flèches",
        "l'Abîme en Dessous",
        "le Pavillon aux Miroirs",
        "la cité des chambres closes",
    ],
    &[
        "la Isla en el Cielo",
        "el Bosque de Puertas",
        "la Torre de Agujas",
        "el Abismo de Abajo",
        "el Pabellón de Espejos",
        "la ciudad de las salas cerradas",
    ],
    &[
        "天空の島",
        "扉の森",
        "尖塔の塔",
        "下なる深淵",
        "鏡の宮殿",
        "閉ざされた部屋の街",
    ],
];
