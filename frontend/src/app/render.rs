//! Top-level renderer. `render_core` is the giant view builder
//! that produces the entire page DOM for one frame: it bakes per-
//! callback closures (Yew can't take params directly), reads
//! authoritative state from `Core`, and dispatches to per-tab
//! sub-views composed from `widgets`.

use shared::{
    area_of, form_slot_mask, form_sprite, format_si, level_of,
    shop_buy_price, skill_buy_price, PresencePayload, PubKey,
    AREAS, CONSUMABLE_FIREBALL, CONSUMABLE_POTION, ENCOUNTERS_PER_MISSION,
    FIREBALL_BOSS_DAMAGE, FIREBALL_PRICE, MISSION_DAMAGE, MISSION_ESSENCE, MISSION_GOLD,
    POTION_PRICE, SKILL_DRAGON_SCALES, SKILL_FELINE_GRACE, SKILL_SLIME_BODY,
    SKILL_STEED_HEART, SLOT_COUNT, STATUS_ADVENTURING, STATUS_DEFEATED,
    STATUS_ESTATE, STATUS_FOCUSING, STATUS_RECOVERING, WHEAT_PER_GOLD,
};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use super::i18n_shared;
use crate::freenet::actions::{
    auto_equip_once, buy_estate_worker_once, buy_gear_once, buy_item_once, buy_skill_once,
    equip_gear_once, export_seed_once, forge_upgrade_once, guild_op_once,
    queue_battle_action_once, reset_inventory_once, run_mission_once, sell_gear_once,
    sell_wheat_once, send_message_once, set_area_once, set_auto_run_once,
    set_idle_action_once, unequip_slot_once, use_consumable_once, work_farm_once,
};
use crate::game::derived::{
    area_of_name, attack_from, defence_from, equipped_bonuses, max_hp_from,
    player_speed_evasion, status_code, status_text, world_boss_state, xp_in_level,
};

use super::core::{ingest_inventory, Core, ONBOARDING_STEPS};
use super::i18n::{locale_code, locale_from_code, Locale, MessageId};

/// Read the live `Locale` from a borrowed `CoreCell`. Used inside
/// closures (confirm dialogs, callbacks) that fire *after* render
/// returns — we can't capture the locale value because it might
/// change mid-session, and we can't reach into the rendered DOM
/// for it either. `Locale::default()` is the fallback if the core
/// isn't initialised yet (in practice the closures only fire after
/// the core is alive, so the fallback is just a type-system stub).
fn locale_for_confirm(core: &CoreCell) -> Locale {
    core.borrow()
        .as_ref()
        .map(|c| c.prefs.locale)
        .unwrap_or_default()
}
use super::util::DEFAULT_WS;
use super::prefs::{apply_theme, clear_all_prefs, save_prefs, SyncCadence, THEMES};
use super::types::{Tab, ToggleField};
use super::util::{now_ms, truncate, webapp_contract_id};
use super::widgets::{
    render_area_card, render_battle_queue, render_battle_stage,
    render_catchup_modal, render_combat_history, render_debug_overlay, render_equipped_slot,
    render_mailbox_panel, render_onboarding, render_stash_grouped, render_toasts, top_actions,
};
use super::core::{CoreCell, PendingCell};

pub fn render_core(
    c: &Core,
    core_cell: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
) -> Html {
    let on_name = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        // `oninput` (fires every keystroke) — not `onchange` (fires
        // only on blur). With auto-mission running, periodic
        // re-renders kept yanking the input's value back to
        // whatever `c.name` held last commit, so half-typed names
        // were silently overwritten by Yew's controlled input
        // reconciliation. Updating state on every keystroke makes
        // the input the source of truth in real time.
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let new_name = truncate(&input.value(), shared::MAX_NAME_BYTES);
            if let Some(c) = core.borrow_mut().as_mut() {
                c.name = new_name.clone();
            }
            // Persist via the delegate (one RPC per keystroke — cheap
            // on a local node, and the WS pipeline coalesces inflight
            // calls). localStorage isn't reliable here because the
            // webapp iframe is null-origin in the default Freenet
            // sandbox, so the delegate is the only place this value
            // survives a reload.
            crate::freenet::actions::settings::save_settings_once(
                core.clone(),
                pending.clone(),
                bump.clone(),
                Some(new_name),
                None,
                None,
                None,
                None,
                None,
            );
            bump.set(now_ms());
        })
    };

    let on_run_mission = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| run_mission_once(core.clone(), pending.clone(), bump.clone()))
    };

    let on_guild_name_input = {
        let core = core_cell.clone();
        let bump = bump.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(c) = core.borrow_mut().as_mut() {
                c.new_guild_name_input = input.value();
            }
            bump.set(now_ms());
        })
    };

    let on_create_guild = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            let name = {
                let g = core.borrow();
                let Some(c) = g.as_ref() else { return };
                c.new_guild_name_input.clone()
            };
            if name.trim().is_empty() {
                return;
            }
            guild_op_once(
                core.clone(),
                pending.clone(),
                bump.clone(),
                shared::GUILD_OP_CREATE,
                name,
            );
            if let Some(c) = core.borrow_mut().as_mut() {
                c.new_guild_name_input.clear();
            }
            bump.set(now_ms());
        })
    };

    let mk_guild_join_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |guild_id_hex: String| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_: MouseEvent| {
                guild_op_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    shared::GUILD_OP_JOIN,
                    guild_id_hex.clone(),
                );
            })
        }
    };
    let mk_guild_leave_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |guild_id_hex: String| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_: MouseEvent| {
                guild_op_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    shared::GUILD_OP_LEAVE,
                    guild_id_hex.clone(),
                );
            })
        }
    };

    // Leader-only disband. Gated behind a `window.confirm()` since
    // it deletes the guild for every member at once (not just self).
    let mk_guild_disband_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |guild_id_hex: String, guild_name: String| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_: MouseEvent| {
                let win = match web_sys::window() { Some(w) => w, None => return };
                let confirmed = win
                    .confirm_with_message(&locale_for_confirm(&core).confirm_disband_guild(&guild_name))
                    .unwrap_or(false);
                if !confirmed { return; }
                guild_op_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    shared::GUILD_OP_DISBAND,
                    guild_id_hex.clone(),
                );
            })
        }
    };

    // Tab-switch closure factory. UI-only state — flip
    // `c.current_tab` and re-render. No delegate roundtrip.
    let mk_tab_cb = {
        let core = core_cell.clone();
        let bump = bump.clone();
        move |tab: Tab| {
            let core = core.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.current_tab = tab;
                }
                bump.set(now_ms());
            })
        }
    };

    // Closure factory: returns a callback that flips to a specific
    // area when clicked. Yew callbacks can't take parameters directly,
    // so we bake area_id into a fresh callback per area card.
    let mk_set_area_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |area_id: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                set_area_once(core.clone(), pending.clone(), bump.clone(), area_id)
            })
        }
    };

    // Same closure-factory pattern for gear and consumable buttons.
    // Each render produces fresh closures because the inventory may
    // have changed; the wrappers themselves are zero-cost.
    let mk_equip_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |catalog_id: u16| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                equip_gear_once(core.clone(), pending.clone(), bump.clone(), catalog_id)
            })
        }
    };
    let mk_unequip_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |slot: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                unequip_slot_once(core.clone(), pending.clone(), bump.clone(), slot)
            })
        }
    };
    let mk_sell_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |catalog_id: u16| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                sell_gear_once(core.clone(), pending.clone(), bump.clone(), catalog_id)
            })
        }
    };
    let mk_forge_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |catalog_id: u16| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                forge_upgrade_once(core.clone(), pending.clone(), bump.clone(), catalog_id)
            })
        }
    };
    // Equipment-panel "Use" callback. Smart-routed by battle state:
    //   * no active battle → `UseConsumable` (immediate heal / boss
    //     damage), like before;
    //   * mid-battle → `QueueBattleAction` (queue for next turn).
    //
    // Single canonical position for consumables = the equipment
    // panel. The scene panel during a fight shows recent turns and
    // the queued-action notice, but no longer duplicates the buttons.
    let mk_use_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |kind: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                let in_battle = {
                    let g = core.borrow();
                    g.as_ref()
                        .map(|c| c.inventory.current_battle.is_some())
                        .unwrap_or(false)
                };
                if in_battle {
                    let action = match kind {
                        CONSUMABLE_POTION => shared::BATTLE_ACTION_POTION,
                        CONSUMABLE_FIREBALL => shared::BATTLE_ACTION_FIREBALL,
                        _ => return,
                    };
                    queue_battle_action_once(
                        core.clone(),
                        pending.clone(),
                        bump.clone(),
                        action,
                    );
                } else {
                    use_consumable_once(core.clone(), pending.clone(), bump.clone(), kind);
                }
            })
        }
    };
    let mk_buy_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |kind: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                buy_item_once(core.clone(), pending.clone(), bump.clone(), kind)
            })
        }
    };
    let on_use_potion = mk_use_cb(CONSUMABLE_POTION);
    let on_use_fireball = mk_use_cb(CONSUMABLE_FIREBALL);
    let on_buy_potion = mk_buy_cb(CONSUMABLE_POTION);
    let on_buy_fireball = mk_buy_cb(CONSUMABLE_FIREBALL);

    // Buy-form callback factory (one closure per form id). Used by
    // the shop's Forms panel — cheap Human reset + expensive
    // direct-form purchases.
    let mk_buy_form_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |form: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                crate::freenet::actions::shop::buy_form_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    form,
                )
            })
        }
    };

    let on_auto_equip = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| auto_equip_once(core.clone(), pending.clone(), bump.clone()))
    };
    let on_work_farm = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| work_farm_once(core.clone(), pending.clone(), bump.clone()))
    };
    let on_sell_all_wheat = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| sell_wheat_once(core.clone(), pending.clone(), bump.clone(), 0))
    };
    let mk_buy_gear_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |slot: u8, tier: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                buy_gear_once(core.clone(), pending.clone(), bump.clone(), slot, tier)
            })
        }
    };
    let mk_buy_skill_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |skill_id: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                buy_skill_once(core.clone(), pending.clone(), bump.clone(), skill_id)
            })
        }
    };

    // Factory for the per-tier "Hire" button in the Estate panel.
    // Captures `tier_id` so the inner Callback can fire the right
    // RPC. Same pattern as `mk_buy_skill_cb`.
    let mk_buy_worker_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |tier_id: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                buy_estate_worker_once(core.clone(), pending.clone(), bump.clone(), tier_id)
            })
        }
    };

    // Legacy node buy factory (C1). One callback per node id so
    // the spend buttons in the Legacy panel can each dispatch
    // their own BuyLegacyNode RPC.
    let mk_buy_legacy_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |node_id: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                crate::freenet::actions::legacy::buy_legacy_node_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    node_id,
                )
            })
        }
    };

    // Ascend handler — soft-resets the run. Confirms first to
    // avoid an accidental click wiping a session's worth of work.
    let on_ascend = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            // Browser-native confirm — same chrome ResetInventory
            // uses. The message is localised from the live `Core`
            // so the player sees it in their picked language.
            let confirm_msg = locale_for_confirm(&core)
                .tr(MessageId::LegacyAscendConfirm)
                .to_string();
            let ok = web_sys::window()
                .and_then(|w| w.confirm_with_message(&confirm_msg).ok())
                .unwrap_or(false);
            if !ok {
                return;
            }
            crate::freenet::actions::legacy::ascend_once(
                core.clone(),
                pending.clone(),
                bump.clone(),
            );
        })
    };

    // Estate idle-action toggle — flips between ESTATE and NONE
    // (single-active rule from §5.6, auto-mission button has its
    // own callback). The delegate's `SetIdleAction` keeps
    // `auto_run_enabled` in sync.
    let on_toggle_estate = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| {
            let next = {
                let g = core.borrow();
                let Some(c) = g.as_ref() else { return };
                if c.inventory.idle_action == shared::IDLE_ACTION_ESTATE {
                    shared::IDLE_ACTION_NONE
                } else {
                    shared::IDLE_ACTION_ESTATE
                }
            };
            let now = now_ms();
            if let Some(c) = core.borrow_mut().as_mut() {
                c.inventory.idle_action = next;
                if next == shared::IDLE_ACTION_ESTATE {
                    c.inventory.estate.last_tick_ms = now;
                    c.inventory.auto_run_enabled = false;
                    c.inventory.auto_last_tick_ms = 0;
                } else {
                    c.inventory.estate.last_tick_ms = 0;
                }
            }
            bump.set(now);
            set_idle_action_once(core.clone(), pending.clone(), bump.clone(), next);
        })
    };

    let on_toggle_auto = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| {
            // Optimistic flip + send. The delegate is authoritative
            // (offline catch-up uses `auto_last_tick_ms`), but
            // waiting for the round-trip before changing the button
            // label feels mushy. The response message overwrites
            // inventory with delegate-side ground truth.
            let next = {
                let g = core.borrow();
                let Some(c) = g.as_ref() else { return };
                !c.inventory.auto_run_enabled
            };
            let now = now_ms();
            if let Some(c) = core.borrow_mut().as_mut() {
                c.inventory.auto_run_enabled = next;
                c.inventory.auto_last_tick_ms = if next { now } else { 0 };
            }
            bump.set(now);
            set_auto_run_once(core.clone(), pending.clone(), bump.clone(), next);
        })
    };

    // Theme picker factory — clicking a theme button writes the id
    // to `<html data-theme="…">`, persists it via the delegate, and
    // mirrors it on `Core.current_theme` so the picker buttons
    // reflect the active selection without a reload.
    let mk_theme_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |theme_id: &'static str| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                apply_theme(theme_id);
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.current_theme = theme_id.to_string();
                }
                crate::freenet::actions::settings::save_settings_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    None,
                    Some(theme_id.to_string()),
                    None,
                    None,
                    None,
                    None,
                );
                bump.set(now_ms());
            })
        }
    };

    // Locale picker callback factory. Mirrors mk_theme_cb: writes the
    // chosen locale to `UserPrefs.locale` (localStorage, for instant
    // re-render before the network round-trip lands) AND fires off a
    // `save_settings_once` so the delegate stores it next to the
    // theme. The delegate copy is what survives a fresh browser or a
    // cleared cache — localStorage in the sandboxed null-origin
    // iframe doesn't.
    let mk_locale_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |code: &'static str| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_: MouseEvent| {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.prefs.locale = locale_from_code(code);
                    save_prefs(&c.prefs);
                }
                crate::freenet::actions::settings::save_settings_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    None,
                    None,
                    None,
                    Some(code.to_string()),
                    None,
                    None,
                );
                bump.set(now_ms());
            })
        }
    };

    // Per-cadence callback factory. The setter copies the enum
    // variant into prefs and persists. Radio-button UI calls this
    // once per option below.
    let mk_cadence_cb = {
        let core = core_cell.clone();
        let bump = bump.clone();
        move |cadence: SyncCadence| {
            let core = core.clone();
            let bump = bump.clone();
            Callback::from(move |_: MouseEvent| {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.prefs.sync_cadence = cadence;
                    save_prefs(&c.prefs);
                }
                bump.set(now_ms());
            })
        }
    };

    let mk_hp_pause_cb = {
        let core = core_cell.clone();
        let bump = bump.clone();
        move |pct: u8| {
            let core = core.clone();
            let bump = bump.clone();
            Callback::from(move |_: MouseEvent| {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.prefs.auto_pause_hp_pct = pct;
                    save_prefs(&c.prefs);
                }
                bump.set(now_ms());
            })
        }
    };

    let mk_toggle_cb = {
        let core = core_cell.clone();
        let bump = bump.clone();
        move |field: ToggleField| {
            let core = core.clone();
            let bump = bump.clone();
            Callback::from(move |_: MouseEvent| {
                if let Some(c) = core.borrow_mut().as_mut() {
                    match field {
                        ToggleField::ReactivePublish => {
                            c.prefs.reactive_publish = !c.prefs.reactive_publish;
                        }
                        ToggleField::HidePubkey => {
                            c.prefs.hide_pubkey = !c.prefs.hide_pubkey;
                        }
                        ToggleField::HideStale => {
                            c.prefs.hide_stale_players = !c.prefs.hide_stale_players;
                        }
                    }
                    save_prefs(&c.prefs);
                }
                bump.set(now_ms());
            })
        }
    };

    let on_reset_prefs = {
        // Wipes the entire prefs blob (and theme) and reloads.
        Callback::from(move |_: MouseEvent| {
            clear_all_prefs();
        })
    };

    let on_reset_progress = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            // Browser-native confirm prompt — simplest two-button
            // gate for a destructive op without modal infrastructure.
            let win = match web_sys::window() { Some(w) => w, None => return };
            let confirmed = win
                .confirm_with_message(locale_for_confirm(&core).confirm_reset_progress())
                .unwrap_or(false);
            if confirmed {
                reset_inventory_once(core.clone(), pending.clone(), bump.clone());
            }
        })
    };

    let on_export_seed = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            // Two-step reveal — confirm first to discourage muscle
            // memory clicks, then dispatch the RPC.
            let win = match web_sys::window() { Some(w) => w, None => return };
            let confirmed = win
                .confirm_with_message(locale_for_confirm(&core).confirm_reveal_seed())
                .unwrap_or(false);
            if !confirmed { return }
            let core_for_cb = core.clone();
            let bump_for_cb = bump.clone();
            export_seed_once(core.clone(), pending.clone(), move |result| {
                if let Some(c) = core_for_cb.borrow_mut().as_mut() {
                    let loc = c.prefs.locale;
                    match result {
                        Ok(seed) => {
                            c.exported_seed_hex = Some(hex::encode(seed));
                            c.status = loc.status_seed_exported().to_string();
                        }
                        Err(e) => {
                            c.status = loc.fmt_status_seed_export_failed(&e);
                        }
                    }
                }
                bump_for_cb.set(now_ms());
            });
        })
    };

    let on_hide_seed = {
        let core = core_cell.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(c) = core.borrow_mut().as_mut() {
                c.exported_seed_hex = None;
            }
            bump.set(now_ms());
        })
    };

    let on_mailbox_self_test = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            // Round-trip test: send a chat message addressed to our
            // own pubkey. The mailbox contract accepts it, we
            // subscribe to ourselves, and the Inbox renders it
            // moments later. Validates the entire signing + publish +
            // subscribe + verify chain.
            let to = {
                let g = core.borrow();
                let Some(c) = g.as_ref() else { return };
                match c.pubkey {
                    Some(pk) => pk,
                    None => return,
                }
            };
            let body = format!("self-test @ {}ms", now_ms()).into_bytes();
            send_message_once(
                core.clone(),
                pending.clone(),
                bump.clone(),
                to,
                shared::MSG_KIND_CHAT,
                body,
            );
        })
    };

    // Onboarding wizard: advance step / dismiss. Each closure
    // mutates `c.onboarding_step` and re-bumps the view. The
    // "dismissed" flag is persisted via the delegate
    // (`save_settings_once`) so it survives reload — localStorage
    // can't be relied on inside the sandboxed iframe.
    let on_onboarding_next = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            let mut just_finished = false;
            if let Some(c) = core.borrow_mut().as_mut() {
                let next = c.onboarding_step.map(|s| s + 1).unwrap_or(0);
                if next >= ONBOARDING_STEPS {
                    c.onboarding_step = None;
                    just_finished = true;
                } else {
                    c.onboarding_step = Some(next);
                }
            }
            if just_finished {
                crate::freenet::actions::settings::save_settings_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    None,
                    None,
                    Some(true),
                    None,
                    None,
                    None,
                );
            }
            bump.set(now_ms());
        })
    };
    let on_onboarding_skip = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(c) = core.borrow_mut().as_mut() {
                c.onboarding_step = None;
            }
            crate::freenet::actions::settings::save_settings_once(
                core.clone(),
                pending.clone(),
                bump.clone(),
                None,
                None,
                Some(true),
                None,
                None,
                None,
            );
            bump.set(now_ms());
        })
    };

    let on_catchup_dismiss = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            let version = env!("BUILD_VERSION").to_string();
            // Capture the catchup's started_ms before we wipe it
            // locally — this is the watermark we persist so the
            // same window doesn't re-pop on next reload.
            let acked_ts = {
                let g = core.borrow();
                let Some(c) = g.as_ref() else { return };
                c.inventory.last_catchup.as_ref().map(|s| s.started_ms).unwrap_or(0)
            };
            if let Some(c) = core.borrow_mut().as_mut() {
                c.catchup_modal_dismissed = true;
                c.last_seen_version = Some(version.clone());
                c.last_catchup_acked_started_ms =
                    c.last_catchup_acked_started_ms.max(acked_ts);
            }
            crate::freenet::actions::settings::save_settings_once(
                core.clone(),
                pending.clone(),
                bump.clone(),
                None,
                None,
                None,
                None,
                Some(version),
                Some(acked_ts),
            );
            bump.set(now_ms());
        })
    };

    let on_ws_input = {
        let core = core_cell.clone();
        let bump = bump.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(c) = core.borrow_mut().as_mut() {
                c.prefs.ws_url_override = input.value();
                save_prefs(&c.prefs);
            }
            bump.set(now_ms());
        })
    };
    let _ = ingest_inventory;

    let my = c.pubkey;
    let now = now_ms();
    let (boss_era, boss_hp, boss_max_hp, total_dmg) = world_boss_state(c);
    let boss_pct = if boss_max_hp == 0 {
        0
    } else {
        (boss_hp * 100 / boss_max_hp).min(100)
    };

    // Leaderboard rows (you + others). Sort by gold descending,
    // tie-break by name so the order is stable across renders.
    let mut rows: Vec<(PubKey, PresencePayload, u64, bool)> = Vec::new();
    if let Some(my) = my {
        rows.push((
            my,
            PresencePayload::new(
                my,
                c.name.clone(),
                c.inventory.gold,
                c.inventory.boss_damage,
                "lobby".into(),
                c.last_published_ms.unwrap_or(0),
            ),
            now,
            true,
        ));
    }
    for (pk, (p, received_ms)) in &c.others {
        // `hide_stale_players` drops rows we haven't seen a fresh
        // heartbeat from in 30 s. Own-row always stays — it's the
        // player themselves, never "stale" in the UX sense.
        if c.prefs.hide_stale_players && now.saturating_sub(*received_ms) >= 30_000 {
            continue;
        }
        rows.push((*pk, p.clone(), *received_ms, false));
    }
    rows.sort_by(|a, b| b.1.gold.cmp(&a.1.gold).then(a.1.name.cmp(&b.1.name)));

    let locale = c.prefs.locale;
    let publish_age = c
        .last_published_ms
        .map(|ms| locale.fmt_seconds_ago(now.saturating_sub(ms) / 1000))
        .unwrap_or_else(|| locale.term_never().to_string());

    let pubkey_text = my
        .map(|pk| match locale.fmt_locale() {
            Locale::En => format!("pubkey (from delegate): {}", crate::short_id(&pk)),
            Locale::Ru => format!("ключ (от делегата): {}", crate::short_id(&pk)),
            Locale::De => unreachable!("fmt_locale normalises De"),
        })
        .unwrap_or_else(|| locale.tr(MessageId::TermPubkeyPending).to_string());

    let auto_label = if c.inventory.auto_run_enabled {
        locale.tr(MessageId::BtnAutoOn)
    } else {
        locale.tr(MessageId::BtnAutoOff)
    };
    // Estate is mutually exclusive with combat (§5.6). Disable the
    // Run Mission button while Estate is the active idle action —
    // delegate would reject the RPC anyway, but the visual gate is
    // clearer than a flashing error.
    let estate_blocking_combat =
        c.inventory.idle_action == shared::IDLE_ACTION_ESTATE;
    let mission_disabled =
        my.is_none() || c.mission_in_flight || estate_blocking_combat;

    let inv = &c.inventory;
    // Reveal-bit slide-in animation is gated to the keys that just
    // flipped on this delegate tick. `ingest_inventory` clears
    // `animate_reveal` on the next ingest, so a section animates
    // exactly once even if the player switches tabs in and out.
    // Returning players see `animate_reveal == 0` on cold load.
    let anim_cls = |key: shared::RevealKey| -> &'static str {
        if c.animate_reveal & key.bit() != 0 {
            "reveal-anim"
        } else {
            ""
        }
    };
    let lvl = level_of(inv);
    let hp_max = max_hp_from(inv);
    let atk = attack_from(inv);
    let def = defence_from(inv);
    let (chap_no, chap_title, chap_body_map) = i18n_shared::chapter(locale, inv);
    let area = area_of(inv.current_area);
    let _mission_gold = MISSION_GOLD.saturating_mul(area.gold_mult);
    let mission_essence = MISSION_ESSENCE.saturating_mul(area.essence_mult);
    let mission_damage = MISSION_DAMAGE.saturating_mul(area.damage_mult);
    let (eq_atk, eq_def, eq_hp) = equipped_bonuses(inv);
    let stash_count = inv.unequipped.len();
    // Auto-Equip Best pre-flight: button stays greyed unless at
    // least one form-allowed slot has a stash piece that scores
    // higher than the currently-equipped one. Keeps the button's
    // visual state honest — pressing it would have been a no-op
    // otherwise.
    let auto_equip_can_improve = crate::game::derived::auto_equip_would_change(inv);
    let auto_equip_tip: String = if auto_equip_can_improve {
        locale.tr(MessageId::TipAutoEquipBest).to_string()
    } else {
        locale.tr(MessageId::TipAutoEquipNothing).to_string()
    };

    // World-map graph layout (C3a). Compute each area's depth =
    // longest path from a starter (predecessors-empty area). Group
    // by depth → columns; render each column vertically with edge
    // hints between adjacent columns. AREAS is small (6 today)
    // so a fixed-point loop is cheaper than topological-sort
    // ceremony. The grouping is read inside `html!` below.
    let area_depths: std::collections::BTreeMap<u8, u8> = {
        let mut depths: std::collections::BTreeMap<u8, u8> =
            std::collections::BTreeMap::new();
        // Seed starters at depth 0.
        for area in AREAS {
            if area.predecessors.is_empty() {
                depths.insert(area.id, 0);
            }
        }
        // Relax until convergence — small graph, cheap.
        let mut changed = true;
        let mut guard = 0usize;
        while changed && guard < 32 {
            changed = false;
            for area in AREAS {
                if area.predecessors.is_empty() {
                    continue;
                }
                let max_pred = area
                    .predecessors
                    .iter()
                    .filter_map(|p| depths.get(p).copied())
                    .max();
                if let Some(d) = max_pred {
                    let new_d = d + 1;
                    let entry = depths.entry(area.id).or_insert(new_d);
                    if *entry != new_d {
                        *entry = new_d;
                        changed = true;
                    } else if depths.get(&area.id).copied().unwrap_or(0) != new_d {
                        changed = true;
                    }
                }
            }
            guard += 1;
        }
        depths
    };
    let area_columns: std::collections::BTreeMap<u8, Vec<&shared::AreaDef>> = {
        let mut by_depth: std::collections::BTreeMap<u8, Vec<&shared::AreaDef>> =
            std::collections::BTreeMap::new();
        for area in AREAS {
            let d = area_depths.get(&area.id).copied().unwrap_or(0);
            by_depth.entry(d).or_default().push(area);
        }
        by_depth
    };
    let (xp_cur, xp_req) = xp_in_level(inv);
    let xp_pct = if xp_req == 0 { 100 } else { (xp_cur * 100 / xp_req).min(100) };
    let (p_speed, p_evasion) = player_speed_evasion(inv);
    let status_pill_cls = match status_code(c) {
        STATUS_DEFEATED => "pill defeated",
        STATUS_FOCUSING => "pill casting",
        STATUS_ADVENTURING => "pill auto",
        STATUS_RECOVERING => "pill recovering",
        STATUS_ESTATE => "pill estate",
        _ => "pill idle",
    };
    // Localised pill text. status_text returns an English &'static str
    // from the shared crate (used by non-UI consumers too); the i18n
    // layer remaps it via status_code → MessageId in the frontend.
    // (`locale` was bound earlier alongside publish_age/auto_label.)
    let status_pill_text = match status_code(c) {
        STATUS_DEFEATED => locale.tr(MessageId::PillDefeated),
        STATUS_FOCUSING => locale.tr(MessageId::PillFocusing),
        STATUS_ADVENTURING => locale.tr(MessageId::PillAdventuring),
        STATUS_RECOVERING => locale.tr(MessageId::PillRecovering),
        STATUS_ESTATE => locale.tr(MessageId::PillEstate),
        _ => locale.tr(MessageId::PillReady),
    };
    let _ = (area_of_name, status_text);

    html! {
        <main>
            { render_toasts(&c.toasts, now) }
            { render_onboarding(locale, c.onboarding_step, on_onboarding_next, on_onboarding_skip) }
            { render_catchup_modal(c, locale, on_catchup_dismiss) }
            <header class="page-head">
                <div class="title-row">
                    <h1>{ "Freenet Idle PoC" }</h1>
                    // Show the crate semver from Cargo.toml — stable
                    // and human-readable, unlike the previous
                    // contract-id prefix which rotated on every
                    // `fdev website publish` and gave no sense of
                    // release order. The full webapp contract id is
                    // still surfaced via the `title` attribute
                    // (tooltip) for diagnostics: which DHT-resolved
                    // bundle the user is actually running.
                    <span class="webapp-version" title={webapp_contract_id().unwrap_or_default()}>
                        { "v" }{ env!("BUILD_VERSION") }
                    </span>
                    <span class={status_pill_cls}>{ status_pill_text }</span>
                    <a class="repo-link"
                       href="https://github.com/Basedfloppa/freenet-idle-poc"
                       target="_blank"
                       rel="noopener noreferrer">
                        { locale.tr(MessageId::SourceLink) }
                    </a>
                </div>
                <p class="status">{ &c.status }</p>
            </header>

            <nav class="top-actions">
                { for top_actions(locale).iter().filter(|(_, _, tab)| {
                    // Phased reveal (A5): tabs stay hidden until
                    // their reveal-bit latches on. Farm / Settings /
                    // Help are always shown so a fresh player has
                    // somewhere to be.
                    match tab {
                        Tab::Farm | Tab::Settings | Tab::Help => true,
                        Tab::WorldMap => inv.revealed_has(shared::RevealKey::WorldMap),
                        Tab::Shop => inv.revealed_has(shared::RevealKey::Shop),
                        Tab::Guilds => inv.revealed_has(shared::RevealKey::Guilds),
                        Tab::Achievements => inv.revealed_has(shared::RevealKey::Achievements),
                    }
                }).map(|(icon, label, tab)| {
                    let is_active = c.current_tab == *tab;
                    let anim = match tab {
                        Tab::WorldMap => anim_cls(shared::RevealKey::WorldMap),
                        Tab::Shop => anim_cls(shared::RevealKey::Shop),
                        Tab::Guilds => anim_cls(shared::RevealKey::Guilds),
                        Tab::Achievements => anim_cls(shared::RevealKey::Achievements),
                        Tab::Farm | Tab::Settings | Tab::Help => "",
                    };
                    let cls = classes!(
                        "icon-btn",
                        if is_active { "active" } else { "" },
                        anim,
                    );
                    html! {
                        <button
                            class={cls}
                            onclick={mk_tab_cb(*tab)}
                            aria-selected={if is_active { "true" } else { "false" }}
                        >
                            <span class="icon">{ *icon }</span>
                            <span class="icon-label">{ *label }</span>
                        </button>
                    }
                }) }
            </nav>

            // Each tab is its own self-contained view. Switching
            // tabs swaps the entire main content; no scrolling
            // between sections, no surplus context bleeding in.
            { match c.current_tab {
                Tab::Farm => html! {
                    <>
                        {
                            if inv.mission_count == 0 {
                                html! {
                                    <section class="panel tutorial">
                                        <h2>{ locale.tr(MessageId::PanelTutorialWelcome) }</h2>
                                        <p>{ locale.tr(MessageId::TutorialBody1) }</p>
                                        <p class="muted small">{ locale.tr(MessageId::TutorialBody2) }</p>
                                    </section>
                                }
                            } else {
                                html! {}
                            }
                        }
                        <section class="grid-3">
                            <article class="panel stats">
                                <h2>{ locale.tr(MessageId::PanelHero) }</h2>
                                <div class="stat-row">
                                    <label>{ format!("{} ", locale.tr(MessageId::StatName)) }
                                        <input type="text" value={c.name.clone()} oninput={on_name} />
                                    </label>
                                </div>
                                <table class="statgrid">
                                    <tbody>
                                        <tr>
                                            <th>{ locale.tr(MessageId::StatForm) }</th>
                                            <td class="num">
                                                <span class="form-name">
                                                    { format!("{} {}", form_sprite(inv.current_form), i18n_shared::form_name(locale, inv.current_form)) }
                                                </span>
                                            </td>
                                        </tr>
                                        <tr><th>{ locale.tr(MessageId::StatLevel) }</th><td class="num">{ lvl }</td></tr>
                                        <tr>
                                            <th>{ locale.tr(MessageId::StatXp) }</th>
                                            <td class="num">
                                                <div class="hp-mini">
                                                    <span>{ format!("{} / {}", format_si(xp_cur), format_si(xp_req)) }</span>
                                                    <div class="hp-mini-bar">
                                                        <div class="hp-mini-fill xp-fill" style={format!("width: {xp_pct}%")}></div>
                                                    </div>
                                                </div>
                                            </td>
                                        </tr>
                                        <tr>
                                            <th>{ locale.tr(MessageId::StatHp) }</th>
                                            <td class="num">
                                                <div class="hp-mini">
                                                    <span>{ format!("{} / {hp_max}", inv.current_hp) }</span>
                                                    <div class="hp-mini-bar">
                                                        <div class="hp-mini-fill" style={
                                                            format!("width: {}%",
                                                                if hp_max == 0 { 0 } else {
                                                                    (inv.current_hp * 100 / hp_max).min(100)
                                                                })
                                                        }></div>
                                                    </div>
                                                </div>
                                            </td>
                                        </tr>
                                        <tr><th>{ locale.tr(MessageId::StatAttack) }</th><td class="num">{ atk }</td></tr>
                                        <tr><th>{ locale.tr(MessageId::StatDefence) }</th><td class="num">{ def }</td></tr>
                                        <tr><th>{ locale.tr(MessageId::StatSpeed) }</th><td class="num">{ p_speed }</td></tr>
                                        <tr><th>{ locale.tr(MessageId::StatEvasion) }</th><td class="num">{ format!("{p_evasion}%") }</td></tr>
                                    </tbody>
                                </table>
                                { if c.prefs.hide_pubkey { html! {} } else {
                                    html! { <p class="muted small">{ &pubkey_text }</p> }
                                } }
                            </article>

                            <article class="panel scene">
                                <h2>{ format!("{chap_title}") }</h2>
                                {
                                    // Sprite stage / HP bars — battle view replaces
                                    // the static emojis only for the actual visual.
                                    // Action row (Run Mission + auto) stays put.
                                    if let Some(battle) = inv.current_battle.as_ref() {
                                        render_battle_stage(locale, battle, inv, hp_max)
                                    } else {
                                        html! {
                                            <div class="stage">
                                                <div class="hero-sprite">{ form_sprite(inv.current_form) }</div>
                                                <div class="vs">{ "⚔" }</div>
                                                <div class="enemy-sprite">
                                                    { shared::area_default_enemy_sprite(inv.current_area) }
                                                </div>
                                            </div>
                                        }
                                    }
                                }
                                <div class="action-row">
                                    <button class="primary"
                                            onclick={on_run_mission}
                                            disabled={mission_disabled || inv.current_battle.is_some()}
                                            title={
                                                if inv.current_battle.is_some() {
                                                    locale.tr(MessageId::TipFightInProgress)
                                                } else if estate_blocking_combat {
                                                    locale.tr(MessageId::TipEstateBlocksCombat)
                                                } else { "" }
                                            }>
                                        { locale.tr(MessageId::BtnRunMission) }
                                    </button>
                                    {
                                        // Phased reveal (A5): the
                                        // Auto-Mission toggle is hidden
                                        // until the player has run
                                        // 25 missions manually — first
                                        // they should learn the loop,
                                        // then they can automate it.
                                        if inv.revealed_has(shared::RevealKey::AutoMission) {
                                            // Auto-mission and Estate are the same kind of
                                            // idle commitment — only one can hold the
                                            // single-active-action slot (§5.6). Grey the
                                            // toggle while Estate is running so the player
                                            // sees the conflict at a glance instead of
                                            // toggling auto-mission and silently flipping
                                            // their accrual mode.
                                            let auto_disabled = my.is_none() || estate_blocking_combat;
                                            let auto_tip = if estate_blocking_combat {
                                                locale.tr(MessageId::TipEstateBlocksCombat)
                                            } else if inv.current_battle.is_some() {
                                                locale.tr(MessageId::TipAutoToggleMidFight)
                                            } else { "" };
                                            html! {
                                                <button class={classes!(anim_cls(shared::RevealKey::AutoMission))}
                                                        onclick={on_toggle_auto}
                                                        disabled={auto_disabled}
                                                        title={auto_tip}>
                                                    { auto_label }
                                                </button>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </div>
                                {
                                    // Mid-fight queue + recent-turns ticker live
                                    // here, below the action row, so the player
                                    // can react without losing the auto / Run
                                    // controls.
                                    if let Some(battle) = inv.current_battle.as_ref() {
                                        render_battle_queue(locale, battle, inv)
                                    } else {
                                        html! {
                                            <p class="tooltip muted">
                                                {
                                                    locale.fmt_mission_summary(
                                                        i18n_shared::area_name(locale, area),
                                                        ENCOUNTERS_PER_MISSION,
                                                        mission_essence,
                                                        mission_damage,
                                                    )
                                                }
                                            </p>
                                        }
                                    }
                                }
                                <p class="muted small">
                                    { locale.fmt_last_publish(
                                        &publish_age,
                                        &format_si(c.last_published.gold),
                                        &format_si(c.last_published.boss_damage),
                                    ) }
                                </p>
                                { render_combat_history(locale, &inv.combat_history) }
                            </article>

                            // Phased reveal (A5): the Equipment panel
                            // stays hidden until the player has either
                            // a piece equipped or one in the stash.
                            // Consumables sub-panel is nested inside
                            // and is independently gated below.
                            {
                                if inv.revealed_has(shared::RevealKey::Equipment) {
                                    html! {
                                        <article class={classes!("panel", "equipment", anim_cls(shared::RevealKey::Equipment))}>
                                            <h2>{ locale.tr(MessageId::PanelEquipment) }</h2>
                                            <p class="muted small">{ locale.fmt_equipped_bonus(eq_atk, eq_def, eq_hp) }</p>
                                            <div class="action-row">
                                                <button
                                                    onclick={on_auto_equip}
                                                    disabled={!auto_equip_can_improve}
                                                    title={auto_equip_tip.clone()}
                                                >
                                                    { locale.tr(MessageId::BtnAutoEquipBest) }
                                                </button>
                                            </div>
                                            <div class="slot-grid">
                                                { for (0..SLOT_COUNT).map(|i| render_equipped_slot(locale, i, inv, &mk_unequip_cb)) }
                                            </div>
                                            {
                                                if stash_count == 0 {
                                                    html! {
                                                        <p class="muted small">
                                                            { locale.fmt_no_spare_loot(shared::GEAR_DROP_EVERY as u32) }
                                                        </p>
                                                    }
                                                } else {
                                                    html! {
                                                        <p class="muted small">
                                                            { locale.fmt_stash_count(stash_count) }
                                                        </p>
                                                    }
                                                }
                                            }
                                            {
                                                // Consumables: only render rows the player actually
                                                // owns. Reveal-bit latches the section on first
                                                // pickup; per-item gates hide stale lines after a
                                                // consume so "0 fireballs" doesn't squat in the
                                                // panel.
                                                if inv.revealed_has(shared::RevealKey::Consumables)
                                                    && (inv.potions > 0 || inv.fireballs > 0)
                                                {
                                                    html! {
                                                        <>
                                                            <h3>{ locale.tr(MessageId::PanelConsumables) }</h3>
                                                            <div class={classes!("consumable-row", anim_cls(shared::RevealKey::Consumables))}>
                                                                {
                                                                    if inv.potions > 0 {
                                                                        html! {
                                                                            <span class="consumable">
                                                                                <span class="name">{ locale.tr(MessageId::ItemPotion) }</span>
                                                                                <span class="qty">{ inv.potions }</span>
                                                                                <button
                                                                                    onclick={on_use_potion}
                                                                                    title={
                                                                                        if inv.current_battle.is_some() {
                                                                                            locale.tr(MessageId::TipPotionQueue)
                                                                                        } else {
                                                                                            locale.tr(MessageId::TipPotionIdle)
                                                                                        }
                                                                                    }
                                                                                >
                                                                                    { locale.tr(MessageId::BtnUse) }
                                                                                </button>
                                                                            </span>
                                                                        }
                                                                    } else { html! {} }
                                                                }
                                                                {
                                                                    if inv.fireballs > 0 {
                                                                        html! {
                                                                            <span class="consumable">
                                                                                <span class="name">{ locale.tr(MessageId::ItemFireball) }</span>
                                                                                <span class="qty">{ inv.fireballs }</span>
                                                                                <button
                                                                                    onclick={on_use_fireball}
                                                                                    title={
                                                                                        if inv.current_battle.is_some() {
                                                                                            locale.tr(MessageId::TipFireballQueue).to_string()
                                                                                        } else {
                                                                                            locale.fmt_fireball_idle(FIREBALL_BOSS_DAMAGE)
                                                                                        }
                                                                                    }
                                                                                >
                                                                                    { locale.tr(MessageId::BtnUse) }
                                                                                </button>
                                                                            </span>
                                                                        }
                                                                    } else { html! {} }
                                                                }
                                                            </div>
                                                        </>
                                                    }
                                                } else {
                                                    html! {}
                                                }
                                            }
                                        </article>
                                    }
                                } else {
                                    html! {}
                                }
                            }
                        </section>

                        // Phased reveal (A5): World Boss panel
                        // appears at mission_count ≥ 10.
                        {
                            if inv.revealed_has(shared::RevealKey::WorldBoss) {
                                html! {
                                    <section class={classes!("panel", "boss", anim_cls(shared::RevealKey::WorldBoss))}>
                                        <h2>{ locale.tr(MessageId::PanelWorldBoss) }</h2>
                                        <div class="hp-bar">
                                            <div class="hp-fill" style={format!("width: {boss_pct}%")}></div>
                                        </div>
                                        <p class="muted">
                                            { locale.fmt_boss_summary(
                                                boss_era,
                                                &format_si(boss_hp),
                                                &format_si(boss_max_hp),
                                                &format_si(total_dmg),
                                                rows.len(),
                                            ) }
                                        </p>
                                    </section>
                                }
                            } else {
                                html! {}
                            }
                        }

                        // Estate panel (backlog B2). Phased-revealed
                        // once the player has 50 gold so the
                        // first-Farmhand cost is reachable. Workers
                        // accrue resources passively while Estate is
                        // the selected idle action (§5.6).
                        {
                            if inv.revealed_has(shared::RevealKey::Estate) {
                                let estate_active = inv.idle_action == shared::IDLE_ACTION_ESTATE;
                                let form_name_str = i18n_shared::form_name(locale, inv.current_form);
                                let toggle_label = if estate_active {
                                    locale.tr(MessageId::EstateBtnPause)
                                } else {
                                    locale.tr(MessageId::EstateBtnRun)
                                };
                                html! {
                                    <section class={classes!("panel", "estate", anim_cls(shared::RevealKey::Estate))}>
                                        <h2>{ locale.tr(MessageId::PanelEstate) }</h2>
                                        <p class="muted small">
                                            { locale.fmt_estate_hint(form_name_str) }
                                        </p>
                                        <div class="action-row">
                                            <button onclick={on_toggle_estate.clone()}>
                                                { toggle_label }
                                            </button>
                                        </div>
                                        <table class="estate-grid">
                                            <thead>
                                                <tr>
                                                    <th>{ locale.tr(MessageId::EstateColTier) }</th>
                                                    <th class="num">{ locale.tr(MessageId::EstateColOwned) }</th>
                                                    <th class="num">{ locale.tr(MessageId::EstateColYield) }</th>
                                                    <th class="num">{ locale.tr(MessageId::EstateColNextPrice) }</th>
                                                    <th>{ "" }</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                { for shared::ESTATE_TIERS.iter().map(|tier| {
                                                    let owned = inv.estate.workers_of(tier.id);
                                                    let next_price = shared::estate_next_price(tier, owned);
                                                    let aff_bp = shared::form_affinity_bp(inv.current_form, tier.id);
                                                    let res_label = match tier.produces {
                                                        shared::EstateResource::Wheat => locale.tr(MessageId::EstateResWheat),
                                                        shared::EstateResource::Gold => locale.tr(MessageId::EstateResGold),
                                                        shared::EstateResource::Essence => locale.tr(MessageId::EstateResEssence),
                                                    };
                                                    let effective_yield = tier
                                                        .yield_per_sec
                                                        .saturating_mul(owned)
                                                        .saturating_mul(aff_bp)
                                                        / 10_000;
                                                    let aff_cls = if aff_bp > 10_000 { "aff-buff" }
                                                        else if aff_bp < 10_000 { "aff-pen" }
                                                        else { "aff-neutral" };
                                                    let aff_pct = (aff_bp as i64 - 10_000) / 100;
                                                    let aff_str = if aff_bp == 10_000 {
                                                        String::from("")
                                                    } else if aff_pct >= 0 {
                                                        format!(" (+{}%)", aff_pct)
                                                    } else {
                                                        format!(" ({}%)", aff_pct)
                                                    };
                                                    let buy_disabled = inv.gold < next_price;
                                                    let onbuy = mk_buy_worker_cb(tier.id);
                                                    html! {
                                                        <tr>
                                                            <td>{ tier.name }</td>
                                                            <td class="num">{ owned }</td>
                                                            <td class="num">
                                                                { format!("{} {}", effective_yield, res_label) }
                                                                <span class={aff_cls}>{ aff_str }</span>
                                                            </td>
                                                            <td class="num">{ format_si(next_price) }{ " g" }</td>
                                                            <td>
                                                                <button onclick={onbuy} disabled={buy_disabled}>
                                                                    { locale.tr(MessageId::BtnHire) }
                                                                </button>
                                                            </td>
                                                        </tr>
                                                    }
                                                }) }
                                            </tbody>
                                        </table>
                                    </section>
                                }
                            } else {
                                html! {}
                            }
                        }

                        <section class="panel resources">
                            <h2>{ locale.tr(MessageId::PanelResources) }</h2>
                            <table class="inventory">
                                <tbody>
                                    <tr><th>{ locale.tr(MessageId::ResGold) }</th><td class="num">{ format_si(inv.gold) }</td></tr>
                                    <tr><th>{ locale.tr(MessageId::ResEssence) }</th><td class="num">{ format_si(inv.essence) }</td></tr>
                                    <tr><th>{ locale.tr(MessageId::ResMissions) }</th><td class="num">{ format_si(inv.mission_count) }</td></tr>
                                    <tr><th>{ locale.tr(MessageId::ResBossDamage) }</th><td class="num">{ format_si(inv.boss_damage) }</td></tr>
                                </tbody>
                            </table>
                        </section>
                    </>
                },
                Tab::WorldMap => html! {
                    <>
                        <section class="panel world-map">
                            <h2>{ locale.tr(MessageId::PanelWorldMap) }</h2>
                            <p class="muted small">
                                { locale.fmt_currently_farming(i18n_shared::area_name(locale, area), lvl) }
                            </p>
                            // Graph view (C3a): top-to-bottom tree
                            // — one row per depth, cards within a
                            // row laid out horizontally and
                            // centred. Each non-starter card has a
                            // CSS connector line + arrowhead above
                            // it (see `.graph-node.has-parent` in
                            // style.css) plus a localised "↑
                            // Predecessor" label. The flow grows
                            // downward as new branches ship —
                            // adding a row is just `predecessors:
                            // &[parent_id]` on an `AreaDef`.
                            <div class="area-graph">
                                { for area_columns.iter().map(|(depth, row_areas)| html! {
                                    <div class={classes!("graph-row", format!("depth-{}", depth))}>
                                        { for row_areas.iter().map(|a| {
                                            let has_parent = !a.predecessors.is_empty();
                                            let upstream_label = if has_parent {
                                                let names: Vec<String> = a.predecessors
                                                    .iter()
                                                    .filter_map(|pid| shared::AREAS.iter().find(|x| x.id == *pid))
                                                    .map(|p| i18n_shared::area_name(locale, p).to_string())
                                                    .collect();
                                                Some(format!("↑ {}", names.join(" / ")))
                                            } else {
                                                None
                                            };
                                            let node_cls = if has_parent { "graph-node has-parent" } else { "graph-node starter" };
                                            html! {
                                                <div class={node_cls}>
                                                    {
                                                        if let Some(label) = upstream_label.as_ref() {
                                                            html! { <p class="graph-edge-hint">{ label }</p> }
                                                        } else { html! {} }
                                                    }
                                                    { render_area_card(locale, a, inv.current_area, lvl, inv, &mk_set_area_cb) }
                                                </div>
                                            }
                                        }) }
                                    </div>
                                }) }
                            </div>
                        </section>
                        <section class="panel plot">
                            <h2>{ locale.tr(MessageId::PanelPlotSoFar) }</h2>
                            <p class="chapter-no muted">{ locale.fmt_chapter(chap_no as u64) }</p>
                            <p>{ chap_body_map }</p>
                        </section>
                    </>
                },
                Tab::Shop => html! {
                    <>
                        <section class="panel shop">
                            <h2>{ locale.tr(MessageId::PanelShop) }</h2>
                            <p class="muted small">
                                { locale.fmt_shop_balance(
                                    &format_si(inv.gold),
                                    &inv.potions.to_string(),
                                    &inv.fireballs.to_string(),
                                ) }
                            </p>
                            <div class="shop-items">
                                <div class="shop-item">
                                    <span class="name">{ locale.tr(MessageId::ItemPotion) }</span>
                                    <span class="desc muted">
                                        { locale.tr(MessageId::PotionShopDesc) }
                                    </span>
                                    <button
                                        onclick={on_buy_potion}
                                        disabled={inv.gold < POTION_PRICE}
                                    >
                                        { locale.fmt_buy_gold(POTION_PRICE) }
                                    </button>
                                </div>
                                <div class="shop-item">
                                    <span class="name">{ locale.tr(MessageId::ItemFireball) }</span>
                                    <span class="desc muted">
                                        { format!("{FIREBALL_BOSS_DAMAGE} instant boss damage") }
                                    </span>
                                    <button
                                        onclick={on_buy_fireball}
                                        disabled={inv.gold < FIREBALL_PRICE}
                                    >
                                        { locale.fmt_buy_gold(FIREBALL_PRICE) }
                                    </button>
                                </div>
                            </div>
                        </section>

                        // Forms shop: panic-reset to Human + the
                        // four expensive direct-form purchases. Each
                        // row shows the form sprite, name, the stat
                        // bundle as the description so the player
                        // knows what they're buying, and a price
                        // button gated by `form_buy_price` and
                        // current gold.
                        <section class="panel shop forms-shop">
                            <h2>{ locale.tr(MessageId::PanelFormsShop) }</h2>
                            <p class="muted small">
                                { locale.tr(MessageId::FormsShopDesc) }
                            </p>
                            <div class="shop-items">
                                { for [
                                    shared::FORM_HUMAN,
                                    shared::FORM_SLIME,
                                    shared::FORM_CAT,
                                    shared::FORM_HORSE,
                                    shared::FORM_DRAGON,
                                ].iter().filter_map(|form| {
                                    let price = match shared::form_buy_price(*form) {
                                        Some(p) => p,
                                        None => return None,
                                    };
                                    let is_current = inv.current_form == *form;
                                    let (atk, def, hp) = shared::form_base_bonuses(*form);
                                    let (speed, eva) = shared::form_speed_evasion(*form);
                                    let mut parts: Vec<String> = Vec::new();
                                    if atk > 0 { parts.push(format!("+{atk} atk")); }
                                    if def > 0 { parts.push(format!("+{def} def")); }
                                    if hp > 0 { parts.push(format!("+{hp} hp")); }
                                    if speed != 100 { parts.push(format!("speed {speed}")); }
                                    if eva > 0 { parts.push(format!("+{eva}% eva")); }
                                    let stat_desc = if parts.is_empty() {
                                        locale.tr(MessageId::FormsShopBaselineDesc).to_string()
                                    } else {
                                        parts.join(" · ")
                                    };
                                    let cb = mk_buy_form_cb(*form);
                                    let disabled = is_current || inv.gold < price;
                                    Some(html! {
                                        <div class="shop-item">
                                            <span class="name">
                                                { format!("{} {}",
                                                    shared::form_sprite(*form),
                                                    i18n_shared::form_name(locale, *form))
                                                }
                                            </span>
                                            <span class="desc muted">{ stat_desc }</span>
                                            <button onclick={cb} disabled={disabled}
                                                    title={if is_current {
                                                        locale.tr(MessageId::TipFormAlreadyActive)
                                                    } else { "" }}>
                                                { locale.fmt_buy_gold(price) }
                                            </button>
                                        </div>
                                    })
                                }) }
                            </div>
                        </section>

                        <section class="panel stash">
                            <h2>{ locale.fmt_stash_header(inv.unequipped.len()) }</h2>
                            <p class="muted small">
                                { locale.tr(MessageId::ShopStashDesc) }
                            </p>
                            { render_stash_grouped(locale, inv, &mk_equip_cb, &mk_sell_cb, &mk_forge_cb) }
                        </section>

                        <section class="panel buy-gear">
                            <h2>{ locale.tr(MessageId::PanelBuyGear) }</h2>
                            <p class="muted small">
                                { locale.tr(MessageId::ShopBuyGearDesc) }
                            </p>
                            <table class="buy-grid">
                                <thead>
                                    <tr>
                                        <th>{ locale.tr(MessageId::ColSlot) }</th>
                                        <th class="num">{ format!("T1 ({}g)", shop_buy_price(1)) }</th>
                                        <th class="num">{ format!("T2 ({}g)", shop_buy_price(2)) }</th>
                                        <th class="num">{ format!("T3 ({}g)", shop_buy_price(3)) }</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    { for (0..SLOT_COUNT).map(|slot_idx| {
                                        let slot_u8 = slot_idx as u8;
                                        let row_cls = if !form_slot_mask(inv.current_form)[slot_idx] {
                                            "disabled-row"
                                        } else { "" };
                                        html! {
                                            <tr class={row_cls}>
                                                <th>{ i18n_shared::slot_name(locale, slot_idx) }</th>
                                                { for [1u8, 2, 3].iter().map(|t| {
                                                    let price = shop_buy_price(*t);
                                                    html! {
                                                        <td class="num">
                                                            <button
                                                                onclick={mk_buy_gear_cb(slot_u8, *t)}
                                                                disabled={inv.gold < price}
                                                            >{ locale.tr(MessageId::BtnBuy) }</button>
                                                        </td>
                                                    }
                                                }) }
                                            </tr>
                                        }
                                    }) }
                                </tbody>
                            </table>
                        </section>

                        <section class="panel sage">
                            <h2>{ locale.tr(MessageId::PanelSage) }</h2>
                            <p class="muted small">
                                { locale.tr(MessageId::ShopSageDesc) }
                            </p>
                            <ul class="skill-shop">
                                { for [SKILL_SLIME_BODY, SKILL_FELINE_GRACE, SKILL_DRAGON_SCALES, SKILL_STEED_HEART].iter().map(|sid| {
                                    let owned = inv.skills_unlocked.contains_key(sid);
                                    let price = skill_buy_price(*sid).unwrap_or(u64::MAX);
                                    let cant_afford = inv.essence < price;
                                    let disabled = owned || cant_afford;
                                    let label = if owned {
                                        locale.tr(MessageId::TermOwned).to_string()
                                    } else {
                                        locale.fmt_buy_essence(price)
                                    };
                                    html! {
                                        <li class={if owned { "skill-shop-row owned" } else { "skill-shop-row" }}>
                                            <span class="skill-name">{ i18n_shared::skill_name(locale, *sid) }</span>
                                            <span class="skill-blurb muted small">{ i18n_shared::skill_blurb(locale, *sid) }</span>
                                            <button onclick={mk_buy_skill_cb(*sid)} disabled={disabled}>{ label }</button>
                                        </li>
                                    }
                                }) }
                            </ul>
                        </section>

                        <section class="panel farm">
                            <h2>{ locale.tr(MessageId::PanelFarm) }</h2>
                            <p class="muted small">
                                { locale.tr(MessageId::ShopFarmDesc) }
                            </p>
                            <p>
                                { locale.fmt_wheat_balance(
                                    &format_si(inv.wheat),
                                    &format_si(inv.wheat / WHEAT_PER_GOLD),
                                ) }
                            </p>
                            <div class="action-row">
                                <button onclick={on_work_farm}>{ locale.tr(MessageId::BtnWorkFarm) }</button>
                                <button
                                    onclick={on_sell_all_wheat}
                                    disabled={inv.wheat < WHEAT_PER_GOLD}
                                    title={locale.fmt_sell_wheat_tooltip(WHEAT_PER_GOLD as u64)}
                                >
                                    { locale.tr(MessageId::BtnSellAllWheat) }
                                </button>
                            </div>
                        </section>
                        <section class="panel resources">
                            <h2>{ locale.tr(MessageId::PanelResources) }</h2>
                            <table class="inventory">
                                <tbody>
                                    <tr><th>{ locale.tr(MessageId::ResGold) }</th><td class="num">{ format_si(inv.gold) }</td></tr>
                                    <tr><th>{ locale.tr(MessageId::ResEssence) }</th><td class="num">{ format_si(inv.essence) }</td></tr>
                                    {
                                        // Hide stash-style inventory rows when empty so the
                                        // table doesn't carry "0 fireballs" forever after a
                                        // consume. Gold/essence stay visible — they're
                                        // progress counters, not pickup-style items.
                                        if inv.potions > 0 {
                                            html! {
                                                <tr>
                                                    <th>{ locale.tr(MessageId::ResPotions) }</th>
                                                    <td class="num">{ inv.potions }</td>
                                                </tr>
                                            }
                                        } else { html! {} }
                                    }
                                    {
                                        if inv.fireballs > 0 {
                                            html! {
                                                <tr>
                                                    <th>{ locale.tr(MessageId::ResFireballs) }</th>
                                                    <td class="num">{ inv.fireballs }</td>
                                                </tr>
                                            }
                                        } else { html! {} }
                                    }
                                </tbody>
                            </table>
                        </section>
                    </>
                },
                Tab::Guilds => {
                    let my_guild_idx = my.and_then(|pk| c.guilds.membership(&pk));
                    let configured = c.guilds_key.is_some();
                    html! {
                        <>
                            <section class="panel">
                                <h2>{ locale.tr(MessageId::PanelGuilds) }</h2>
                                <p class="muted small">
                                    { locale.tr(MessageId::GuildsPanelDesc) }
                                </p>
                                {
                                    if !configured {
                                        html! {
                                            <p class="muted small">
                                                { locale.tr(MessageId::GuildsContractMissing) }
                                                <code>{ "guilds-contract" }</code>
                                                { locale.tr(MessageId::GuildsViaScript) }
                                                <code>{ "scripts/dev-publish.sh" }</code>
                                                { locale.tr(MessageId::GuildsContractMissingTail) }
                                                <code>{ "dev-keys.json" }</code>
                                                { "." }
                                            </p>
                                        }
                                    } else if my_guild_idx.is_none() {
                                        html! {
                                            <div class="guild-create">
                                                <h3>{ locale.tr(MessageId::PanelCreateGuild) }</h3>
                                                <div class="action-row">
                                                    <input
                                                        type="text"
                                                        placeholder={locale.tr(MessageId::GuildNamePlaceholder)}
                                                        value={c.new_guild_name_input.clone()}
                                                        oninput={on_guild_name_input}
                                                    />
                                                    <button class="primary"
                                                            onclick={on_create_guild}
                                                            disabled={c.new_guild_name_input.trim().is_empty()}>
                                                        { locale.tr(MessageId::BtnCreate) }
                                                    </button>
                                                </div>
                                            </div>
                                        }
                                    } else {
                                        let idx = my_guild_idx.unwrap();
                                        let g = &c.guilds.guilds[idx];
                                        let id_hex = hex::encode(g.id);
                                        let is_leader = my.map(|pk| g.leader == pk).unwrap_or(false);
                                        let leave_cb = mk_guild_leave_cb(id_hex.clone());
                                        let disband_cb = mk_guild_disband_cb(id_hex, g.name.clone());
                                        let leader_label = if is_leader {
                                            locale.tr(MessageId::TermYouLeader).to_string()
                                        } else {
                                            crate::short_id(&g.leader)
                                        };
                                        html! {
                                            <div class="guild-mine">
                                                <h3>{ locale.fmt_you_are_in_guild(&g.name) }</h3>
                                                <p class="muted small">
                                                    { locale.fmt_guild_meta(
                                                        g.members.len(),
                                                        shared::MAX_GUILD_MEMBERS,
                                                        &leader_label,
                                                    ) }
                                                </p>
                                                <div class="action-row">
                                                    <button onclick={leave_cb}>{ locale.tr(MessageId::BtnLeaveGuild) }</button>
                                                    {
                                                        if is_leader {
                                                            html! {
                                                                <button onclick={disband_cb}
                                                                        title={locale.tr(MessageId::TipDisbandLeader)}>
                                                                    { locale.tr(MessageId::BtnDisbandGuild) }
                                                                </button>
                                                            }
                                                        } else { html! {} }
                                                    }
                                                </div>
                                            </div>
                                        }
                                    }
                                }
                            </section>
                            <section class="panel">
                                <h2>{ locale.fmt_directory(c.guilds.guilds.len()) }</h2>
                                {
                                    if c.guilds.guilds.is_empty() {
                                        html! { <p class="muted small">{ locale.tr(MessageId::GuildsEmptyList) }</p> }
                                    } else {
                                        html! {
                                            <ul class="guild-list">
                                                { for c.guilds.guilds.iter().map(|g| {
                                                    let id_hex = hex::encode(g.id);
                                                    let is_mine = my.map(|pk| g.members.iter().any(|m| m == &pk)).unwrap_or(false);
                                                    let can_join = my_guild_idx.is_none() && g.members.len() < shared::MAX_GUILD_MEMBERS;
                                                    let join_cb = mk_guild_join_cb(id_hex.clone());
                                                    html! {
                                                        <li class={if is_mine { "guild-row mine" } else { "guild-row" }}>
                                                            <span class="guild-name">{ g.name.clone() }</span>
                                                            <span class="muted small">
                                                                { format!("{} / {}", g.members.len(), shared::MAX_GUILD_MEMBERS) }
                                                            </span>
                                                            <span class="muted small">
                                                                { crate::short_id(&g.leader) }
                                                            </span>
                                                            {
                                                                if is_mine {
                                                                    html! { <span class="badge live">{ locale.tr(MessageId::TermYouBadge) }</span> }
                                                                } else {
                                                                    html! {
                                                                        <button onclick={join_cb} disabled={!can_join}>
                                                                            { locale.tr(MessageId::BtnJoin) }
                                                                        </button>
                                                                    }
                                                                }
                                                            }
                                                        </li>
                                                    }
                                                }) }
                                            </ul>
                                        }
                                    }
                                }
                            </section>
                        </>
                    }
                },
                Tab::Achievements => crate::app::tabs::render_achievements_tab(
                    locale, inv, now, boss_era, boss_hp, boss_max_hp, boss_pct, total_dmg, &rows,
                ),
                Tab::Help => crate::app::tabs::render_help_tab(locale),
                Tab::Settings => html! {
                    <>
                        // Legacy / Epoch panel (backlog C1). Shows
                        // accumulated stars, spendable nodes, and
                        // the Ascend control. Always visible on
                        // Settings — the modal feel for "prestige
                        // dashboard" earns its own real estate.
                        { render_legacy_panel(c, &mk_buy_legacy_cb, on_ascend.clone()) }
                        <section class="panel settings">
                            <h2>{ locale.tr(MessageId::SettingsTitle) }</h2>

                            <h3>{ locale.tr(MessageId::SettingsLanguage) }</h3>
                            <div class="theme-picker">
                                { for [Locale::En, Locale::Ru, Locale::De].iter().map(|loc| {
                                    let is_active = c.prefs.locale == *loc;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    // Render the label in its OWN
                                    // locale (so "English" / "Русский"
                                    // / "Deutsch" always read natively,
                                    // never get translated). This is
                                    // the endonym convention used by
                                    // mainstream language pickers.
                                    let label: &'static str = match loc {
                                        Locale::En => "English",
                                        Locale::Ru => "Русский",
                                        Locale::De => "Deutsch",
                                    };
                                    html! {
                                        <button
                                            class={cls}
                                            onclick={mk_locale_cb(locale_code(*loc))}
                                            disabled={is_active}
                                        >
                                            { label }
                                        </button>
                                    }
                                }) }
                            </div>

                            <h3>{ locale.tr(MessageId::SettingsTheme) }</h3>
                            <p class="muted small">
                                { locale.tr(MessageId::SettingsThemeDesc) }
                            </p>
                            <div class="theme-picker">
                                { for THEMES.iter().map(|(id, label)| {
                                    let is_active = c.current_theme == *id;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    html! {
                                        <button
                                            class={cls}
                                            onclick={mk_theme_cb(*id)}
                                            disabled={is_active}
                                        >
                                            { *label }
                                        </button>
                                    }
                                }) }
                            </div>

                            <h3>{ locale.tr(MessageId::SettingsSyncCadence) }</h3>
                            <p class="muted small">
                                { locale.tr(MessageId::SettingsCadenceDesc) }
                            </p>
                            <div class="theme-picker">
                                { for [SyncCadence::Aggressive, SyncCadence::Normal, SyncCadence::Easy].iter().map(|cad| {
                                    let is_active = c.prefs.sync_cadence == *cad;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    html! {
                                        <button class={cls} onclick={mk_cadence_cb(*cad)} disabled={is_active}>
                                            { locale.fmt_sync_cadence(*cad) }
                                        </button>
                                    }
                                }) }
                            </div>

                            <h3>{ locale.tr(MessageId::SettingsAutoMission) }</h3>
                            <p class="muted small">
                                { locale.tr(MessageId::SettingsAutoMissionDesc) }
                            </p>
                            <div class="theme-picker">
                                { for [0u8, 25, 50].iter().map(|pct| {
                                    let is_active = c.prefs.auto_pause_hp_pct == *pct;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    let label = locale.fmt_hp_pause_label(*pct);
                                    html! {
                                        <button class={cls} onclick={mk_hp_pause_cb(*pct)} disabled={is_active}>
                                            { label }
                                        </button>
                                    }
                                }) }
                            </div>

                            <h3>{ locale.tr(MessageId::SettingsPublishBehavior) }</h3>
                            <label class="setting-toggle">
                                <input
                                    type="checkbox"
                                    checked={c.prefs.reactive_publish}
                                    onclick={mk_toggle_cb(ToggleField::ReactivePublish)}
                                />
                                { locale.tr(MessageId::SettingsPublishCheckbox) }
                            </label>

                            <h3>{ locale.tr(MessageId::SettingsIdentityBackup) }</h3>
                            <p class="muted small">
                                {
                                    if c.prefs.hide_pubkey {
                                        locale.tr(MessageId::TermPubkeyHidden).to_string()
                                    } else {
                                        pubkey_text.clone()
                                    }
                                }
                            </p>
                            {
                                if !c.prefs.hide_pubkey {
                                    if let Some(pk) = my {
                                        let hex = hex::encode(pk);
                                        html! { <p><code class="pubkey-full">{ hex }</code></p> }
                                    } else {
                                        html! { <p><span class="muted">{ locale.tr(MessageId::TermPubkeyPendingShort) }</span></p> }
                                    }
                                } else { html! {} }
                            }
                            <p class="muted small">
                                { locale.tr(MessageId::SettingsIdentityBody) }
                                <strong>{ locale.tr(MessageId::SettingsIdentityBodyStrong) }</strong>
                                { locale.tr(MessageId::SettingsIdentityBodyTail) }
                            </p>
                            <div class="action-row">
                                <button onclick={on_export_seed} disabled={my.is_none()}>
                                    { locale.tr(MessageId::BtnExportSeed) }
                                </button>
                                <button onclick={on_reset_progress} disabled={my.is_none()}>
                                    { locale.tr(MessageId::BtnResetProgress) }
                                </button>
                            </div>
                            {
                                if let Some(hex) = c.exported_seed_hex.as_ref() {
                                    html! {
                                        <div class="seed-reveal">
                                            <p class="muted small">
                                                { locale.tr(MessageId::SettingsSeedRevealWarn) }
                                            </p>
                                            <code class="pubkey-full">{ hex.clone() }</code>
                                            <div class="action-row">
                                                <button onclick={on_hide_seed.clone()}>{ locale.tr(MessageId::BtnHide) }</button>
                                            </div>
                                        </div>
                                    }
                                } else { html! {} }
                            }

                            <details class="settings-advanced">
                                <summary>{ locale.tr(MessageId::SettingsAdvanced) }</summary>
                                <p class="muted small">
                                    { locale.tr(MessageId::SettingsAdvancedDesc) }
                                </p>

                                <label class="setting-toggle">
                                    <input
                                        type="checkbox"
                                        checked={c.prefs.hide_pubkey}
                                        onclick={mk_toggle_cb(ToggleField::HidePubkey)}
                                    />
                                    { locale.tr(MessageId::SettingsHidePubkey) }
                                </label>

                                <label class="setting-toggle">
                                    <input
                                        type="checkbox"
                                        checked={c.prefs.hide_stale_players}
                                        onclick={mk_toggle_cb(ToggleField::HideStale)}
                                    />
                                    { locale.tr(MessageId::SettingsHideStale) }
                                </label>

                                <label class="setting-text">
                                    <span>{ locale.tr(MessageId::SettingsWsOverride) }</span>
                                    <input
                                        type="text"
                                        value={c.prefs.ws_url_override.clone()}
                                        oninput={on_ws_input}
                                        placeholder={DEFAULT_WS.to_string()}
                                    />
                                </label>

                                <h3 class="advanced-subhead">{ locale.tr(MessageId::SettingsResetUiPrefs) }</h3>
                                <p class="muted small">
                                    { locale.tr(MessageId::SettingsResetUiPrefsDesc) }
                                </p>
                                <div class="action-row">
                                    <button onclick={on_reset_prefs}>{ locale.tr(MessageId::BtnResetDefaults) }</button>
                                </div>

                                <h3 class="advanced-subhead">{ locale.tr(MessageId::SettingsMailbox) }</h3>
                                { render_mailbox_panel(locale, c, on_mailbox_self_test) }

                                { render_debug_overlay(c, now) }
                            </details>

                            <h3>{ locale.tr(MessageId::SettingsWhereStateLives) }</h3>
                            <p class="muted small">
                                { locale.tr(MessageId::SettingsWhereStateBody) }
                            </p>
                        </section>
                    </>
                },
            } }
        </main>
    }
}

/// Legacy / Epoch dashboard rendered on the Settings tab (C1).
/// Lists each node's current level, multiplier value, and next-cost,
/// plus the Ascend button. Hidden entirely when the player has no
/// stars *and* no purchased nodes — a fresh account doesn't need a
/// prestige UI cluttering Settings yet.
fn render_legacy_panel<F>(
    c: &Core,
    mk_buy_cb: &F,
    on_ascend: Callback<MouseEvent>,
) -> Html
where
    F: Fn(u8) -> Callback<MouseEvent>,
{
    let legacy = &c.inventory.legacy;
    let locale = c.prefs.locale;
    if legacy.stars == 0 && legacy.nodes.is_empty() && legacy.ascend_count == 0 {
        return html! {};
    }
    let next_star_lvl = ((legacy.last_awarded_level / shared::STARS_PER_N_LEVELS) + 1)
        * shared::STARS_PER_N_LEVELS;
    html! {
        <section class="panel legacy">
            <h2>{ locale.tr(MessageId::PanelLegacy) }</h2>
            <p class="muted small">
                { locale.fmt_legacy_header(legacy.stars, legacy.ascend_count, next_star_lvl) }
            </p>
            <table class="legacy-grid">
                <thead>
                    <tr>
                        <th>{ locale.tr(MessageId::LegacyColNode) }</th>
                        <th class="num">{ locale.tr(MessageId::LegacyColLevel) }</th>
                        <th class="num">{ locale.tr(MessageId::LegacyColMultiplier) }</th>
                        <th class="num">{ locale.tr(MessageId::LegacyColNextCost) }</th>
                        <th>{ "" }</th>
                    </tr>
                </thead>
                <tbody>
                    { for shared::LegacyNode::ALL.iter().map(|node| {
                        let lvl = legacy.node_level(*node);
                        let mult_bp = legacy.node_multiplier_bp(*node);
                        let cost = node.next_cost(lvl);
                        let disabled = legacy.stars < cost;
                        let cb = mk_buy_cb(node.id());
                        let mult_label = format!(
                            "×{}.{:02}",
                            mult_bp / 10_000,
                            (mult_bp % 10_000) / 100,
                        );
                        html! {
                            <tr>
                                <td>{ node.name() }</td>
                                <td class="num">{ lvl }</td>
                                <td class="num">{ mult_label }</td>
                                <td class="num">{ format!("{}★", cost) }</td>
                                <td>
                                    <button onclick={cb} disabled={disabled}>
                                        { locale.tr(MessageId::BtnBuy) }
                                    </button>
                                </td>
                            </tr>
                        }
                    }) }
                </tbody>
            </table>
            <div class="action-row">
                <button class="danger" onclick={on_ascend}>{ locale.tr(MessageId::BtnAscend) }</button>
                <span class="muted small">
                    { locale.tr(MessageId::LegacyAscendBlurb) }
                </span>
            </div>
        </section>
    }
}
