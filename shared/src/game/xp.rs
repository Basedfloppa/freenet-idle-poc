//! XP / level curve — soft-knee exponential up to level 100.
//!
//! 1.5× growth keeps the early ramp pacy (L1→L20 in ~30 min of
//! active play). After level 20 the factor drops to 1.3× so the
//! L20→L30 stretch — which used to need 5-10 wall-clock hours of
//! grinding under the flat 1.5× curve — is reachable in a single
//! active session (~1-2 hours) without flattening the mountain
//! to triviality. See UX analysis §«Балансные проблемы» for the
//! before/after numbers.

use super::Inventory;

/// Level at which the curve transitions from steep (1.5×) to
/// gentler (1.3×). Don't change without auditing balance —
/// `idle-poc` ships content gated by levels both below and above
/// this knee.
pub const XP_KNEE_LEVEL: u64 = 20;

pub fn level_of(inv: &Inventory) -> u64 {
    level_from_xp(inv.experience)
}

pub fn xp_for_level(level: u64) -> u64 {
    if level == 0 {
        return 0;
    }
    let mut req: u128 = 100;
    let mut i = 1;
    while i < level {
        // Multiply by 3/2 below the knee (matches the legacy
        // curve), by 13/10 above. Integer ratios keep the
        // computation deterministic across platforms.
        let (num, den) = if i < XP_KNEE_LEVEL { (3, 2) } else { (13, 10) };
        req = req.saturating_mul(num) / den;
        if req > u64::MAX as u128 {
            return u64::MAX;
        }
        i += 1;
    }
    req as u64
}

pub fn xp_total_for_level(level: u64) -> u64 {
    let mut total: u128 = 0;
    for l in 1..level {
        total = total.saturating_add(xp_for_level(l) as u128);
    }
    total.min(u64::MAX as u128) as u64
}

pub fn level_from_xp(xp: u64) -> u64 {
    let mut lvl = 1u64;
    let mut consumed: u64 = 0;
    while lvl < 100 {
        let req = xp_for_level(lvl);
        if consumed.saturating_add(req) > xp {
            return lvl;
        }
        consumed = consumed.saturating_add(req);
        lvl += 1;
    }
    100
}
