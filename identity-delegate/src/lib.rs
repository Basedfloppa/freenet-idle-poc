//! Identity delegate: the freenet-side authority for every piece
//! of game state.
//!
//! Module layout:
//!
//!   * [`state`] — secret-store I/O (load/save seed + Inventory),
//!     monotonic-clock guard, HP regen catch-up.
//!   * [`derived`] — stat formulae (max HP / attack / defence /
//!     speed / evasion). Pure functions of `Inventory` — no I/O.
//!   * [`progression`] — idempotent unlock checks for
//!     achievements, skills, endings.
//!   * [`combat`] — turn-by-turn encounter resolver and the
//!     multi-encounter mission chain.
//!   * [`actions`] — every other RPC handler (set area, equip,
//!     buy, sell, forge, farm, etc.).
//!
//! This file is the dispatcher only — it maps `DelegateRequest`
//! variants to handler functions in the modules above.

mod actions;
mod combat;
mod derived;
mod progression;
mod state;

use freenet_stdlib::prelude::*;
use shared::{
    DelegateEnvelopeIn, DelegateEnvelopeOut, DelegateRequest as AppRequest,
    DelegateResponse as AppResponse,
};

struct IdentityDelegate;

#[delegate]
impl DelegateInterface for IdentityDelegate {
    fn process(
        ctx: &mut DelegateCtx,
        _params: Parameters<'static>,
        _origin: Option<MessageOrigin>,
        message: InboundDelegateMsg,
    ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
        let app_msg = match message {
            InboundDelegateMsg::ApplicationMessage(m) => m,
            _ => return Err(DelegateError::Other("unsupported message type".into())),
        };

        let envelope: DelegateEnvelopeIn = bincode::deserialize(app_msg.payload.as_slice())
            .map_err(|e| DelegateError::Other(format!("deser envelope: {e}")))?;

        let response = match envelope.request {
            AppRequest::GetPubkey { seed_if_missing } => {
                match state::load_or_install_seed(ctx, &seed_if_missing) {
                    Ok(sk) => AppResponse::Pubkey {
                        pubkey: sk.verifying_key().to_bytes(),
                    },
                    Err(e) => AppResponse::Error(e),
                }
            }
            AppRequest::PublishPresence { name, area, now_ms } => {
                match actions::publish_presence(ctx, name, area, now_ms) {
                    Ok((payload, signature)) => {
                        AppResponse::SignedPresence { payload, signature }
                    }
                    Err(e) => AppResponse::Error(e),
                }
            }
            AppRequest::LoadInventory { now_ms } => map_inv(actions::touch_inventory(ctx, now_ms)),
            AppRequest::RunMission { now_ms } => map_inv(combat::run_mission(ctx, now_ms)),
            AppRequest::SetArea { area_id, now_ms } => {
                map_inv(actions::set_area(ctx, area_id, now_ms))
            }
            AppRequest::EquipGear { catalog_id, now_ms } => {
                map_inv(actions::equip_gear(ctx, catalog_id, now_ms))
            }
            AppRequest::UnequipSlot { slot, now_ms } => {
                map_inv(actions::unequip_slot(ctx, slot, now_ms))
            }
            AppRequest::UseConsumable { kind, now_ms } => {
                map_inv(actions::use_consumable(ctx, kind, now_ms))
            }
            AppRequest::BuyItem { kind, now_ms } => map_inv(actions::buy_item(ctx, kind, now_ms)),
            AppRequest::SellGear { catalog_id, now_ms } => {
                map_inv(actions::sell_gear(ctx, catalog_id, now_ms))
            }
            AppRequest::ForgeUpgrade { catalog_id, now_ms } => {
                map_inv(actions::forge_upgrade(ctx, catalog_id, now_ms))
            }
            AppRequest::WorkFarm { now_ms } => map_inv(actions::work_farm(ctx, now_ms)),
            AppRequest::SellWheat { amount, now_ms } => {
                map_inv(actions::sell_wheat(ctx, amount, now_ms))
            }
            AppRequest::BuyGearRoll { slot, tier, now_ms } => {
                map_inv(actions::buy_gear_roll(ctx, slot, tier, now_ms))
            }
            AppRequest::AutoEquipBest { now_ms } => {
                map_inv(actions::auto_equip_best(ctx, now_ms))
            }
            AppRequest::BuySkill { skill_id, now_ms } => {
                map_inv(actions::buy_skill(ctx, skill_id, now_ms))
            }
            AppRequest::SetAutoRun { enabled, now_ms } => {
                map_inv(actions::set_auto_run(ctx, enabled, now_ms))
            }
            AppRequest::ExportSeed => match state::load_seed(ctx) {
                Ok(Some(sk)) => AppResponse::Seed { seed: sk.to_bytes() },
                Ok(None) => AppResponse::Error("no seed installed yet".into()),
                Err(e) => AppResponse::Error(e),
            },
            AppRequest::ResetInventory { now_ms } => {
                map_inv(actions::reset_inventory(ctx, now_ms))
            }
            AppRequest::SendMessage { to, kind, body, now_ms } => {
                match actions::send_message(ctx, to, kind, body, now_ms) {
                    Ok((payload, signature)) => {
                        AppResponse::SignedMessage { payload, signature }
                    }
                    Err(e) => AppResponse::Error(e),
                }
            }
            AppRequest::QueueBattleAction { action, now_ms } => {
                map_inv(actions::queue_battle_action(ctx, action, now_ms))
            }
            AppRequest::TickBattle { now_ms } => map_inv(actions::tick_only(ctx, now_ms)),
            AppRequest::SignGuildOp { op_kind, name_or_id, now_ms } => {
                match actions::sign_guild_op(ctx, op_kind, name_or_id, now_ms) {
                    Ok((payload, signature)) => {
                        AppResponse::SignedGuildOp { payload, signature }
                    }
                    Err(e) => AppResponse::Error(e),
                }
            }
        };

        let out_envelope = DelegateEnvelopeOut {
            request_id: envelope.request_id,
            response,
        };
        let resp_bytes = bincode::serialize(&out_envelope)
            .map_err(|e| DelegateError::Other(format!("ser response: {e}")))?;
        let out = ApplicationMessage::new(resp_bytes).processed(true);
        Ok(vec![OutboundDelegateMsg::ApplicationMessage(out)])
    }
}

fn map_inv(r: Result<shared::Inventory, String>) -> AppResponse {
    match r {
        Ok(inv) => AppResponse::Inventory(inv),
        Err(e) => AppResponse::Error(e),
    }
}
