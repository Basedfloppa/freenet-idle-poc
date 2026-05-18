//! Legacy / Ascend RPCs (backlog C1).

use shared::DelegateRequest as AppRequest;
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

use super::dispatch::delegate_op_once;

pub fn buy_legacy_node_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    node_id: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::BuyLegacyNode { node_id, now_ms },
        "buy legacy node",
    );
}

pub fn buy_legacy_node_bulk_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    node_id: u8,
    count: u32,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::BuyLegacyNodeBulk { node_id, count, now_ms },
        "buy legacy node bulk",
    );
}

pub fn ascend_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::Ascend { now_ms },
        "ascend",
    );
}
