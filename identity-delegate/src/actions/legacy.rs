//! Legacy / Epoch handlers (backlog C1, MVP scope). Awards stars
//! on every `STARS_PER_N_LEVELS` levels the player has earned,
//! lets them spend stars on node levels, and offers a soft-reset
//! Ascend action.
//!
//! Contract-side opt-in boss attack + global era rewards are not
//! yet wired — that's a follow-up requiring contract cooperation.
//! In the meantime, personal level milestones drive the loop so
//! the spend tree and Ascend are exercisable in isolation.

use freenet_stdlib::prelude::*;

use shared::{level_of, EstateState, Inventory, LegacyNode, STARS_PER_N_LEVELS};

use crate::progression::check_achievements;
use crate::state::{enter_action, load_inventory_raw, save_inventory};

/// Award any pending stars based on the player's current level vs
/// the `last_awarded_level` watermark. Idempotent on repeat calls
/// — only crossing a new milestone level adds stars.
///
/// Called from `save_inventory` (via a hook below) so every state
/// mutation that bumps the player's level also pays out the
/// matching stars without each call site having to remember to
/// poke the legacy ledger.
pub fn award_pending_stars(inv: &mut Inventory) {
    let lvl = level_of(inv);
    let prior = inv.legacy.last_awarded_level;
    if lvl <= prior {
        return;
    }
    let prior_milestone = prior / STARS_PER_N_LEVELS;
    let new_milestone = lvl / STARS_PER_N_LEVELS;
    if new_milestone > prior_milestone {
        let gained = new_milestone - prior_milestone;
        inv.legacy.stars = inv.legacy.stars.saturating_add(gained);
    }
    inv.legacy.last_awarded_level = lvl;
}

pub fn buy_legacy_node(
    ctx: &mut DelegateCtx,
    node_id: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let node = LegacyNode::from_id(node_id)
        .ok_or_else(|| format!("unknown legacy node {node_id}"))?;
    award_pending_stars(&mut inv);
    let current_level = inv.legacy.node_level(node);
    let cost = node.next_cost(current_level);
    if inv.legacy.stars < cost {
        return Err(format!(
            "not enough stars: need {cost}, have {}",
            inv.legacy.stars
        ));
    }
    inv.legacy.stars -= cost;
    let entry = inv.legacy.nodes.entry(node.id()).or_insert(0);
    *entry = entry.saturating_add(1);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Soft-reset: clear gold, gear (equipped + stash), Estate workers,
/// active battle, current area. Also resets the player's level/XP
/// and area clear counters so the loop genuinely restarts —
/// post-reset the milestone math sees the new ceiling via the
/// existing `last_awarded_level` watermark and only awards stars
/// for crossing levels above the pre-ascend high-water mark.
pub fn ascend(ctx: &mut DelegateCtx, now_ms: u64) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    // One final pending-star award against the pre-reset state so
    // the player doesn't lose a milestone they crossed during the
    // run that's being ascended out of.
    award_pending_stars(&mut inv);
    // Reset run state. Levels/XP, mission_count and area_clears get
    // wiped too so the world map re-gates from the village — the
    // ascend reward is the permanent legacy ledger, not retained
    // mid-game progress.
    inv.base.base.base.gold = 0;
    inv.base.base.base.unequipped.clear();
    inv.base.base.base.equipped = [None; shared::SLOT_COUNT];
    inv.base.base.base.potions = 0;
    inv.base.base.base.fireballs = 0;
    inv.base.base.base.current_hp = shared::STARTING_HP;
    inv.base.base.base.current_battle = None;
    inv.base.base.base.current_area = 0;
    inv.base.base.base.wheat = 0;
    inv.base.base.base.auto_run_enabled = false;
    inv.base.base.base.auto_last_tick_ms = 0;
    inv.base.base.base.last_catchup = None;
    inv.base.base.base.experience = 0;
    inv.base.base.base.mission_count = 0;
    inv.base.base.area_clears.clear();
    inv.base.estate = EstateState::default();
    inv.base.idle_action = shared::IDLE_ACTION_NONE;
    inv.legacy.ascend_count = inv.legacy.ascend_count.saturating_add(1);
    // Reset the per-life awarded-level watermark so new levels
    // earned post-ascend re-trigger star milestones from 0.
    inv.legacy.last_awarded_level = 0;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

