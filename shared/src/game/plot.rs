//! Procedural opening backstory — seeded by `Inventory.plot_seed`
//! so each fresh account gets a different home/villain/macguffin
//! mashup that stays stable for the rest of that save.

pub const PLOT_HOMES: &[&str] = &[
    "floating castle of Bloodpool",
    "hamlet of Kirkwent",
    "village of Greenmoor",
    "port of Saltreach",
    "mire of Thornveil",
    "drowned town of Felgrave",
];
pub const PLOT_MACGUFFINS: &[&str] = &[
    "Chest of Cats",
    "sacred amulet of Sundered Light",
    "Last Egg",
    "world's only working watch",
    "Heart of the Mountain",
    "name of your mother",
];
pub const PLOT_VILLAINS: &[&str] = &[
    "Dark Lord",
    "Whispering King",
    "Lich of the Salt Plain",
    "Shadow Council",
    "Wandering Hunger",
    "Crowned Glutton",
];
pub const PLOT_METHODS: &[&str] = &[
    "Rain of Destruction",
    "midnight raid",
    "hex of forgetting",
    "terrible bargain at midnight",
    "summons from the deep",
    "ledger of broken oaths",
];
pub const PLOT_FINAL_LOCATIONS: &[&str] = &[
    "Island in the Sky",
    "Forest of Doors",
    "Tower of Spires",
    "Abyss Below",
    "Mirror Pavilion",
    "city of locked rooms",
];

pub fn plot_tuple(
    seed: u32,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    let s = seed as u64;
    (
        PLOT_HOMES[(s % PLOT_HOMES.len() as u64) as usize],
        PLOT_MACGUFFINS[((s / 7) % PLOT_MACGUFFINS.len() as u64) as usize],
        PLOT_VILLAINS[((s / 53) % PLOT_VILLAINS.len() as u64) as usize],
        PLOT_METHODS[((s / 211) % PLOT_METHODS.len() as u64) as usize],
        PLOT_FINAL_LOCATIONS[((s / 1009) % PLOT_FINAL_LOCATIONS.len() as u64) as usize],
    )
}
