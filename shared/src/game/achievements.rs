//! Achievement IDs + the criterion table that the delegate's
//! `progression` module checks each tick. Chip tooltips and
//! unlock toasts read `achievement_reason` from the same table.

pub const ACH_FIRST_MISSION: u8 = 0;
pub const ACH_BRONZE_GRINDER: u8 = 1;
pub const ACH_SILVER_GRINDER: u8 = 2;
pub const ACH_GOLD_GRINDER: u8 = 3;
pub const ACH_FIRST_BLOOD: u8 = 4;
pub const ACH_LIEUTENANT: u8 = 5;
pub const ACH_CAPTAIN: u8 = 6;
pub const ACH_TREASURER: u8 = 7;
pub const ACH_SOUL_BOUND: u8 = 8;
pub const ACH_FIRST_KILL: u8 = 9;
pub const ACH_FIRST_LEGENDARY: u8 = 10;

pub fn achievement_label(id: u8) -> &'static str {
    match id {
        ACH_FIRST_MISSION => "first mission",
        ACH_BRONZE_GRINDER => "bronze grinder",
        ACH_SILVER_GRINDER => "silver grinder",
        ACH_GOLD_GRINDER => "gold grinder",
        ACH_FIRST_BLOOD => "first blood",
        ACH_LIEUTENANT => "lieutenant",
        ACH_CAPTAIN => "captain",
        ACH_TREASURER => "treasurer",
        ACH_SOUL_BOUND => "soul-bound",
        ACH_FIRST_KILL => "first kill",
        ACH_FIRST_LEGENDARY => "first legendary",
        _ => "?",
    }
}

/// Human-readable unlock criterion for an achievement, computed
/// from the entry in `ACHIEVEMENT_TABLE`. Used by the chip tooltip
/// and the unlock toast — single source of truth, so adding a new
/// achievement check requires only one line in the table.
pub fn achievement_reason(id: u8) -> String {
    for (aid, check) in ACHIEVEMENT_TABLE {
        if *aid == id {
            return match check {
                AchievementCheck::Missions(n) => format!("Run {n} missions"),
                AchievementCheck::BossDamage(n) => format!("Deal {n} damage to the World Boss"),
                AchievementCheck::Gold(n) => format!("Accumulate {n} gold"),
                AchievementCheck::Essence(n) => format!("Accumulate {n} essence"),
                AchievementCheck::WinCount(n) => format!("Win {n} encounters"),
                AchievementCheck::LegendaryEquipped => "Equip a Legendary (T4) item".into(),
            };
        }
    }
    "unknown achievement".into()
}

#[derive(Debug, Clone, Copy)]
pub enum AchievementCheck {
    Missions(u64),
    BossDamage(u64),
    Gold(u64),
    Essence(u64),
    WinCount(u64),
    LegendaryEquipped,
}

pub const ACHIEVEMENT_TABLE: &[(u8, AchievementCheck)] = &[
    (ACH_FIRST_MISSION, AchievementCheck::Missions(1)),
    (ACH_BRONZE_GRINDER, AchievementCheck::Missions(10)),
    (ACH_SILVER_GRINDER, AchievementCheck::Missions(50)),
    (ACH_GOLD_GRINDER, AchievementCheck::Missions(100)),
    (ACH_FIRST_BLOOD, AchievementCheck::BossDamage(1)),
    (ACH_LIEUTENANT, AchievementCheck::BossDamage(50)),
    (ACH_CAPTAIN, AchievementCheck::BossDamage(100)),
    (ACH_TREASURER, AchievementCheck::Gold(1_000)),
    (ACH_SOUL_BOUND, AchievementCheck::Essence(500)),
    (ACH_FIRST_KILL, AchievementCheck::WinCount(1)),
    (ACH_FIRST_LEGENDARY, AchievementCheck::LegendaryEquipped),
];
