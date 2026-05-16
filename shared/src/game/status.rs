//! Status pills shown in the player chip — pure UI projection.

pub const STATUS_DEFEATED: u8 = 0;
pub const STATUS_ADVENTURING: u8 = 1;
pub const STATUS_FOCUSING: u8 = 2;
pub const STATUS_READY: u8 = 3;
pub const STATUS_RECOVERING: u8 = 4;
/// Estate is the active idle action — workers accrue yield while
/// auto-mission is paused (§5.6 mutually-exclusive rule).
pub const STATUS_ESTATE: u8 = 5;

pub fn status_label(s: u8) -> &'static str {
    match s {
        STATUS_DEFEATED => "DEFEATED",
        STATUS_ADVENTURING => "ADVENTURING",
        STATUS_FOCUSING => "FOCUSING",
        STATUS_RECOVERING => "RECOVERING",
        STATUS_READY => "READY",
        STATUS_ESTATE => "ESTATE",
        _ => "?",
    }
}
