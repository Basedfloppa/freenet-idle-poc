//! Per-zone activities (backlog A1). Non-combat repeatable
//! actions themed to each area: only the activity matching the
//! player's currently-selected area can be picked as the active
//! idle action. Resources produced are the existing currencies
//! (wheat / gold / essence / insight) — no new resource fields on
//! the inventory, keeps the schema bump lean.

use super::EstateResource;

/// One activity definition. The id is the wire token sent in
/// `AppRequest::SetActivity` — pinned so future additions append
/// at the end. `0` is reserved as "no activity selected".
#[derive(Debug, Clone, Copy)]
pub struct ActivityDef {
    pub id: u8,
    pub area_id: u8,
    pub name: &'static str,
    pub produces: ActivityResource,
    /// Per-second yield while this activity is the selected idle
    /// action. Multiplied by elapsed-seconds in `tick_activity`;
    /// no per-form affinity stack — that's Estate's lever.
    pub yield_per_sec: u64,
    /// Minimum hero level. Reuses the existing area-gate semantics
    /// so e.g. "Decode sigils" in Astral isn't pickable until the
    /// player can actually visit Astral.
    pub min_level: u64,
}

/// Which currency an activity produces. Reuses `EstateResource`
/// for the three shared currencies (wheat/gold/essence) and adds
/// `Insight` for the Astral path (backlog B5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityResource {
    Wheat,
    Gold,
    Essence,
    Insight,
}

impl From<EstateResource> for ActivityResource {
    fn from(r: EstateResource) -> Self {
        match r {
            EstateResource::Wheat => Self::Wheat,
            EstateResource::Gold => Self::Gold,
            EstateResource::Essence => Self::Essence,
        }
    }
}

pub const ACTIVITY_NONE: u8 = 0;

/// Activity table. Two activities per area in the MVP — small
/// surface so we can balance numbers without a big design pass.
pub const ACTIVITIES: &[ActivityDef] = &[
    // Village Fields (id 0) — easy income.
    ActivityDef {
        id: 1,
        area_id: 0,
        name: "Tend the farm",
        produces: ActivityResource::Wheat,
        yield_per_sec: 1,
        min_level: 1,
    },
    ActivityDef {
        id: 2,
        area_id: 0,
        name: "Pray at the chapel",
        produces: ActivityResource::Essence,
        yield_per_sec: 1,
        min_level: 1,
    },
    // Forest Road (id 1).
    ActivityDef {
        id: 3,
        area_id: 1,
        name: "Forage berries",
        produces: ActivityResource::Wheat,
        yield_per_sec: 3,
        min_level: 3,
    },
    ActivityDef {
        id: 4,
        area_id: 1,
        name: "Track game",
        produces: ActivityResource::Gold,
        yield_per_sec: 1,
        min_level: 3,
    },
    // Mountain Pass (id 2).
    ActivityDef {
        id: 5,
        area_id: 2,
        name: "Mine ore",
        produces: ActivityResource::Gold,
        yield_per_sec: 2,
        min_level: 6,
    },
    ActivityDef {
        id: 6,
        area_id: 2,
        name: "Meditate",
        produces: ActivityResource::Essence,
        yield_per_sec: 2,
        min_level: 6,
    },
    // Boss's Lair (id 3) — no idle activity (the player is
    // expected to be actively fighting here).
    // Deep Forest (id 4).
    ActivityDef {
        id: 7,
        area_id: 4,
        name: "Channel essence",
        produces: ActivityResource::Essence,
        yield_per_sec: 4,
        min_level: 4,
    },
    ActivityDef {
        id: 8,
        area_id: 4,
        name: "Decode sigils",
        produces: ActivityResource::Insight,
        yield_per_sec: 1,
        min_level: 4,
    },
    // Snowfields (id 5).
    ActivityDef {
        id: 9,
        area_id: 5,
        name: "Quarry stone",
        produces: ActivityResource::Gold,
        yield_per_sec: 3,
        min_level: 8,
    },
];

pub fn activity_def(id: u8) -> Option<&'static ActivityDef> {
    ACTIVITIES.iter().find(|a| a.id == id)
}

/// All activities available in `area_id`. UI uses this to render
/// the per-area activity panel; `set_activity` validates against
/// the same list.
pub fn activities_for_area(area_id: u8) -> impl Iterator<Item = &'static ActivityDef> {
    ACTIVITIES.iter().filter(move |a| a.area_id == area_id)
}
