//! Insight currency handlers (backlog B5, MVP scope). Awards one
//! insight per `INSIGHT_PER_MISSIONS` missions completed (the
//! `last_awarded_mission` watermark mirrors the Legacy ascend-
//! resistant trick) and exposes a tiny three-node spend tree.

use freenet_stdlib::prelude::*;

use shared::{InsightNode, Inventory, INSIGHT_PER_MISSIONS};

use crate::state::{enter_action, load_inventory_raw, save_inventory};

/// Idempotent — re-running on the same `mission_count` is a
/// no-op. Hooked into `save_inventory` like `award_pending_stars`.
pub fn award_pending_insight(inv: &mut Inventory) {
    let mc = inv.mission_count;
    let prior = inv.insight.last_awarded_mission;
    if mc <= prior {
        return;
    }
    let prior_milestone = prior / INSIGHT_PER_MISSIONS;
    let new_milestone = mc / INSIGHT_PER_MISSIONS;
    if new_milestone > prior_milestone {
        let gained = new_milestone - prior_milestone;
        inv.insight.balance = inv.insight.balance.saturating_add(gained);
    }
    inv.insight.last_awarded_mission = mc;
}

pub fn buy_insight_node(
    ctx: &mut DelegateCtx,
    node_id: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    buy_insight_node_n(ctx, node_id, 1, now_ms)
}

/// Bulk-buy variant. `count == 0` = "buy as many as insight
/// allows" (capped at 100). Same all-or-something semantics as
/// `buy_legacy_node_n`: if zero levels are affordable, fail.
pub fn buy_insight_node_n(
    ctx: &mut DelegateCtx,
    node_id: u8,
    count: u32,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let node = InsightNode::from_id(node_id)
        .ok_or_else(|| format!("unknown insight node {node_id}"))?;
    award_pending_insight(&mut inv);
    let cap: u32 = if count == 0 { 100 } else { count.min(100) };
    let mut bought = 0u32;
    while bought < cap {
        let current_level = inv.insight.node_level(node);
        let cost = node.next_cost(current_level);
        if inv.insight.balance < cost {
            break;
        }
        inv.insight.balance -= cost;
        let entry = inv.insight.nodes.entry(node.id()).or_insert(0);
        *entry = entry.saturating_add(1);
        bought += 1;
    }
    if bought == 0 {
        return Err("not enough insight for even one level".into());
    }
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
