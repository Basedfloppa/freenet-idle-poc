//! Per-tab views split out of `render_core` for readability. Each
//! function returns the full `<>...</>` fragment for one tab.

mod achievements;
mod help;

pub use achievements::render_achievements_tab;
pub use help::render_help_tab;
