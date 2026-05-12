//! XP / level curve — exponential up to level 100.

use super::Inventory;

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
        req = req.saturating_mul(3) / 2;
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
