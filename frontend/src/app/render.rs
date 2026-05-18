//! Top-level renderer. `render_core` is the giant view builder
//! that produces the entire page DOM for one frame: it bakes per-
//! callback closures (Yew can't take params directly), reads
//! authoritative state from `Core`, and dispatches to per-tab
//! sub-views composed from `widgets`.

use shared::{
    form_slot_mask, form_sprite, format_si, level_of,
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
    set_idle_action_once, unequip_slot_once, use_consumable_once,
};
use crate::game::derived::{
    area_of_name, attack_from, defence_from, equipped_bonuses, max_hp_from,
    player_speed_evasion, status_code, status_text, world_boss_state, xp_in_level,
};

use super::core::{ingest_inventory, Core, ONBOARDING_STEPS};
use super::i18n::{locale_from_code, Locale, MessageId};

use super::util::DEFAULT_WS;
use super::prefs::{apply_theme, clear_all_prefs, save_prefs, SyncCadence};
use super::types::{Tab, ToggleField};
use super::util::{now_ms, truncate, webapp_contract_id};
use super::widgets::{
    render_area_card, render_battle_queue, render_battle_stage,
    render_catchup_modal, render_catchup_progress_modal, render_combat_history,
    render_confirm_modal, render_debug_overlay,
    render_equipped_slot, render_mailbox_panel, render_onboarding, render_stash_grouped,
    render_toasts, top_actions,
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

    // Leader-only disband. Stages a custom `<ConfirmModal>` since
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
                let msg = core.borrow().as_ref()
                    .map(|c| c.prefs.locale.confirm_disband_guild(&guild_name))
                    .unwrap_or_default();
                let core_for_action = core.clone();
                let pending_for_action = pending.clone();
                let bump_for_action = bump.clone();
                let guild_id_hex_owned = guild_id_hex.clone();
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.pending_confirm = Some(crate::app::core::PendingConfirm {
                        message: msg,
                        on_confirm: std::rc::Rc::new(move || {
                            guild_op_once(
                                core_for_action.clone(),
                                pending_for_action.clone(),
                                bump_for_action.clone(),
                                shared::GUILD_OP_DISBAND,
                                guild_id_hex_owned.clone(),
                            );
                        }),
                    });
                }
                bump.set(now_ms());
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
    // Bulk-sell factory: one click drops every copy of a single
    // `catalog_id` in the stash for `count × tier_price` gold.
    // Saves the player from clicking sell 50× on identical drops.
    let mk_sell_all_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |catalog_id: u16| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                // §8.A8 — confirm before liquidating a whole batch
                // when `confirm_destructive` is on. Single-row
                // sells stay one-click via `mk_sell_cb`.
                let needs_confirm = core.borrow().as_ref()
                    .map(|c| c.prefs.confirm_destructive)
                    .unwrap_or(false);
                if !needs_confirm {
                    crate::freenet::actions::gear::sell_gear_all_once(
                        core.clone(), pending.clone(), bump.clone(), catalog_id,
                    );
                    return;
                }
                // Count copies + look up tier for a richer message.
                let (count, tier) = core.borrow().as_ref()
                    .map(|c| {
                        let n = c.inventory.unequipped.iter()
                            .filter(|&&id| id == catalog_id).count();
                        let t = shared::gear_template(catalog_id)
                            .map(|tt| tt.tier).unwrap_or(0);
                        (n, t)
                    })
                    .unwrap_or((0, 0));
                let core_for_action = core.clone();
                let pending_for_action = pending.clone();
                let bump_for_action = bump.clone();
                let msg = core.borrow().as_ref()
                    .map(|c| {
                        c.prefs.locale.tr_key("confirm.sell_all_gear")
                            .replace("{count}", &count.to_string())
                            .replace("{tier}", &tier.to_string())
                    })
                    .unwrap_or_default();
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.pending_confirm = Some(crate::app::core::PendingConfirm {
                        message: msg,
                        on_confirm: std::rc::Rc::new(move || {
                            crate::freenet::actions::gear::sell_gear_all_once(
                                core_for_action.clone(),
                                pending_for_action.clone(),
                                bump_for_action.clone(),
                                catalog_id,
                            );
                        }),
                    });
                }
                bump.set(now_ms());
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
    // Buy-consumable factory. Applies an optimistic local
    // gold-debit + qty bump so the button feels instant — the
    // delegate's authoritative response overwrites the inventory
    // when it arrives (typically <50 ms locally, more over an SSH
    // tunnel to a prod node). On a rejection (e.g. price changed
    // mid-flight from a parallel call) the response's Error path
    // doesn't ingest, but a subsequent pull-tick resyncs the
    // inventory; the brief optimistic flicker is the cost of
    // never blocking the click.
    let mk_buy_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |kind: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                let (price, can_afford) = {
                    let g = core.borrow();
                    let Some(c) = g.as_ref() else { return };
                    let price = match kind {
                        CONSUMABLE_POTION => POTION_PRICE,
                        CONSUMABLE_FIREBALL => FIREBALL_PRICE,
                        _ => return,
                    };
                    (price, c.inventory.gold >= price)
                };
                if can_afford {
                    if let Some(c) = core.borrow_mut().as_mut() {
                        c.inventory.gold = c.inventory.gold.saturating_sub(price);
                        match kind {
                            CONSUMABLE_POTION => {
                                c.inventory.potions = c.inventory.potions.saturating_add(1);
                            }
                            CONSUMABLE_FIREBALL => {
                                c.inventory.fireballs = c.inventory.fireballs.saturating_add(1);
                            }
                            _ => {}
                        }
                    }
                    bump.set(now_ms());
                }
                buy_item_once(core.clone(), pending.clone(), bump.clone(), kind)
            })
        }
    };
    let on_use_potion = mk_use_cb(CONSUMABLE_POTION);
    let on_use_fireball = mk_use_cb(CONSUMABLE_FIREBALL);
    let on_buy_potion = mk_buy_cb(CONSUMABLE_POTION);
    let on_buy_fireball = mk_buy_cb(CONSUMABLE_FIREBALL);
    // Sell-stack callbacks: empty the whole pile for `count ×
    // unit_price / 2` gold. `amount == 0` is the wire signal
    // for "sell all" the delegate understands.
    let mk_sell_consumable_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |kind: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                crate::freenet::actions::shop::sell_consumable_once(
                    core.clone(), pending.clone(), bump.clone(), kind, 0,
                )
            })
        }
    };
    let on_sell_potions = mk_sell_consumable_cb(CONSUMABLE_POTION);
    let on_sell_fireballs = mk_sell_consumable_cb(CONSUMABLE_FIREBALL);

    // World-map view switcher (Linear ↔ Wilds). UI-only state on
    // `Core::map_view`; flip + bump triggers a re-render.
    let mk_map_view_cb = {
        let core = core_cell.clone();
        let bump = bump.clone();
        move |view: crate::app::types::MapView| {
            let core = core.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.map_view = view;
                }
                bump.set(now_ms());
            })
        }
    };

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
                // §8.A8 — wrap form-change in a custom confirm
                // modal when `confirm_destructive` is on. Skips
                // the modal otherwise for the legacy one-click
                // behaviour.
                let needs_confirm = core.borrow().as_ref()
                    .map(|c| c.prefs.confirm_destructive)
                    .unwrap_or(false);
                if !needs_confirm {
                    crate::freenet::actions::shop::buy_form_once(
                        core.clone(), pending.clone(), bump.clone(), form,
                    );
                    return;
                }
                let core_for_action = core.clone();
                let pending_for_action = pending.clone();
                let bump_for_action = bump.clone();
                let msg = core.borrow().as_ref()
                    .map(|c| {
                        let form_name = crate::app::i18n_shared::form_name(c.prefs.locale, form);
                        c.prefs.locale.tr_key("confirm.form_change")
                            .replace("{form}", form_name)
                    })
                    .unwrap_or_default();
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.pending_confirm = Some(crate::app::core::PendingConfirm {
                        message: msg,
                        on_confirm: std::rc::Rc::new(move || {
                            crate::freenet::actions::shop::buy_form_once(
                                core_for_action.clone(),
                                pending_for_action.clone(),
                                bump_for_action.clone(),
                                form,
                            );
                        }),
                    });
                }
                bump.set(now_ms());
            })
        }
    };

    let on_auto_equip = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| auto_equip_once(core.clone(), pending.clone(), bump.clone()))
    };
    // `on_work_farm` removed with the Farm panel — passive Estate
    // yield is the only way to gain wheat now. `work_farm_once`
    // remains in the RPC module for now in case the action is
    // resurrected for tutorials.
    let on_sell_all_wheat = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| sell_wheat_once(core.clone(), pending.clone(), bump.clone(), 0))
    };
    let on_convert_all_essence = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| {
            crate::freenet::actions::convert_essence_to_gold_once(
                core.clone(),
                pending.clone(),
                bump.clone(),
                0,
            )
        })
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
    // Bulk-buy factory: `count == 0` is the "max-affordable"
    // wire signal the delegate caps at 100. Used by the +10 /
    // ×max buttons next to single-Buy on the shop gear table.
    let mk_bulk_buy_gear_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |slot: u8, tier: u8, count: u32| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                crate::freenet::actions::gear::bulk_buy_gear_roll_once(
                    core.clone(), pending.clone(), bump.clone(), slot, tier, count,
                )
            })
        }
    };
    // Same idea for shop consumables.
    let mk_bulk_buy_item_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |kind: u8, count: u32| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                crate::freenet::actions::shop::bulk_buy_item_once(
                    core.clone(), pending.clone(), bump.clone(), kind, count,
                )
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
                // Optimistic gold-debit + worker count bump so the
                // Hire button doesn't feel mushy over a tunnelled WS.
                // Snapshot the price under the same `tier_id`/`owned`
                // the delegate will see — collisions with a parallel
                // hire are ironed out on the response ingest.
                let mutated = {
                    let g = core.borrow();
                    let Some(c) = g.as_ref() else { return };
                    let Some(tier) = shared::estate_tier(tier_id) else { return };
                    let owned = c.inventory.estate.workers_of(tier_id);
                    let price = shared::estate_next_price_with_discount(
                        tier, owned, c.inventory.insight.frugality_mult_bp(),
                    );
                    c.inventory.gold >= price
                };
                if mutated {
                    if let Some(c) = core.borrow_mut().as_mut() {
                        if let Some(tier) = shared::estate_tier(tier_id) {
                            let owned = c.inventory.estate.workers_of(tier_id);
                            let price = shared::estate_next_price_with_discount(
                                tier, owned, c.inventory.insight.frugality_mult_bp(),
                            );
                            c.inventory.gold = c.inventory.gold.saturating_sub(price);
                            c.inventory.estate.hire(tier_id);
                        }
                    }
                    bump.set(now_ms());
                }
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
    // ×N bulk variant — single closure factory for both ×10 and
    // max-buy. Used by Legacy + Insight panels.
    let mk_buy_legacy_bulk_cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        move |node_id: u8, count: u32| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_| {
                crate::freenet::actions::legacy::buy_legacy_node_bulk_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    node_id,
                    count,
                )
            })
        }
    };

    // Ascend handler — soft-resets the run. Stages our custom
    // `<ConfirmModal>` (§8.A8) instead of the browser-native
    // `window.confirm()` so the prompt matches the rest of the UI.
    let on_ascend = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            let core_for_action = core.clone();
            let pending_for_action = pending.clone();
            let bump_for_action = bump.clone();
            let msg = core.borrow().as_ref()
                .map(|c| c.prefs.locale.tr(MessageId::LegacyAscendConfirm).to_string())
                .unwrap_or_default();
            if let Some(c) = core.borrow_mut().as_mut() {
                c.pending_confirm = Some(crate::app::core::PendingConfirm {
                    message: msg,
                    on_confirm: std::rc::Rc::new(move || {
                        crate::freenet::actions::legacy::ascend_once(
                            core_for_action.clone(),
                            pending_for_action.clone(),
                            bump_for_action.clone(),
                        );
                    }),
                });
            }
            bump.set(now_ms());
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
        let pending = pending.clone();
        let bump = bump.clone();
        move |pct: u8| {
            let core = core.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            Callback::from(move |_: MouseEvent| {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.prefs.auto_pause_hp_pct = pct;
                    save_prefs(&c.prefs);
                }
                // Mirror the picker onto the delegate-side Settings
                // blob so the value survives a reload in the
                // null-origin sandbox iframe.
                crate::freenet::actions::settings::save_settings_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(pct),
                );
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
                        ToggleField::HideGold => {
                            c.prefs.hide_gold = !c.prefs.hide_gold;
                        }
                        ToggleField::HideBossDamage => {
                            c.prefs.hide_boss_damage = !c.prefs.hide_boss_damage;
                        }
                        ToggleField::ReducedMotion => {
                            c.prefs.reduced_motion = !c.prefs.reduced_motion;
                            crate::app::prefs::apply_visual_prefs(&c.prefs);
                        }
                        ToggleField::ReducedFlash => {
                            c.prefs.reduced_flash = !c.prefs.reduced_flash;
                            crate::app::prefs::apply_visual_prefs(&c.prefs);
                        }
                        ToggleField::OverlayMode => {
                            c.prefs.overlay_mode = !c.prefs.overlay_mode;
                            crate::app::prefs::apply_visual_prefs(&c.prefs);
                        }
                        ToggleField::KeyboardShortcuts => {
                            c.prefs.keyboard_shortcuts = !c.prefs.keyboard_shortcuts;
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
            // §8.A8 — stage a confirm prompt. The custom
            // `<ConfirmModal>` runs the captured closure on OK.
            let core_for_action = core.clone();
            let pending_for_action = pending.clone();
            let bump_for_action = bump.clone();
            let msg = core.borrow().as_ref()
                .map(|c| c.prefs.locale.confirm_reset_progress().to_string())
                .unwrap_or_default();
            if let Some(c) = core.borrow_mut().as_mut() {
                c.pending_confirm = Some(crate::app::core::PendingConfirm {
                    message: msg,
                    on_confirm: std::rc::Rc::new(move || {
                        reset_inventory_once(
                            core_for_action.clone(),
                            pending_for_action.clone(),
                            bump_for_action.clone(),
                        );
                    }),
                });
            }
            bump.set(now_ms());
        })
    };

    let on_export_seed = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            // Stage confirm; OK handler clones run the real
            // `export_seed_once` and update Core with the seed hex.
            let core_for_action = core.clone();
            let pending_for_action = pending.clone();
            let bump_for_action = bump.clone();
            let msg = core.borrow().as_ref()
                .map(|c| c.prefs.locale.confirm_reveal_seed().to_string())
                .unwrap_or_default();
            if let Some(c) = core.borrow_mut().as_mut() {
                c.pending_confirm = Some(crate::app::core::PendingConfirm {
                    message: msg,
                    on_confirm: std::rc::Rc::new(move || {
                        let core_for_cb = core_for_action.clone();
                        let bump_for_cb = bump_for_action.clone();
                        export_seed_once(
                            core_for_action.clone(),
                            pending_for_action.clone(),
                            move |result| {
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
                            },
                        );
                    }),
                });
            }
            bump.set(now_ms());
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
                None,
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
    // Dependency token for `GraphEdgeOverlay` — changes whenever
    // layout-affecting state shifts. We don't need precise tracking;
    // a bump every few inventory mutations is enough for the SVG to
    // re-measure node positions.
    let map_bump: u64 = c.inventory.area_clears.values().copied().sum::<u64>()
        .wrapping_add(c.inventory.current_area as u64)
        .wrapping_add(match c.map_view {
            crate::app::types::MapView::Linear => 0,
            crate::app::types::MapView::Wilds => 1_000_000,
        });
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
        let own_area = shared::current_area_def(&c.inventory);
        let own_locale = c.prefs.locale;
        let own_area_name = i18n_shared::area_name(own_locale, &own_area).to_string();
        let own_champion = c.inventory.tokens.owns(shared::TokenPerk::ChampionBadge);
        // §E-tier: own row should preview the cosmetics the player
        // will publish on the next heartbeat. Without this the
        // motto/accent show up only on OTHER players' clients but
        // never on the player's own row — confusing "did my publish
        // work" UX.
        rows.push((
            my,
            PresencePayload::new_with_cosmetics(
                my,
                c.name.clone(),
                c.inventory.gold,
                c.inventory.boss_damage,
                own_area_name,
                c.last_published_ms.unwrap_or(0),
                c.inventory.current_area,
                own_champion,
                c.inventory.routine.public_motto.clone(),
                c.inventory.routine.public_accent,
                c.inventory.routine.public_frame,
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
        .map(|pk| {
            let short = crate::short_id(&pk);
            crate::app::i18n_loader::fmt(
                locale.as_str(),
                "render.pubkey_from_delegate",
                &[("short_id", short.as_str())],
            )
        })
        .unwrap_or_else(|| locale.tr(MessageId::TermPubkeyPending).to_string());

    let auto_label = if c.inventory.auto_run_enabled {
        locale.tr(MessageId::BtnAutoOn)
    } else {
        locale.tr(MessageId::BtnAutoOff)
    };
    // §⚠️#2 (2026-05-18): Estate yield is parallel for every player;
    // delegate no longer rejects Run Mission while Estate is the
    // active idle action. The visual gate is gone.
    let mission_disabled = my.is_none() || c.mission_in_flight;

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
    // Owned AreaDef so Wilds (id ≥ 100) returns its dynamic
    // entry instead of falling through to the Village starter.
    let area = shared::current_area_def(inv);
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
    // The manual "+1 wheat" Work Farm button is hidden once any
    // Estate Farmhand worker exists — they produce wheat
    // passively at a much higher rate and the manual click
    // would feel like a no-op. New players without a Farmhand
    // still get the original click-to-farm path.
    let _farmhand_active = inv.base.base.estate.workers_of(0) > 0;
    // Show the Linear/Wilds map switcher once the player has
    // some buffer over Wilds-entrance min_level (15). Five-level
    // buffer so the option appears slightly before it becomes
    // useful — players get to see the second map exists.
    let wilds_unlocked = lvl + 5 >= 15;
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

    // Custom confirm modal (§8.A8) — OK fires the staged closure
    // and clears the slot; Cancel just clears the slot.
    let on_confirm_ok = {
        let core = core_cell.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            let action = if let Some(c) = core.borrow_mut().as_mut() {
                c.pending_confirm.take().map(|p| p.on_confirm)
            } else { None };
            if let Some(on_ok) = action {
                on_ok();
            }
            bump.set(now_ms());
        })
    };
    let on_confirm_cancel = {
        let core = core_cell.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(c) = core.borrow_mut().as_mut() {
                c.pending_confirm = None;
            }
            bump.set(now_ms());
        })
    };

    html! {
        <main>
            { render_toasts(&c.toasts, now) }
            { render_onboarding(locale, c.onboarding_step, on_onboarding_next, on_onboarding_skip) }
            { render_catchup_modal(c, locale, on_catchup_dismiss) }
            { render_catchup_progress_modal(c, locale) }
            { render_confirm_modal(locale, &c.pending_confirm, on_confirm_ok, on_confirm_cancel) }
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
                    // §8 B3: user-hidden tabs. Bit `i` set = hide
                    // the i-th tab. Home/Settings/Help are
                    // protected — hiding them would deadlock the
                    // UI (no way to get back to settings to unhide).
                    let idx = *tab as u8;
                    let protected = matches!(tab, Tab::Home | Tab::Settings | Tab::Help);
                    if !protected && (c.prefs.hidden_tabs & (1u32 << idx)) != 0 {
                        return false;
                    }
                    // Phased reveal (A5): tabs stay hidden until
                    // their reveal-bit latches on. Home / Settings /
                    // Help are always shown so a fresh player has
                    // somewhere to be.
                    match tab {
                        Tab::Home | Tab::Settings | Tab::Help => true,
                        Tab::WorldMap => inv.revealed_has(shared::RevealKey::WorldMap),
                        Tab::Shop => inv.revealed_has(shared::RevealKey::Shop),
                        Tab::Guilds => inv.revealed_has(shared::RevealKey::Guilds),
                        Tab::Achievements => inv.revealed_has(shared::RevealKey::Achievements),
                        // Mastery surfaces once the player has earned
                        // their first Legacy star — that's the same
                        // reveal that used to render the Legacy
                        // panel inside Settings.
                        Tab::Mastery => {
                            inv.legacy.stars > 0
                                || !inv.legacy.nodes.is_empty()
                                || inv.legacy.ascend_count > 0
                                || inv.insight.last_awarded_mission > 0
                                || inv.tokens.last_awarded_boss_damage > 0
                                || inv.mission_count >= shared::BOSS_ATTACK_MIN_MISSIONS / 2
                                || inv.base.base.estate.workers.values().any(|n| *n > 0)
                        }
                    }
                }).map(|(icon, label, tab)| {
                    let is_active = c.current_tab == *tab;
                    let anim = match tab {
                        Tab::WorldMap => anim_cls(shared::RevealKey::WorldMap),
                        Tab::Shop => anim_cls(shared::RevealKey::Shop),
                        Tab::Guilds => anim_cls(shared::RevealKey::Guilds),
                        Tab::Achievements => anim_cls(shared::RevealKey::Achievements),
                        Tab::Home | Tab::Settings | Tab::Help | Tab::Mastery => "",
                    };
                    let cls = classes!(
                        "icon-btn",
                        if is_active { "active" } else { "" },
                        anim,
                    );
                    // §8 D4: data-shortcut="1".."9" — pressed
                    // keys 1-9 switch to the corresponding visible
                    // tab. Indexing AFTER the reveal-bit + hidden_tabs
                    // filter so the digit lines up with what the
                    // player actually sees.
                    let shortcut = format!("{}", (*tab as usize) + 1);
                    html! {
                        <button
                            class={cls}
                            data-shortcut={shortcut}
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
                Tab::Home => html! {
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
                                <h2>
                                    { locale.tr(MessageId::PanelHero) }
                                    {
                                        // Champion badge perk (C2 tokens) — shows a
                                        // permanent marker on the Hero panel header
                                        // once the player owns it.
                                        if inv.tokens.owns(shared::TokenPerk::ChampionBadge) {
                                            html! {
                                                <span
                                                    class="champion-badge"
                                                    title={ locale.tr_key("token_perk_name.champion_badge") }
                                                >{ "🏆" }</span>
                                            }
                                        } else { html! {} }
                                    }
                                </h2>
                                <div class="stat-row">
                                    <label>{ format!("{} ", locale.tr(MessageId::StatName)) }
                                        <input type="text" value={c.name.clone()} oninput={on_name} />
                                    </label>
                                </div>
                                {
                                    // §8 C6: motto display under hero name. Hidden when empty.
                                    if !c.prefs.motto.is_empty() {
                                        html! { <p class="hero-motto muted small">{ &c.prefs.motto }</p> }
                                    } else { html! {} }
                                }
                                { render_daily_checkin(c, locale, core_cell.clone(), pending.clone(), bump.clone()) }
                                <table class="statgrid">
                                    <tbody>
                                        <tr>
                                            <th>{ locale.tr(MessageId::StatForm) }</th>
                                            <td class="num">
                                                <span class="form-name">
                                                    {
                                                        // §8 C5: hero_skin override. Trust only
                                                        // non-empty strings whose chars are in
                                                        // the picker whitelist (single emoji glyph).
                                                        format!(
                                                            "{} {}",
                                                            if !c.prefs.hero_skin.is_empty() {
                                                                c.prefs.hero_skin.clone()
                                                            } else {
                                                                form_sprite(inv.current_form).to_string()
                                                            },
                                                            i18n_shared::form_name(locale, inv.current_form),
                                                        )
                                                    }
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
                                {
                                    // §UX 2026-05-17 fix: HP-regen banner.
                                    // When auto-mission is on AND the
                                    // player isn't currently in a battle
                                    // AND HP is below max, surface the
                                    // remaining regen time so the player
                                    // knows the loop is paused for a
                                    // known reason, not stuck. Hidden
                                    // when HP is full (loop will pick up
                                    // on next tick) or when manually idle.
                                    if inv.auto_run_enabled
                                        && inv.current_battle.is_none()
                                        && inv.current_hp < hp_max
                                    {
                                        let missing = hp_max.saturating_sub(inv.current_hp);
                                        let secs = if hp_max == 0 { 0 } else {
                                            (missing * shared::HP_FULL_REGEN_MS / hp_max / 1000).max(1)
                                        };
                                        html! {
                                            <p class="hp-regen-banner muted small">
                                                { locale.tr_key("hp_regen.banner")
                                                    .replace("{secs}", &secs.to_string()) }
                                            </p>
                                        }
                                    } else { html! {} }
                                }
                                {
                                    // §8 C4: pubkey display variant gate.
                                    // Hidden → suppress; Short/Full → show
                                    // (Full is the legacy long-form; Short
                                    // reads `pubkey_text` which already
                                    // formats as `tag…short_id` via the
                                    // locale string).
                                    match c.prefs.pubkey_display {
                                        crate::app::prefs::PUBKEY_DISPLAY_HIDDEN => html! {},
                                        _ => html! { <p class="muted small">{ &pubkey_text }</p> },
                                    }
                                }
                            </article>

                            <article class="panel scene">
                                <h2>{ format!("{chap_title}") }</h2>
                                {
                                    // Sprite stage / HP bars — battle view replaces
                                    // the static emojis only for the actual visual.
                                    // Action row (Run Mission + auto) stays put.
                                    if let Some(battle) = inv.current_battle.as_ref() {
                                        render_battle_stage(locale, battle, inv, hp_max, &c.prefs)
                                    } else {
                                        html! {
                                            <div class="stage">
                                                <div class="hero-sprite">{
                                                    if !c.prefs.hero_skin.is_empty() {
                                                        c.prefs.hero_skin.clone()
                                                    } else {
                                                        form_sprite(inv.current_form).to_string()
                                                    }
                                                }</div>
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
                                            data-shortcut="M"
                                            onclick={on_run_mission}
                                            disabled={mission_disabled || inv.current_battle.is_some()}
                                            title={
                                                if inv.current_battle.is_some() {
                                                    locale.tr(MessageId::TipFightInProgress)
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
                                            // §⚠️#2 (2026-05-18): Estate yields run in
                                            // parallel with combat for every player, so
                                            // toggling auto-mission no longer fights the
                                            // Estate's single-active-action slot.
                                            let auto_disabled = my.is_none();
                                            let auto_tip = if inv.current_battle.is_some() {
                                                locale.tr(MessageId::TipAutoToggleMidFight)
                                            } else { "" };
                                            html! {
                                                <button class={classes!(anim_cls(shared::RevealKey::AutoMission))}
                                                        data-shortcut="A"
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
                                        render_battle_queue(locale, battle, inv, &c.prefs)
                                    } else {
                                        html! {
                                            <p class="tooltip muted">
                                                {
                                                    locale.fmt_mission_summary(
                                                        i18n_shared::area_name(locale, &area),
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
                                // §8 D5 bit 2: keep per-battle history
                                // visible (it carries outcome + gold +
                                // turn count) — only the per-encounter
                                // "dealt N taken N" tail is suppressed.
                                // The per-turn ticker DURING a battle is
                                // gated separately inside
                                // `render_battle_queue`.
                                { render_combat_history(locale, &inv.combat_history, (c.prefs.numerical_assists & 0b100) != 0) }
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
                                                    data-shortcut="E"
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

                        // Phased reveal (A5): World Boss panel appears
                        // at mission_count ≥ 10, but only while the
                        // player is actually on a boss-contact area
                        // (`damage_mult > 0`) — non-boss zones hide
                        // the HP gauge so the panel doesn't claim
                        // relevance from areas that can't chip it.
                        {
                            if inv.revealed_has(shared::RevealKey::WorldBoss)
                                && area.damage_mult > 0
                            {
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
                                                    let next_price = shared::estate_next_price_with_discount(
                                                        tier, owned, inv.insight.frugality_mult_bp(),
                                                    );
                                                    let insight_aff = inv.insight.node_level(shared::InsightNode::FormAffinity);
                                                    let aff_bp = shared::form_affinity_bp_with_insight(
                                                        inv.current_form, tier.id, insight_aff,
                                                    );
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
                                                    let tier_key = format!("estate_tier_name.{}", tier.id);
                                                    let tier_name_tr = locale.tr_key(&tier_key);
                                                    let tier_label: &str = if tier_name_tr.starts_with('?') { tier.name } else { tier_name_tr };
                                                    html! {
                                                        <tr>
                                                            <td>{ tier_label }</td>
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
                                    <tr><th>{ locale.tr(MessageId::ResGold) }</th><td class="num">{ crate::app::prefs::render_gold(c.prefs.hide_gold, inv.gold) }</td></tr>
                                    <tr>
                                        <th>{ locale.tr(MessageId::ResEssence) }</th>
                                        <td class="num">{ format_si(inv.essence) }</td>
                                    </tr>
                                    <tr class="res-divider"><td colspan="2" class="muted small">{ locale.tr_key("res.progressive_group") }</td></tr>
                                    <tr><th>{ locale.tr(MessageId::ResMissions) }</th><td class="num">{ format_si(inv.mission_count) }</td></tr>
                                    <tr><th>{ locale.tr(MessageId::ResBossDamage) }</th><td class="num">{ crate::app::prefs::render_boss_damage(c.prefs.hide_boss_damage, inv.boss_damage) }</td></tr>
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
                                { locale.fmt_currently_farming(i18n_shared::area_name(locale, &area), lvl) }
                            </p>
                            // Map-view switcher (Linear ↔ Wilds).
                            // Wilds tab is gated by entrance level
                            // so a fresh player doesn't see a
                            // locked second option staring back at
                            // them. Selection is UI-only — picking
                            // a node from either view still goes
                            // through the same `SetArea` RPC.
                            {
                                if wilds_unlocked {
                                    html! {
                                        <div class="map-switcher">
                                            <button
                                                class={if c.map_view == crate::app::types::MapView::Linear { "primary" } else { "" }}
                                                onclick={mk_map_view_cb(crate::app::types::MapView::Linear)}>
                                                { locale.tr(MessageId::MapViewLinear) }
                                            </button>
                                            <button
                                                class={if c.map_view == crate::app::types::MapView::Wilds { "primary" } else { "" }}
                                                onclick={mk_map_view_cb(crate::app::types::MapView::Wilds)}
                                                title={
                                                    // §P3 wilds preview tooltip. Count
                                                    // landmark-bearing Wilds areas the
                                                    // player hasn't claimed yet.
                                                    let mut unclaimed = 0u32;
                                                    for area_id in shared::WILDS_AREA_BASE..=255u8 {
                                                        if shared::wilds_landmark(area_id).is_some()
                                                            && !inv.landmark_claims.contains_key(&area_id)
                                                        {
                                                            unclaimed += 1;
                                                        }
                                                    }
                                                    locale.tr_key("map.wilds_tooltip")
                                                        .replace("{n}", &unclaimed.to_string())
                                                }
                                            >
                                                { locale.tr(MessageId::MapViewWilds) }
                                            </button>
                                        </div>
                                    }
                                } else { html! {} }
                            }
                            {
                                if c.map_view == crate::app::types::MapView::Linear {
                                    html! {
                                        <div id="area-graph-linear" class="area-graph">
                                            <crate::app::widgets::GraphEdgeOverlay
                                                host_id={"area-graph-linear"}
                                                bump={map_bump}
                                            />
                                            { for area_columns.iter().map(|(depth, row_areas)| html! {
                                                <div class={classes!("graph-row", format!("depth-{}", depth))}>
                                                    { for row_areas.iter().map(|a| {
                                                        let has_parent = !a.predecessors.is_empty();
                                                        let parent_ids_csv = a.predecessors.iter()
                                                            .map(|p| p.to_string())
                                                            .collect::<Vec<_>>()
                                                            .join(",");
                                                        let node_cls = if has_parent { "graph-node has-parent" } else { "graph-node starter" };
                                                        let area_id_str = a.id.to_string();
                                                        html! {
                                                            <div
                                                                class={node_cls}
                                                                data-area-id={area_id_str}
                                                                data-parent-ids={parent_ids_csv}
                                                            >
                                                                { render_area_card(locale, a, inv.current_area, lvl, inv, &mk_set_area_cb) }
                                                            </div>
                                                        }
                                                    }) }
                                                </div>
                                            }) }
                                        </div>
                                    }
                                } else {
                                    render_wilds_panel_body(c, locale, &mk_set_area_cb)
                                }
                            }
                        </section>
                        // Per-zone activities panel (A1).
                        { render_activities_panel(c, locale, core_cell.clone(), pending.clone(), bump.clone()) }
                        // §8 A5 spoiler-safe — hide plot copy when
                        // the toggle is on so streamers can show the
                        // session without revealing the story beat.
                        {
                            if c.prefs.spoiler_safe { html! {} } else {
                                render_collapsible_panel(
                                    "panel plot",
                                    crate::app::prefs::PANEL_BIT_PLOT,
                                    &locale.tr(MessageId::PanelPlotSoFar).to_string(),
                                    html! {
                                        <>
                                            <p class="chapter-no muted">{ locale.fmt_chapter(chap_no as u64) }</p>
                                            <p>{ chap_body_map }</p>
                                        </>
                                    },
                                    c.prefs.collapsed_panels,
                                    core_cell.clone(),
                                    bump.clone(),
                                )
                            }
                        }
                    </>
                },
                Tab::Shop => html! {
                    <>
                        <section class="panel shop">
                            <h2>{ locale.tr(MessageId::PanelShop) }</h2>
                            <p class="muted small">
                                { locale.fmt_shop_balance(
                                    &crate::app::prefs::render_gold(c.prefs.hide_gold, inv.gold),
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
                                    <div class="shop-item-actions">
                                        <button
                                            onclick={on_buy_potion}
                                            disabled={inv.gold < POTION_PRICE}
                                        >
                                            { locale.fmt_buy_gold(POTION_PRICE) }
                                        </button>
                                        <button
                                            onclick={mk_bulk_buy_item_cb(CONSUMABLE_POTION, 10)}
                                            disabled={inv.gold < POTION_PRICE.saturating_mul(10)}
                                            title={locale.tr_key("shop.buy_x10_tooltip")}
                                        >
                                            { locale.tr_key("shop.buy_x10")
                                                .replace("{gold}", &(POTION_PRICE * 10).to_string()) }
                                        </button>
                                        <button
                                            onclick={mk_bulk_buy_item_cb(CONSUMABLE_POTION, 0)}
                                            disabled={inv.gold < POTION_PRICE}
                                            title={locale.tr_key("shop.buy_max_tooltip")}
                                        >
                                            { locale.tr_key("shop.buy_max") }
                                        </button>
                                        {
                                            if inv.potions > 0 {
                                                let unit = shared::consumable_sell_price(
                                                    CONSUMABLE_POTION).unwrap_or(0);
                                                let total = unit.saturating_mul(inv.potions as u64);
                                                html! {
                                                    <button onclick={on_sell_potions.clone()}
                                                            title={locale.tr_key("shop.sell_potions_tooltip")
                                                                .replace("{count}", &inv.potions.to_string())
                                                                .replace("{gold}", &total.to_string())}>
                                                        { locale.tr_key("shop.sell_consumable")
                                                            .replace("{count}", &inv.potions.to_string())
                                                            .replace("{gold}", &total.to_string()) }
                                                    </button>
                                                }
                                            } else { html! {} }
                                        }
                                    </div>
                                </div>
                                <div class="shop-item">
                                    <span class="name">{ locale.tr(MessageId::ItemFireball) }</span>
                                    <span class="desc muted">
                                        { locale.tr_key("shop.fireball_desc")
                                            .replace("{dmg}", &FIREBALL_BOSS_DAMAGE.to_string()) }
                                    </span>
                                    <div class="shop-item-actions">
                                        <button
                                            onclick={on_buy_fireball}
                                            disabled={inv.gold < FIREBALL_PRICE}
                                        >
                                            { locale.fmt_buy_gold(FIREBALL_PRICE) }
                                        </button>
                                        <button
                                            onclick={mk_bulk_buy_item_cb(CONSUMABLE_FIREBALL, 10)}
                                            disabled={inv.gold < FIREBALL_PRICE.saturating_mul(10)}
                                            title={locale.tr_key("shop.buy_x10_tooltip")}
                                        >
                                            { locale.tr_key("shop.buy_x10")
                                                .replace("{gold}", &(FIREBALL_PRICE * 10).to_string()) }
                                        </button>
                                        <button
                                            onclick={mk_bulk_buy_item_cb(CONSUMABLE_FIREBALL, 0)}
                                            disabled={inv.gold < FIREBALL_PRICE}
                                            title={locale.tr_key("shop.buy_max_tooltip")}
                                        >
                                            { locale.tr_key("shop.buy_max") }
                                        </button>
                                        {
                                            if inv.fireballs > 0 {
                                                let unit = shared::consumable_sell_price(
                                                    CONSUMABLE_FIREBALL).unwrap_or(0);
                                                let total = unit.saturating_mul(inv.fireballs as u64);
                                                html! {
                                                    <button onclick={on_sell_fireballs.clone()}
                                                            title={locale.tr_key("shop.sell_fireballs_tooltip")
                                                                .replace("{count}", &inv.fireballs.to_string())
                                                                .replace("{gold}", &total.to_string())}>
                                                        { locale.tr_key("shop.sell_consumable")
                                                            .replace("{count}", &inv.fireballs.to_string())
                                                            .replace("{gold}", &total.to_string()) }
                                                    </button>
                                                }
                                            } else { html! {} }
                                        }
                                    </div>
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

                        { render_collapsible_panel(
                            "panel stash",
                            crate::app::prefs::PANEL_BIT_STASH,
                            &locale.fmt_stash_header(inv.unequipped.len()),
                            html! {
                                <>
                                    <p class="muted small">{ locale.tr(MessageId::ShopStashDesc) }</p>
                                    { render_stash_toolbar(c, locale, core_cell.clone(), bump.clone()) }
                                    { render_stash_grouped(
                                        locale, inv,
                                        c.prefs.stash_filter, c.prefs.stash_sort,
                                        &mk_equip_cb, &mk_sell_cb, &mk_sell_all_cb, &mk_forge_cb,
                                    ) }
                                </>
                            },
                            c.prefs.collapsed_panels,
                            core_cell.clone(),
                            bump.clone(),
                        ) }

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
                                                    let bulk10_price = price.saturating_mul(10);
                                                    html! {
                                                        <td class="num">
                                                            <div class="buy-grid-actions">
                                                                <button
                                                                    onclick={mk_buy_gear_cb(slot_u8, *t)}
                                                                    disabled={inv.gold < price}
                                                                >{ locale.tr(MessageId::BtnBuy) }</button>
                                                                <button
                                                                    onclick={mk_bulk_buy_gear_cb(slot_u8, *t, 10)}
                                                                    disabled={inv.gold < bulk10_price}
                                                                    title={format!("buy 10 at {bulk10_price}g")}
                                                                >{ "×10" }</button>
                                                                <button
                                                                    onclick={mk_bulk_buy_gear_cb(slot_u8, *t, 0)}
                                                                    disabled={inv.gold < price}
                                                                    title="buy as many as gold allows (capped at 100)"
                                                                >{ "max" }</button>
                                                            </div>
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
                                    // §P3 skill-up animation: tag the
                                    // row with `skill-unlock-anim` if
                                    // the skill just landed (set in
                                    // core.animate_skills).
                                    let mut row_cls = if owned { "skill-shop-row owned".to_string() } else { "skill-shop-row".to_string() };
                                    if c.animate_skills.contains(sid) {
                                        row_cls.push_str(" skill-unlock-anim");
                                    }
                                    html! {
                                        <li class={row_cls}>
                                            <span class="skill-name">{ i18n_shared::skill_name(locale, *sid) }</span>
                                            <span class="skill-blurb muted small">{ i18n_shared::skill_blurb(locale, *sid) }</span>
                                            <button onclick={mk_buy_skill_cb(*sid)} disabled={disabled}>{ label }</button>
                                        </li>
                                    }
                                }) }
                            </ul>
                        </section>

                        // §C: Merchant exchange — promoted out of the
                        // Resources row to its own panel.
                        { render_exchange_panel(c, locale, on_convert_all_essence.clone(), on_sell_all_wheat.clone()) }

                        <section class="panel resources">
                            <h2>{ locale.tr(MessageId::PanelResources) }</h2>
                            <table class="inventory">
                                <tbody>
                                    <tr><th>{ locale.tr(MessageId::ResGold) }</th><td class="num">{ crate::app::prefs::render_gold(c.prefs.hide_gold, inv.gold) }</td></tr>
                                    <tr>
                                        <th>{ locale.tr(MessageId::ResEssence) }</th>
                                        <td class="num">{ format_si(inv.essence) }</td>
                                    </tr>
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
                                    {
                                        // Wheat row appears only once the
                                        // player has accumulated some — Estate
                                        // Farmhand path produces it passively;
                                        // for fresh players the Resources panel
                                        // doesn't carry a zero stub.
                                        if inv.wheat > 0 {
                                            html! {
                                                <tr>
                                                    <th>{ locale.tr_key("res.wheat") }</th>
                                                    <td class="num">{ format_si(inv.wheat) }</td>
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
                Tab::Mastery => html! {
                    <>
                        <section class="panel mastery-intro">
                            <h2>{ locale.tr(MessageId::TabMastery) }</h2>
                            <p class="muted small">
                                { locale.tr(MessageId::MasteryIntro) }
                            </p>
                        </section>
                        { render_legacy_panel(c, &mk_buy_legacy_cb, &mk_buy_legacy_bulk_cb, on_ascend.clone()) }
                        { render_routine_panel(c, locale, core_cell.clone(), pending.clone(), bump.clone()) }
                        { render_insight_panel(c, locale, core_cell.clone(), pending.clone(), bump.clone()) }
                        { render_boss_attack_panel(c, locale, core_cell.clone(), pending.clone(), bump.clone()) }
                        { render_tokens_panel(c, locale, core_cell.clone(), pending.clone(), bump.clone()) }
                    </>
                },
                Tab::Settings => html! {
                    <>
                        // Legacy / Epoch panel (backlog C1). Shows
                        // accumulated stars, spendable nodes, and
                        // the Ascend control. Always visible on
                        // Settings — the modal feel for "prestige
                        // dashboard" earns its own real estate.
                        <section class="panel settings">
                            <h2>{ locale.tr(MessageId::SettingsTitle) }</h2>

                            <h3>{ locale.tr(MessageId::SettingsLanguage) }</h3>
                            <div class="theme-picker">
                                { for crate::app::i18n::available_locales().into_iter().map(|loc| {
                                    let is_active = c.prefs.locale == loc;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    // Endonym (locale's own name) keeps the
                                    // button readable to a speaker of that
                                    // language regardless of current locale.
                                    let label = loc.endonym();
                                    html! {
                                        <button
                                            class={cls}
                                            onclick={mk_locale_cb(loc.as_str())}
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
                                { for crate::app::prefs::themes_list().into_iter().map(|(id, label)| {
                                    let is_active = c.current_theme == id;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    html! {
                                        <button
                                            class={cls}
                                            onclick={mk_theme_cb(id)}
                                            disabled={is_active}
                                        >
                                            { label }
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

                            // §8 customization (A1/A2/A5/A8).
                            <h3>{ locale.tr_key("settings.number_format") }</h3>
                            <p class="muted small">{ locale.tr_key("settings.number_format_desc") }</p>
                            <div class="theme-picker">
                                { for ["compact", "full", "raw"].iter().map(|fmt| {
                                    let is_active = c.prefs.number_format == *fmt;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    let label = locale.tr_key(&format!("settings.number_format.{fmt}"));
                                    let fmt_owned = fmt.to_string();
                                    let cb = {
                                        let core = core_cell.clone();
                                        let pending = pending.clone();
                                        let bump = bump.clone();
                                        let fmt_clone = fmt_owned.clone();
                                        Callback::from(move |_: MouseEvent| {
                                            if let Some(c) = core.borrow_mut().as_mut() {
                                                c.prefs.number_format = fmt_clone.clone();
                                                save_prefs(&c.prefs);
                                            }
                                            crate::freenet::actions::settings::save_settings_once(
                                                core.clone(), pending.clone(), bump.clone(),
                                                None, None, None, None, None, None, None,
                                            );
                                            bump.set(now_ms());
                                        })
                                    };
                                    html! {
                                        <button class={cls} onclick={cb} disabled={is_active}>{ label }</button>
                                    }
                                }) }
                            </div>

                            <h3>{ locale.tr_key("settings.font_scale") }</h3>
                            <p class="muted small">{ locale.tr_key("settings.font_scale_desc") }</p>
                            <div class="theme-picker">
                                { for [80u8, 100, 120, 140].iter().map(|pct| {
                                    let is_active = c.prefs.font_scale == *pct;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    let pct_v = *pct;
                                    let cb = {
                                        let core = core_cell.clone();
                                        let pending = pending.clone();
                                        let bump = bump.clone();
                                        Callback::from(move |_: MouseEvent| {
                                            if let Some(c) = core.borrow_mut().as_mut() {
                                                c.prefs.font_scale = pct_v;
                                                crate::app::prefs::apply_font_scale(pct_v);
                                                save_prefs(&c.prefs);
                                            }
                                            crate::freenet::actions::settings::save_settings_once(
                                                core.clone(), pending.clone(), bump.clone(),
                                                None, None, None, None, None, None, None,
                                            );
                                            bump.set(now_ms());
                                        })
                                    };
                                    html! {
                                        <button class={cls} onclick={cb} disabled={is_active}>{ format!("{pct_v}%") }</button>
                                    }
                                }) }
                            </div>

                            <h3>{ locale.tr_key("settings.spoiler_safe") }</h3>
                            <label class="setting-toggle">
                                <input
                                    type="checkbox"
                                    checked={c.prefs.spoiler_safe}
                                    onclick={{
                                        let core = core_cell.clone();
                                        let pending = pending.clone();
                                        let bump = bump.clone();
                                        Callback::from(move |_: MouseEvent| {
                                            if let Some(c) = core.borrow_mut().as_mut() {
                                                c.prefs.spoiler_safe = !c.prefs.spoiler_safe;
                                                save_prefs(&c.prefs);
                                            }
                                            crate::freenet::actions::settings::save_settings_once(
                                                core.clone(), pending.clone(), bump.clone(),
                                                None, None, None, None, None, None, None,
                                            );
                                            bump.set(now_ms());
                                        })
                                    }}
                                />
                                { locale.tr_key("settings.spoiler_safe_desc") }
                            </label>

                            <h3>{ locale.tr_key("settings.confirm_destructive") }</h3>
                            <label class="setting-toggle">
                                <input
                                    type="checkbox"
                                    checked={c.prefs.confirm_destructive}
                                    onclick={{
                                        let core = core_cell.clone();
                                        let pending = pending.clone();
                                        let bump = bump.clone();
                                        Callback::from(move |_: MouseEvent| {
                                            if let Some(c) = core.borrow_mut().as_mut() {
                                                c.prefs.confirm_destructive = !c.prefs.confirm_destructive;
                                                save_prefs(&c.prefs);
                                            }
                                            crate::freenet::actions::settings::save_settings_once(
                                                core.clone(), pending.clone(), bump.clone(),
                                                None, None, None, None, None, None, None,
                                            );
                                            bump.set(now_ms());
                                        })
                                    }}
                                />
                                { locale.tr_key("settings.confirm_destructive_desc") }
                            </label>

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
                                    // §8 C4: 3-variant pubkey display.
                                    // Hidden → masked sentinel; Short →
                                    // short_id label only; Full → legacy
                                    // long-form below.
                                    match c.prefs.pubkey_display {
                                        crate::app::prefs::PUBKEY_DISPLAY_HIDDEN =>
                                            locale.tr(MessageId::TermPubkeyHidden).to_string(),
                                        _ => pubkey_text.clone(),
                                    }
                                }
                            </p>
                            {
                                if c.prefs.pubkey_display == crate::app::prefs::PUBKEY_DISPLAY_FULL {
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

                            <details class="settings-customization">
                                <summary>{ locale.tr_key("settings.customization") }</summary>
                                <p class="muted small">
                                    { locale.tr_key("settings.customization_desc") }
                                </p>

                                <label class="setting-toggle">
                                    <input type="checkbox"
                                        checked={c.prefs.hide_gold}
                                        onclick={mk_toggle_cb(ToggleField::HideGold)} />
                                    { locale.tr_key("settings.hide_gold") }
                                </label>
                                <label class="setting-toggle">
                                    <input type="checkbox"
                                        checked={c.prefs.hide_boss_damage}
                                        onclick={mk_toggle_cb(ToggleField::HideBossDamage)} />
                                    { locale.tr_key("settings.hide_boss_damage") }
                                </label>
                                <label class="setting-toggle">
                                    <input type="checkbox"
                                        checked={c.prefs.reduced_motion}
                                        onclick={mk_toggle_cb(ToggleField::ReducedMotion)} />
                                    { locale.tr_key("settings.reduced_motion") }
                                </label>
                                <label class="setting-toggle">
                                    <input type="checkbox"
                                        checked={c.prefs.reduced_flash}
                                        onclick={mk_toggle_cb(ToggleField::ReducedFlash)} />
                                    { locale.tr_key("settings.reduced_flash") }
                                </label>
                                <label class="setting-toggle">
                                    <input type="checkbox"
                                        checked={c.prefs.overlay_mode}
                                        onclick={mk_toggle_cb(ToggleField::OverlayMode)} />
                                    { locale.tr_key("settings.overlay_mode") }
                                </label>
                                <label class="setting-toggle">
                                    <input type="checkbox"
                                        checked={c.prefs.keyboard_shortcuts}
                                        onclick={mk_toggle_cb(ToggleField::KeyboardShortcuts)} />
                                    { locale.tr_key("settings.keyboard_shortcuts") }
                                </label>
                                <p class="muted small">{ locale.tr_key("settings.keyboard_shortcuts_help") }</p>

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.theme_schedule") }</h3>
                                <p class="muted small">{ locale.tr_key("settings.theme_schedule_desc") }</p>
                                <label class="setting-text">
                                    <span>{ locale.tr_key("settings.theme_schedule.day") }</span>
                                    <select onchange={
                                        let core = core_cell.clone();
                                        let bump = bump.clone();
                                        Callback::from(move |e: Event| {
                                            let val = e.target_dyn_into::<web_sys::HtmlSelectElement>()
                                                .map(|s| s.value()).unwrap_or_default();
                                            if let Some(c) = core.borrow_mut().as_mut() {
                                                c.prefs.theme_schedule_day = val;
                                                save_prefs(&c.prefs);
                                            }
                                            bump.set(now_ms());
                                        })
                                    }>
                                        <option value="" selected={c.prefs.theme_schedule_day.is_empty()}>
                                            { locale.tr_key("settings.theme_schedule.unset") }
                                        </option>
                                        { for crate::app::theme_loader::available_codes().iter().map(|code| {
                                            html! {
                                                <option value={code.to_string()}
                                                        selected={c.prefs.theme_schedule_day == *code}>
                                                    { code.to_string() }
                                                </option>
                                            }
                                        }) }
                                    </select>
                                </label>
                                <label class="setting-text">
                                    <span>{ locale.tr_key("settings.theme_schedule.night") }</span>
                                    <select onchange={
                                        let core = core_cell.clone();
                                        let bump = bump.clone();
                                        Callback::from(move |e: Event| {
                                            let val = e.target_dyn_into::<web_sys::HtmlSelectElement>()
                                                .map(|s| s.value()).unwrap_or_default();
                                            if let Some(c) = core.borrow_mut().as_mut() {
                                                c.prefs.theme_schedule_night = val;
                                                save_prefs(&c.prefs);
                                            }
                                            bump.set(now_ms());
                                        })
                                    }>
                                        <option value="" selected={c.prefs.theme_schedule_night.is_empty()}>
                                            { locale.tr_key("settings.theme_schedule.unset") }
                                        </option>
                                        { for crate::app::theme_loader::available_codes().iter().map(|code| {
                                            html! {
                                                <option value={code.to_string()}
                                                        selected={c.prefs.theme_schedule_night == *code}>
                                                    { code.to_string() }
                                                </option>
                                            }
                                        }) }
                                    </select>
                                </label>
                                <label class="setting-text">
                                    <span>{ locale.tr_key("settings.theme_schedule.night_hour") }</span>
                                    <input type="number" min="0" max="23"
                                        value={
                                            if c.prefs.theme_night_hour == crate::app::prefs::THEME_NIGHT_HOUR_DISABLED {
                                                String::new()
                                            } else {
                                                c.prefs.theme_night_hour.to_string()
                                            }
                                        }
                                        oninput={
                                            let core = core_cell.clone();
                                            let bump = bump.clone();
                                            Callback::from(move |e: InputEvent| {
                                                let val = e.target_dyn_into::<web_sys::HtmlInputElement>()
                                                    .map(|i| i.value()).unwrap_or_default();
                                                if let Some(c) = core.borrow_mut().as_mut() {
                                                    if val.is_empty() {
                                                        c.prefs.theme_night_hour =
                                                            crate::app::prefs::THEME_NIGHT_HOUR_DISABLED;
                                                    } else if let Ok(h) = val.parse::<u8>() {
                                                        c.prefs.theme_night_hour = h.min(23);
                                                    }
                                                    save_prefs(&c.prefs);
                                                }
                                                bump.set(now_ms());
                                            })
                                        }
                                    />
                                </label>

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.stash_density") }</h3>
                                <div class="theme-picker">
                                    { for [0u8, 1, 2].iter().map(|d| {
                                        let is_active = c.prefs.stash_density == *d;
                                        let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                        let label = locale.tr_key(match *d {
                                            1 => "settings.stash_density.compact",
                                            2 => "settings.stash_density.tight",
                                            _ => "settings.stash_density.comfortable",
                                        });
                                        let cb = {
                                            let core = core_cell.clone();
                                            let bump = bump.clone();
                                            let val = *d;
                                            Callback::from(move |_: MouseEvent| {
                                                if let Some(c) = core.borrow_mut().as_mut() {
                                                    c.prefs.stash_density = val;
                                                    save_prefs(&c.prefs);
                                                    crate::app::prefs::apply_visual_prefs(&c.prefs);
                                                }
                                                bump.set(now_ms());
                                            })
                                        };
                                        html! {
                                            <button class={cls} onclick={cb} disabled={is_active}>
                                                { label }
                                            </button>
                                        }
                                    }) }
                                </div>

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.motto") }</h3>
                                <p class="muted small">{ locale.tr_key("settings.motto_desc") }</p>
                                <label class="setting-text">
                                    <input type="text" maxlength="64"
                                        value={c.prefs.motto.clone()}
                                        oninput={
                                            // Local save on every keystroke; debounce
                                            // the publish RPC to `onchange` (fires on
                                            // blur / Enter) so we don't spam the
                                            // delegate during typing.
                                            let core = core_cell.clone();
                                            let bump = bump.clone();
                                            Callback::from(move |e: InputEvent| {
                                                let val = e.target_dyn_into::<web_sys::HtmlInputElement>()
                                                    .map(|i| i.value())
                                                    .unwrap_or_default();
                                                if let Some(c) = core.borrow_mut().as_mut() {
                                                    c.prefs.motto = val.chars().take(64).collect();
                                                    save_prefs(&c.prefs);
                                                }
                                                bump.set(now_ms());
                                            })
                                        }
                                        onchange={
                                            let core = core_cell.clone();
                                            let pending = pending.clone();
                                            let bump = bump.clone();
                                            Callback::from(move |_: Event| {
                                                // Publish the current local motto +
                                                // accent + frame so the next presence
                                                // heartbeat picks it up.
                                                let (motto, accent, frame) = {
                                                    let g = core.borrow();
                                                    match g.as_ref() {
                                                        Some(c) => (
                                                            c.prefs.motto.clone(),
                                                            c.prefs.row_accent,
                                                            c.inventory.routine.public_frame,
                                                        ),
                                                        None => (String::new(), 0u8, 0u8),
                                                    }
                                                };
                                                crate::freenet::actions::activity::set_public_cosmetics_once(
                                                    core.clone(), pending.clone(), bump.clone(),
                                                    motto, accent, frame,
                                                );
                                            })
                                        }
                                        placeholder={locale.tr_key("settings.motto_placeholder").to_string()}
                                    />
                                </label>

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.hero_skin") }</h3>
                                <p class="muted small">{ locale.tr_key("settings.hero_skin_desc") }</p>
                                <div class="theme-picker">
                                    { for ["", "😎", "🤠", "🥷", "🧛", "🦊", "🤖", "🧝"].iter().map(|skin| {
                                        let is_active = c.prefs.hero_skin == *skin;
                                        let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                        let label = if skin.is_empty() {
                                            locale.tr_key("settings.hero_skin.default").to_string()
                                        } else {
                                            skin.to_string()
                                        };
                                        let cb = {
                                            let core = core_cell.clone();
                                            let bump = bump.clone();
                                            let val = skin.to_string();
                                            Callback::from(move |_: MouseEvent| {
                                                if let Some(c) = core.borrow_mut().as_mut() {
                                                    c.prefs.hero_skin = val.clone();
                                                    save_prefs(&c.prefs);
                                                }
                                                bump.set(now_ms());
                                            })
                                        };
                                        html! {
                                            <button class={cls} onclick={cb} disabled={is_active}>{ label }</button>
                                        }
                                    }) }
                                </div>

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.public_cosmetics") }</h3>
                                <p class="muted small">{ locale.tr_key("settings.public_cosmetics_desc") }</p>
                                <div class="action-row">
                                    <button onclick={
                                        let core = core_cell.clone();
                                        let pending = pending.clone();
                                        let bump = bump.clone();
                                        Callback::from(move |_: MouseEvent| {
                                            // Re-read prefs at click time so user-edited
                                            // motto/accent take effect even after typing.
                                            let g = core.borrow();
                                            let Some(c) = g.as_ref() else { return };
                                            let motto = c.prefs.motto.clone();
                                            let accent = c.prefs.row_accent;
                                            // No separate "frame" pref yet; piggyback on accent
                                            // for now so player has a single dial. Future PR can
                                            // add a dedicated `publish_frame` UserPrefs field.
                                            let frame = 0u8;
                                            drop(g);
                                            crate::freenet::actions::activity::set_public_cosmetics_once(
                                                core.clone(), pending.clone(), bump.clone(),
                                                motto, accent, frame,
                                            )
                                        })
                                    }>{ locale.tr_key("settings.publish_cosmetics_btn") }</button>
                                </div>

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.row_accent") }</h3>
                                <p class="muted small">{ locale.tr_key("settings.row_accent_desc") }</p>
                                <div class="accent-picker">
                                    { for [0u8, 1, 2, 3, 4, 5, 6].iter().map(|a| {
                                        let is_active = c.prefs.row_accent == *a;
                                        let cls = if is_active { "accent-swatch active" } else { "accent-swatch" };
                                        // Match the 6 leaderboard-row hues from feed.rs
                                        // exactly so the swatch reads as a true preview
                                        // of (a) the left-border ribbon and (b) the
                                        // name/motto text colour. Background stays the
                                        // panel default so the preview works on both
                                        // light and dark themes.
                                        let hue = match a {
                                            1 => "#e57373",
                                            2 => "#64b5f6",
                                            3 => "#81c784",
                                            4 => "#ffd54f",
                                            5 => "#9575cd",
                                            6 => "#ff8a65",
                                            _ => "",
                                        };
                                        let style = if *a == 0 {
                                            "border: 1px dashed currentColor;".to_string()
                                        } else {
                                            format!(
                                                "box-shadow: inset 4px 0 0 0 {hue}; color: {hue}; font-weight: 600;"
                                            )
                                        };
                                        let cb = {
                                            let core = core_cell.clone();
                                            let pending = pending.clone();
                                            let bump = bump.clone();
                                            let val = *a;
                                            Callback::from(move |_: MouseEvent| {
                                                // Update local pref, then fire the
                                                // publish RPC so the next presence
                                                // heartbeat carries the new accent.
                                                // Read motto + current frame at click
                                                // time so we don't lock-in stale values
                                                // captured at render.
                                                let (motto, frame) = {
                                                    let g = core.borrow();
                                                    match g.as_ref() {
                                                        Some(c) => (
                                                            c.prefs.motto.clone(),
                                                            c.inventory.routine.public_frame,
                                                        ),
                                                        None => (String::new(), 0u8),
                                                    }
                                                };
                                                if let Some(c) = core.borrow_mut().as_mut() {
                                                    c.prefs.row_accent = val;
                                                    save_prefs(&c.prefs);
                                                }
                                                crate::freenet::actions::activity::set_public_cosmetics_once(
                                                    core.clone(), pending.clone(), bump.clone(),
                                                    motto, val, frame,
                                                );
                                                bump.set(now_ms());
                                            })
                                        };
                                        let title = if *a == 0 {
                                            locale.tr_key("settings.row_accent.none").to_string()
                                        } else {
                                            format!("accent {a}")
                                        };
                                        let label = if *a == 0 {
                                            locale.tr_key("settings.row_accent.none").to_string()
                                        } else {
                                            locale.tr_key("settings.row_accent.preview").to_string()
                                        };
                                        html! {
                                            <button class={cls} onclick={cb}
                                                    disabled={is_active}
                                                    style={style}
                                                    title={title}>
                                                { label }
                                            </button>
                                        }
                                    }) }
                                </div>

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.hidden_tabs") }</h3>
                                <p class="muted small">{ locale.tr_key("settings.hidden_tabs_desc") }</p>
                                { for [
                                    (Tab::WorldMap, "tab.world_map"),
                                    (Tab::Shop, "tab.shop"),
                                    (Tab::Guilds, "tab.guilds"),
                                    (Tab::Achievements, "tab.achievements"),
                                    (Tab::Mastery, "tab.mastery"),
                                ].iter().map(|(tab, key)| {
                                    let idx = *tab as u8;
                                    let bit = 1u32 << idx;
                                    let hidden = (c.prefs.hidden_tabs & bit) != 0;
                                    let cb = {
                                        let core = core_cell.clone();
                                        let bump = bump.clone();
                                        Callback::from(move |_: MouseEvent| {
                                            if let Some(c) = core.borrow_mut().as_mut() {
                                                c.prefs.hidden_tabs ^= bit;
                                                save_prefs(&c.prefs);
                                            }
                                            bump.set(now_ms());
                                        })
                                    };
                                    html! {
                                        <label class="setting-toggle">
                                            <input type="checkbox" checked={hidden} onclick={cb} />
                                            { format!(" hide: {}", locale.tr_key(key)) }
                                        </label>
                                    }
                                }) }

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.numerical_assists") }</h3>
                                <p class="muted small">{ locale.tr_key("settings.numerical_assists_desc") }</p>
                                { for [
                                    ("settings.assist.enemy_hp_pct", 0u32),
                                    ("settings.assist.hide_hero_hp_numbers", 1u32),
                                    ("settings.assist.hide_damage_numbers", 2u32),
                                ].iter().map(|(key, bit_idx)| {
                                    let bit = 1u32 << bit_idx;
                                    let on = (c.prefs.numerical_assists & bit) != 0;
                                    let cb = {
                                        let core = core_cell.clone();
                                        let bump = bump.clone();
                                        Callback::from(move |_: MouseEvent| {
                                            if let Some(c) = core.borrow_mut().as_mut() {
                                                c.prefs.numerical_assists ^= bit;
                                                save_prefs(&c.prefs);
                                            }
                                            bump.set(now_ms());
                                        })
                                    };
                                    html! {
                                        <label class="setting-toggle">
                                            <input type="checkbox" checked={on} onclick={cb} />
                                            { locale.tr_key(key) }
                                        </label>
                                    }
                                }) }

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.notifications") }</h3>
                                <p class="muted small">{ locale.tr_key("settings.notifications_desc") }</p>
                                { for [
                                    (crate::app::types::ToastKind::Achievement, "settings.notif.achievement"),
                                    (crate::app::types::ToastKind::LevelUp, "settings.notif.level_up"),
                                    (crate::app::types::ToastKind::FormChange, "settings.notif.form_change"),
                                    (crate::app::types::ToastKind::PotionIdle, "settings.notif.potion_idle"),
                                ].iter().map(|(kind, key)| {
                                    let bit = kind.bit();
                                    let on = (c.prefs.toast_filter & bit) != 0;
                                    let cb = {
                                        let core = core_cell.clone();
                                        let bump = bump.clone();
                                        Callback::from(move |_: MouseEvent| {
                                            if let Some(c) = core.borrow_mut().as_mut() {
                                                c.prefs.toast_filter ^= bit;
                                                save_prefs(&c.prefs);
                                            }
                                            bump.set(now_ms());
                                        })
                                    };
                                    html! {
                                        <label class="setting-toggle">
                                            <input type="checkbox" checked={on} onclick={cb} />
                                            { locale.tr_key(key) }
                                        </label>
                                    }
                                }) }
                            </details>

                            <details class="settings-advanced">
                                <summary>{ locale.tr(MessageId::SettingsAdvanced) }</summary>
                                <p class="muted small">
                                    { locale.tr(MessageId::SettingsAdvancedDesc) }
                                </p>

                                <h3 class="advanced-subhead">{ locale.tr_key("settings.pubkey_display") }</h3>
                                <p class="muted small">{ locale.tr_key("settings.pubkey_display_desc") }</p>
                                <div class="theme-picker">
                                    { for [
                                        (crate::app::prefs::PUBKEY_DISPLAY_FULL, "settings.pubkey_display.full"),
                                        (crate::app::prefs::PUBKEY_DISPLAY_SHORT, "settings.pubkey_display.short"),
                                        (crate::app::prefs::PUBKEY_DISPLAY_HIDDEN, "settings.pubkey_display.hidden"),
                                    ].iter().map(|(val, key)| {
                                        let is_active = c.prefs.pubkey_display == *val;
                                        let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                        let cb = {
                                            let core = core_cell.clone();
                                            let bump = bump.clone();
                                            let v = *val;
                                            Callback::from(move |_: MouseEvent| {
                                                if let Some(c) = core.borrow_mut().as_mut() {
                                                    c.prefs.pubkey_display = v;
                                                    // Keep legacy `hide_pubkey` in sync so old code
                                                    // paths (and the leaderboard short-id logic)
                                                    // that still read it don't desync.
                                                    c.prefs.hide_pubkey =
                                                        v == crate::app::prefs::PUBKEY_DISPLAY_HIDDEN;
                                                    save_prefs(&c.prefs);
                                                }
                                                bump.set(now_ms());
                                            })
                                        };
                                        html! {
                                            <button class={cls} onclick={cb} disabled={is_active}>
                                                { locale.tr_key(key) }
                                            </button>
                                        }
                                    }) }
                                </div>

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
/// §C: Merchant exchange panel — single home for essence→gold and
/// wheat→gold currency conversions. Hidden when neither side has
/// anything to spend, so it doesn't sit as a dead stub for fresh
/// players.
fn render_exchange_panel(
    c: &Core,
    locale: Locale,
    on_convert_essence: Callback<MouseEvent>,
    on_sell_wheat: Callback<MouseEvent>,
) -> Html {
    let inv = &c.inventory;
    let can_convert_essence = inv.essence > 0;
    let can_sell_wheat = inv.wheat >= shared::WHEAT_PER_GOLD;
    if !can_convert_essence && !can_sell_wheat {
        return html! {};
    }
    html! {
        <section class="panel exchange">
            <h2>{ locale.tr_key("panel.exchange") }</h2>
            <p class="muted small">{ locale.tr_key("panel.exchange_desc") }</p>
            <div class="action-row">
                {
                    if can_convert_essence {
                        let preview = format!(
                            " ({} → {}g)",
                            format_si(inv.essence),
                            format_si(inv.essence.saturating_mul(shared::ESSENCE_TO_GOLD_RATE)),
                        );
                        html! {
                            <button
                                onclick={on_convert_essence}
                                title={locale.tr_key("shop.convert_essence_tooltip")
                                    .replace("{rate}", &shared::ESSENCE_TO_GOLD_RATE.to_string())}
                            >
                                { locale.tr_key("btn.convert_essence") }{ preview }
                            </button>
                        }
                    } else { html! {} }
                }
                {
                    if can_sell_wheat {
                        let preview = format!(
                            " ({} → {}g)",
                            format_si(inv.wheat),
                            format_si(inv.wheat / shared::WHEAT_PER_GOLD),
                        );
                        html! {
                            <button
                                onclick={on_sell_wheat}
                                title={locale.fmt_sell_wheat_tooltip(shared::WHEAT_PER_GOLD as u64)}
                            >
                                { locale.tr(MessageId::BtnSellAllWheat) }{ preview }
                            </button>
                        }
                    } else { html! {} }
                }
            </div>
        </section>
    }
}

/// Offline-cap preset buttons. Lifted out of the inline html!
/// because the conditional `presets` array needs a `let` binding
/// that the yew macro doesn't accept inside a `{ ... }`
/// interpolation. Returns a Vec<Html> for splat into the
/// `<div class="action-row">` parent.
fn render_offline_cap_buttons(
    inv: &shared::Inventory,
    locale: Locale,
    core_cell: CoreCell,
    pending: PendingCell,
    bump: yew::UseStateSetter<u64>,
) -> Vec<Html> {
    let lhf = inv.tokens.long_haul();
    let presets: &[u8] = if lhf {
        &[0, 1, 2, 4, 8, 12, 24, 48, 72, 168]
    } else {
        &[0, 1, 2, 4, 8, 12, 24]
    };
    presets
        .iter()
        .map(|h| {
            let is_active = inv.routine.offline_cap_hours == *h;
            let label = if *h == 0 {
                locale.tr_key("routine.offline_cap.default").to_string()
            } else {
                format!("{h}h")
            };
            let cb = {
                let core = core_cell.clone();
                let pending = pending.clone();
                let bump = bump.clone();
                let val = *h;
                Callback::from(move |_: MouseEvent| {
                    crate::freenet::actions::activity::set_routine_offline_cap_hours_once(
                        core.clone(),
                        pending.clone(),
                        bump.clone(),
                        val,
                    )
                })
            };
            html! {
                <button onclick={cb} class={if is_active { "primary" } else { "" }}>
                    { label }
                </button>
            }
        })
        .collect()
}

/// §P3 daily check-in row rendered under the Hero name. Streak
/// counter + claim button; button disabled (with "Claimed today"
/// label) when the player has already claimed for the current
/// UTC day.
fn render_daily_checkin(
    c: &Core,
    locale: Locale,
    core_cell: CoreCell,
    pending: PendingCell,
    bump: yew::UseStateSetter<u64>,
) -> Html {
    let now = crate::now_ms();
    let today = now / 86_400_000;
    let claimed_today = c.inventory.routine.last_checkin_day == today;
    // First-time players (streak = 0 + already-claimed somehow,
    // shouldn't happen but cheap to guard) get the same muted line
    // as post-claim. The promient button only renders when a claim
    // is actually available.
    if claimed_today {
        return html! {
            <p class="daily-checkin daily-checkin-muted muted small">
                { "🔥 " }
                { locale.tr_key("daily.streak_label")
                    .replace("{n}", &c.inventory.routine.streak_days.to_string()) }
                { " · " }
                { locale.tr_key("daily.claimed") }
            </p>
        };
    }
    let cb = Callback::from(move |_: MouseEvent| {
        crate::freenet::actions::activity::claim_daily_checkin_once(
            core_cell.clone(), pending.clone(), bump.clone(),
        )
    });
    html! {
        <p class="daily-checkin muted small">
            { locale.tr_key("daily.streak_label")
                .replace("{n}", &c.inventory.routine.streak_days.to_string()) }
            { " " }
            <button onclick={cb} title={locale.tr_key("daily.claim_tooltip")}>
                { locale.tr_key("daily.claim") }
            </button>
        </p>
    }
}

/// Render a panel with a collapse toggle in its `<h2>` header
/// (§8 B2). The toggled state is persisted in
/// `c.prefs.collapsed_panels` (one bit per `PANEL_BIT_*` constant)
/// and saved to localStorage, so the player's choice survives
/// reloads. Body is dropped from the DOM when collapsed —
/// rendering empty would still cost layout.
fn render_collapsible_panel(
    panel_class: &str,
    panel_bit: u8,
    header_text: &str,
    body: Html,
    bits: u64,
    core_cell: CoreCell,
    bump: yew::UseStateSetter<u64>,
) -> Html {
    let collapsed = crate::app::prefs::is_panel_collapsed(bits, panel_bit);
    let toggle_cb = {
        let core = core_cell.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(c) = core.borrow_mut().as_mut() {
                c.prefs.collapsed_panels ^= 1u64 << panel_bit;
                save_prefs(&c.prefs);
            }
            bump.set(now_ms());
        })
    };
    html! {
        <section class={classes!(panel_class.to_string(), if collapsed { "collapsed" } else { "" })}>
            <h2>
                <button class="panel-collapse-toggle" onclick={toggle_cb}>
                    { if collapsed { "▸" } else { "▾" } }
                </button>
                { " " }{ header_text }
            </h2>
            { if collapsed { html! {} } else { body } }
        </section>
    }
}

/// Toolbar above the stash: slot filter + sort selector (§8 B1).
/// Persists prefs to localStorage. No delegate-side state needed —
/// this is purely a frontend filter on the existing `unequipped`
/// list.
fn render_stash_toolbar(
    c: &Core,
    locale: Locale,
    core_cell: CoreCell,
    bump: yew::UseStateSetter<u64>,
) -> Html {
    let cur_filter = c.prefs.stash_filter;
    let cur_sort = c.prefs.stash_sort;

    let mk_filter_cb = |slot: u8| {
        let core = core_cell.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(c) = core.borrow_mut().as_mut() {
                c.prefs.stash_filter = slot;
                crate::app::prefs::save_prefs(&c.prefs);
            }
            bump.set(now_ms());
        })
    };
    let mk_sort_cb = |mode: u8| {
        let core = core_cell.clone();
        let bump = bump.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(c) = core.borrow_mut().as_mut() {
                c.prefs.stash_sort = mode;
                crate::app::prefs::save_prefs(&c.prefs);
            }
            bump.set(now_ms());
        })
    };

    let filter_btn = |slot_label: String, slot_value: u8| -> Html {
        let active = cur_filter == slot_value;
        let cls = if active { "primary" } else { "" };
        html! {
            <button class={cls} onclick={mk_filter_cb(slot_value)}>{ slot_label }</button>
        }
    };
    let sort_btn = |label_key: &str, mode: u8| -> Html {
        let active = cur_sort == mode;
        let cls = if active { "primary" } else { "" };
        let label = locale.tr_key(label_key).to_string();
        html! {
            <button class={cls} onclick={mk_sort_cb(mode)}>{ label }</button>
        }
    };

    html! {
        <div class="stash-toolbar">
            <span class="muted small">{ locale.tr_key("stash.filter_label") }</span>
            { filter_btn(locale.tr_key("stash.filter_all").to_string(),
                         crate::app::prefs::STASH_FILTER_NONE) }
            { for (0..shared::SLOT_COUNT).map(|s| {
                filter_btn(
                    i18n_shared::slot_name(locale, s).to_string(),
                    s as u8,
                )
            }) }
            <span class="muted small" style="margin-left:1em">{ locale.tr_key("stash.sort_label") }</span>
            { sort_btn("stash.sort_catalog", 0) }
            { sort_btn("stash.sort_tier", 1) }
            { sort_btn("stash.sort_score", 2) }
        </div>
    }
}

/// plus the Ascend button. Hidden entirely when the player has no
/// stars *and* no purchased nodes — a fresh account doesn't need a
/// prestige UI cluttering Settings yet.
fn render_legacy_panel<F, G>(
    c: &Core,
    mk_buy_cb: &F,
    mk_buy_bulk_cb: &G,
    on_ascend: Callback<MouseEvent>,
) -> Html
where
    F: Fn(u8) -> Callback<MouseEvent>,
    G: Fn(u8, u32) -> Callback<MouseEvent>,
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
                        let cb_x10 = mk_buy_bulk_cb(node.id(), 10);
                        let cb_max = mk_buy_bulk_cb(node.id(), 0);
                        let mult_label = format!(
                            "×{}.{:02}",
                            mult_bp / 10_000,
                            (mult_bp % 10_000) / 100,
                        );
                        let name_key = format!("legacy_node_name.{}", node.key());
                        let desc_key = format!("legacy_node_desc.{}", node.key());
                        let name_tr = locale.tr_key(&name_key);
                        let desc_tr = locale.tr_key(&desc_key);
                        let node_name: &str = if name_tr.starts_with('?') { node.name() } else { name_tr };
                        let node_desc: &str = if desc_tr.starts_with('?') { node.description() } else { desc_tr };
                        html! {
                            <tr title={node_desc}>
                                <td>
                                    <div>{ node_name }</div>
                                    <div class="muted small">{ node_desc }</div>
                                </td>
                                <td class="num">{ lvl }</td>
                                <td class="num">{ mult_label }</td>
                                <td class="num">{ format!("{}★", cost) }</td>
                                <td>
                                    <button onclick={cb} disabled={disabled}>
                                        { locale.tr(MessageId::BtnBuy) }
                                    </button>
                                    { " " }
                                    <button onclick={cb_x10} disabled={disabled}
                                            title={locale.tr_key("mastery.buy_x10_tooltip")}>
                                        { locale.tr_key("mastery.buy_x10") }
                                    </button>
                                    { " " }
                                    <button onclick={cb_max} disabled={disabled}
                                            title={locale.tr_key("mastery.buy_max_tooltip")}>
                                        { locale.tr_key("mastery.buy_max") }
                                    </button>
                                </td>
                            </tr>
                        }
                    }) }
                </tbody>
            </table>
            <div class="ascend-divider">
                <h3>{ locale.tr_key("legacy.ascend_section") }</h3>
                <p class="muted small">
                    { locale.tr(MessageId::LegacyAscendBlurb) }
                </p>
                <div class="action-row">
                    <button class="danger" onclick={on_ascend}>{ locale.tr(MessageId::BtnAscend) }</button>
                </div>
            </div>
        </section>
    }
}


/// Routine panel (B1). Per-Estate-tier headcount target. Shows
/// "Owned / Target" with +/- buttons; setting target = 0 turns
/// auto-hire off for that tier. Panel hides entirely until the
/// player has at least one worker — no point cluttering Settings
/// before Estate is on the radar.
fn render_routine_panel(
    c: &Core,
    locale: Locale,
    core_cell: CoreCell,
    pending: PendingCell,
    bump: yew::UseStateSetter<u64>,
) -> Html {
    let inv = &c.inventory;
    // Used to gate the whole panel on `any_worker > 0` so a fresh
    // player wasn't bombarded by routine knobs they had nothing to
    // configure. In practice this hid the panel from post-Ascend
    // players too — Ascend wipes Estate workers, but a player who
    // has earned a Legacy star (= the Mastery tab is open) is
    // categorically past the "fresh" stage and benefits from the
    // gear/consumable/skill/battle sub-sections immediately.
    // Estate sub-section still shows 0/0 rows but that's harmless.

    // ----------- Estate sub-section -----------
    let estate_table = html! {
        <table class="legacy-grid">
            <thead>
                <tr>
                    <th>{ locale.tr(MessageId::RoutineColTier) }</th>
                    <th class="num">{ locale.tr(MessageId::RoutineColCurrent) }</th>
                    <th class="num">{ locale.tr(MessageId::RoutineColTarget) }</th>
                    <th>{ "" }</th>
                </tr>
            </thead>
            <tbody>
                { for shared::ESTATE_TIERS.iter().map(|tier| {
                    let owned = inv.base.base.estate.workers_of(tier.id);
                    let target = inv.routine.target_for(tier.id).unwrap_or(0);
                    let cb_inc = {
                        let core = core_cell.clone();
                        let pending = pending.clone();
                        let bump = bump.clone();
                        let tid = tier.id;
                        let next_target = target.saturating_add(1);
                        Callback::from(move |_| {
                            crate::freenet::actions::activity::set_routine_estate_target_once(
                                core.clone(), pending.clone(), bump.clone(), tid, next_target,
                            )
                        })
                    };
                    let cb_dec = {
                        let core = core_cell.clone();
                        let pending = pending.clone();
                        let bump = bump.clone();
                        let tid = tier.id;
                        let next_target = target.saturating_sub(1);
                        Callback::from(move |_| {
                            crate::freenet::actions::activity::set_routine_estate_target_once(
                                core.clone(), pending.clone(), bump.clone(), tid, next_target,
                            )
                        })
                    };
                    let tier_key = format!("estate_tier_name.{}", tier.id);
                    let tier_name_tr = locale.tr_key(&tier_key);
                    let tier_label: &str = if tier_name_tr.starts_with('?') { tier.name } else { tier_name_tr };
                    html! {
                        <tr>
                            <td>{ tier_label }</td>
                            <td class="num">{ owned }</td>
                            <td class="num">{ target }</td>
                            <td>
                                <button onclick={cb_dec} disabled={target == 0}>{ "−" }</button>
                                { " " }
                                <button onclick={cb_inc}>{ "+" }</button>
                            </td>
                        </tr>
                    }
                }) }
            </tbody>
        </table>
    };

    // ----------- Gear sub-section -----------
    let cb_lock_current = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| {
            crate::freenet::actions::activity::lock_routine_gear_to_equipped_once(
                core.clone(), pending.clone(), bump.clone(),
            )
        })
    };
    let auto_equip_best = inv.routine.auto_equip_best_on_drop;
    let cb_toggle_auto_equip = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        let next = !auto_equip_best;
        Callback::from(move |_| {
            crate::freenet::actions::activity::set_routine_auto_equip_best_once(
                core.clone(), pending.clone(), bump.clone(), next,
            )
        })
    };
    let any_equipped = inv.equipped.iter().any(|s| s.is_some());
    let gear_rows = html! {
        <table class="legacy-grid">
            <thead>
                <tr>
                    <th>{ locale.tr_key("routine.col_slot") }</th>
                    <th class="num">{ locale.tr_key("routine.col_gear_target") }</th>
                    <th>{ "" }</th>
                </tr>
            </thead>
            <tbody>
                { for (0..shared::SLOT_COUNT).map(|s| {
                    let slot_idx = s as u8;
                    let target = inv.routine.gear_target_for(slot_idx).unwrap_or(0);
                    let cb = |new_tier: u8| {
                        let core = core_cell.clone();
                        let pending = pending.clone();
                        let bump = bump.clone();
                        Callback::from(move |_| {
                            crate::freenet::actions::activity::set_routine_gear_target_once(
                                core.clone(), pending.clone(), bump.clone(), slot_idx, new_tier,
                            )
                        })
                    };
                    html! {
                        <tr>
                            <td>{ i18n_shared::slot_name(locale, s) }</td>
                            <td class="num">
                                { if target == 0 { "—".to_string() } else { format!("T{target}") } }
                            </td>
                            <td>
                                <button onclick={cb(0)} disabled={target == 0}>{ "off" }</button>
                                { " " }
                                <button onclick={cb(1)}>{ "T1" }</button>
                                { " " }
                                <button onclick={cb(2)}>{ "T2" }</button>
                                { " " }
                                <button onclick={cb(3)}>{ "T3" }</button>
                            </td>
                        </tr>
                    }
                }) }
            </tbody>
        </table>
    };

    // ----------- Consumables sub-section -----------
    let mk_consumable_row = |kind: u8, label: &'static str| -> Html {
        let target = inv.routine.consumable_target_for(kind).unwrap_or(0);
        let cb_inc = {
            let core = core_cell.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            let next = target.saturating_add(5);
            Callback::from(move |_| {
                crate::freenet::actions::activity::set_routine_consumable_target_once(
                    core.clone(), pending.clone(), bump.clone(), kind, next,
                )
            })
        };
        let cb_dec = {
            let core = core_cell.clone();
            let pending = pending.clone();
            let bump = bump.clone();
            let next = target.saturating_sub(5);
            Callback::from(move |_| {
                crate::freenet::actions::activity::set_routine_consumable_target_once(
                    core.clone(), pending.clone(), bump.clone(), kind, next,
                )
            })
        };
        html! {
            <tr>
                <td>{ label }</td>
                <td class="num">{ target }</td>
                <td>
                    <button onclick={cb_dec} disabled={target == 0}>{ "−5" }</button>
                    { " " }
                    <button onclick={cb_inc}>{ "+5" }</button>
                </td>
            </tr>
        }
    };
    let consumable_rows = html! {
        <table class="legacy-grid">
            <thead>
                <tr>
                    <th>{ locale.tr_key("routine.col_consumable") }</th>
                    <th class="num">{ locale.tr_key("routine.col_keep") }</th>
                    <th>{ "" }</th>
                </tr>
            </thead>
            <tbody>
                { mk_consumable_row(shared::CONSUMABLE_POTION, locale.tr(MessageId::ItemPotion)) }
                { mk_consumable_row(shared::CONSUMABLE_FIREBALL, locale.tr(MessageId::ItemFireball)) }
            </tbody>
        </table>
    };

    // ----------- Skills + Battle-policy sub-section -----------
    let auto_skill = inv.routine.auto_skill_unlock;
    let cb_skill = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        let next = !auto_skill;
        Callback::from(move |_| {
            crate::freenet::actions::activity::set_routine_auto_skill_once(
                core.clone(), pending.clone(), bump.clone(), next,
            )
        })
    };
    let policy = inv.routine.battle_action_policy;
    let cb_policy_manual = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| {
            crate::freenet::actions::activity::set_routine_battle_policy_once(
                core.clone(), pending.clone(), bump.clone(),
                shared::BattleActionPolicy::Manual,
            )
        })
    };
    let cb_policy_auto = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| {
            crate::freenet::actions::activity::set_routine_battle_policy_once(
                core.clone(), pending.clone(), bump.clone(),
                shared::BattleActionPolicy::Auto {
                    potion_below_hp_pct: 40,
                    fireball_per_n_turns: 5,
                },
            )
        })
    };

    let auto_label = if auto_skill {
        locale.tr_key("routine.auto_skill_on")
    } else {
        locale.tr_key("routine.auto_skill_off")
    };
    let policy_label = match policy {
        shared::BattleActionPolicy::Manual => locale.tr_key("routine.battle_manual"),
        shared::BattleActionPolicy::Auto { .. } => locale.tr_key("routine.battle_auto"),
    };

    html! {
        <section class="panel routine">
            <h2>{ locale.tr(MessageId::PanelRoutine) }</h2>
            <p class="muted small">{ locale.tr(MessageId::RoutineDesc) }</p>

            <h3>{ locale.tr_key("routine.section_estate") }</h3>
            { estate_table }

            <h3>{ locale.tr_key("routine.section_gear") }</h3>
            <div class="action-row">
                <button
                    class={if auto_equip_best { "primary" } else { "" }}
                    onclick={cb_toggle_auto_equip}
                    title={locale.tr_key("routine.auto_equip_best_tooltip")}
                >
                    {
                        if auto_equip_best {
                            locale.tr_key("routine.auto_equip_best_on")
                        } else {
                            locale.tr_key("routine.auto_equip_best_off")
                        }
                    }
                </button>
                { " " }
                <button onclick={cb_lock_current} disabled={!any_equipped}
                        title={locale.tr_key("routine.lock_current_tooltip")}>
                    { locale.tr_key("routine.lock_current") }
                </button>
            </div>
            { gear_rows }

            <h3>{ locale.tr_key("routine.section_consumables") }</h3>
            { consumable_rows }

            <h3>{ locale.tr_key("routine.section_skills") }</h3>
            <p class="muted small">{ locale.tr_key("routine.auto_skill_desc") }</p>
            <div class="action-row">
                <button onclick={cb_skill}>{ auto_label }</button>
            </div>

            <h3>{ locale.tr_key("routine.section_battle") }</h3>
            <p class="muted small">{ locale.tr_key("routine.battle_desc") }</p>
            <div class="action-row">
                <button
                    class={if matches!(policy, shared::BattleActionPolicy::Manual) { "primary" } else { "" }}
                    onclick={cb_policy_manual}>{ locale.tr_key("routine.battle_manual") }</button>
                <button
                    class={if matches!(policy, shared::BattleActionPolicy::Auto { .. }) { "primary" } else { "" }}
                    onclick={cb_policy_auto}>{ locale.tr_key("routine.battle_auto") }</button>
                <span class="muted small">{ format!(" — {policy_label}") }</span>
            </div>

            <h3>{ locale.tr_key("routine.section_offline_cap") }</h3>
            <p class="muted small">{ locale.tr_key("routine.offline_cap_desc") }</p>
            <div class="action-row">
                { render_offline_cap_buttons(inv, locale, core_cell.clone(), pending.clone(), bump.clone()) }
            </div>
            {
                if inv.routine.offline_cap_hours > 4 {
                    html! {
                        <p class="muted small">
                            { locale.tr_key("routine.offline_cap.analytical_warning") }
                        </p>
                    }
                } else { html! {} }
            }

            <h3>{ locale.tr_key("routine.section_mission_cycle") }</h3>
            <p class="muted small">{ locale.tr_key("routine.mission_cycle_desc") }</p>
            <div class="action-row">
                { for [
                    (shared::MISSION_CYCLE_STATIC, "routine.cycle.static"),
                    (shared::MISSION_CYCLE_ROTATE, "routine.cycle.rotate"),
                    (shared::MISSION_CYCLE_BOSS_FIRST, "routine.cycle.boss_first"),
                ].iter().map(|(m, key)| {
                    let is_active = inv.routine.mission_cycle_mode == *m;
                    let cb = {
                        let core = core_cell.clone();
                        let pending = pending.clone();
                        let bump = bump.clone();
                        let mode = *m;
                        let areas = inv.routine.mission_cycle_areas.clone();
                        Callback::from(move |_: MouseEvent| {
                            crate::freenet::actions::activity::set_routine_mission_cycle_once(
                                core.clone(), pending.clone(), bump.clone(), mode, areas.clone(),
                            )
                        })
                    };
                    html! {
                        <button onclick={cb} class={if is_active { "primary" } else { "" }}>
                            { locale.tr_key(key) }
                        </button>
                    }
                }) }
            </div>
            <p class="muted small">
                { format!("{}: [{}]",
                    locale.tr_key("routine.cycle_areas_current"),
                    inv.routine.mission_cycle_areas
                        .iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ")
                ) }
            </p>
            <p class="muted small">{ locale.tr_key("routine.cycle_areas_hint") }</p>

            <h3>{ locale.tr_key("routine.section_combat_speed") }</h3>
            <p class="muted small">{ locale.tr_key("routine.combat_speed_desc") }</p>
            <div class="action-row">
                { for [
                    (0u32, "1×"),
                    (5_000u32, "0.5×"),
                    (10_000u32, "1×"),
                    (15_000u32, "1.5×"),
                    (20_000u32, "2×"),
                    (30_000u32, "3×"),
                ].iter().map(|(bp, label)| {
                    let is_active = inv.routine.combat_speed_bp == *bp;
                    let cb = {
                        let core = core_cell.clone();
                        let pending = pending.clone();
                        let bump = bump.clone();
                        let val = *bp;
                        Callback::from(move |_: MouseEvent| {
                            crate::freenet::actions::activity::set_routine_combat_speed_once(
                                core.clone(), pending.clone(), bump.clone(), val,
                            )
                        })
                    };
                    html! {
                        <button onclick={cb} class={if is_active { "primary" } else { "" }}>
                            { *label }
                        </button>
                    }
                }) }
            </div>
        </section>
    }
}

/// Insight panel (B5). Compact three-row spend tree gated by
/// owning at least 1 insight at any point (the balance might be
/// 0 after spending — `last_awarded_mission > 0` is the better
/// "have you ever seen this" test).
fn render_insight_panel(
    c: &Core,
    locale: Locale,
    core_cell: CoreCell,
    pending: PendingCell,
    bump: yew::UseStateSetter<u64>,
) -> Html {
    let inv = &c.inventory;
    if inv.insight.last_awarded_mission == 0 && inv.insight.balance == 0 {
        return html! {};
    }
    html! {
        <section class="panel legacy">
            <h2>{ locale.tr(MessageId::PanelInsight) }</h2>
            <p class="muted small">
                { format!("{} {} · {}", inv.insight.balance,
                    locale.tr(MessageId::ResInsight),
                    locale.tr(MessageId::InsightDesc)) }
            </p>
            <table class="legacy-grid">
                <thead>
                    <tr>
                        <th>{ locale.tr(MessageId::InsightColNode) }</th>
                        <th class="num">{ locale.tr(MessageId::InsightColLevel) }</th>
                        <th class="num">{ locale.tr(MessageId::InsightColNextCost) }</th>
                        <th>{ "" }</th>
                    </tr>
                </thead>
                <tbody>
                    { for shared::InsightNode::ALL.iter().map(|node| {
                        let lvl = inv.insight.node_level(*node);
                        let cost = node.next_cost(lvl);
                        let disabled = inv.insight.balance < cost;
                        let nid = node.id();
                        let cb = {
                            let core = core_cell.clone();
                            let pending = pending.clone();
                            let bump = bump.clone();
                            Callback::from(move |_| {
                                crate::freenet::actions::activity::buy_insight_node_once(
                                    core.clone(), pending.clone(), bump.clone(), nid,
                                )
                            })
                        };
                        let cb_x10 = {
                            let core = core_cell.clone();
                            let pending = pending.clone();
                            let bump = bump.clone();
                            Callback::from(move |_| {
                                crate::freenet::actions::activity::buy_insight_node_bulk_once(
                                    core.clone(), pending.clone(), bump.clone(), nid, 10,
                                )
                            })
                        };
                        let cb_max = {
                            let core = core_cell.clone();
                            let pending = pending.clone();
                            let bump = bump.clone();
                            Callback::from(move |_| {
                                crate::freenet::actions::activity::buy_insight_node_bulk_once(
                                    core.clone(), pending.clone(), bump.clone(), nid, 0,
                                )
                            })
                        };
                        let name_key = format!("insight_node_name.{}", node.key());
                        let desc_key = format!("insight_node_desc.{}", node.key());
                        let name_tr = locale.tr_key(&name_key);
                        let desc_tr = locale.tr_key(&desc_key);
                        let node_name: &str = if name_tr.starts_with('?') { node.name() } else { name_tr };
                        let node_desc: &str = if desc_tr.starts_with('?') { node.description() } else { desc_tr };
                        html! {
                            <tr title={node_desc}>
                                <td>
                                    <div>{ node_name }</div>
                                    <div class="muted small">{ node_desc }</div>
                                </td>
                                <td class="num">{ lvl }</td>
                                <td class="num">{ cost }</td>
                                <td>
                                    <button onclick={cb} disabled={disabled}>
                                        { locale.tr(MessageId::BtnBuy) }
                                    </button>
                                    { " " }
                                    <button onclick={cb_x10} disabled={disabled}
                                            title={locale.tr_key("mastery.buy_x10_tooltip")}>
                                        { locale.tr_key("mastery.buy_x10") }
                                    </button>
                                    { " " }
                                    <button onclick={cb_max} disabled={disabled}
                                            title={locale.tr_key("mastery.buy_max_tooltip")}>
                                        { locale.tr_key("mastery.buy_max") }
                                    </button>
                                </td>
                            </tr>
                        }
                    }) }
                </tbody>
            </table>
        </section>
    }
}

/// Boss-attack panel (C1). Renders the locked / unlocked state
/// reactively — the locked variant explains the gates so the
/// player knows what they're working toward.
fn render_boss_attack_panel(
    c: &Core,
    locale: Locale,
    core_cell: CoreCell,
    pending: PendingCell,
    bump: yew::UseStateSetter<u64>,
) -> Html {
    let inv = &c.inventory;
    // Pre-flight mirrors the delegate `boss_attack_unlocked`.
    let unlocked = inv.mission_count >= shared::BOSS_ATTACK_MIN_MISSIONS
        && shared::level_of(inv) >= shared::BOSS_ATTACK_MIN_LEVEL
        && inv.base.base.estate.workers.values().any(|n| *n > 0);
    // Hide pre-gate; surface once the player gets close (~half
    // missions or level).
    if inv.mission_count < shared::BOSS_ATTACK_MIN_MISSIONS / 2 {
        return html! {};
    }
    let cb = {
        let core = core_cell.clone();
        let pending = pending.clone();
        let bump = bump.clone();
        Callback::from(move |_| {
            crate::freenet::actions::activity::boss_attack_once(
                core.clone(), pending.clone(), bump.clone(),
            )
        })
    };
    let can_spend = inv.essence >= shared::BOSS_ATTACK_ESSENCE_COST;
    html! {
        <section class="panel boss">
            <h2>{ locale.tr(MessageId::PanelBossAttack) }</h2>
            <p class="muted small">{ locale.tr(MessageId::BossAttackDesc) }</p>
            {
                if !unlocked {
                    html! { <p class="muted small">{ locale.tr(MessageId::BossAttackLocked) }</p> }
                } else {
                    html! {
                        <div class="action-row">
                            <button class="primary" onclick={cb} disabled={!can_spend}
                                    title={locale.tr_key("boss.attack_tooltip")
                                        .replace("{cost}", &shared::BOSS_ATTACK_ESSENCE_COST.to_string())
                                        .replace("{dmg}", &shared::BOSS_ATTACK_DAMAGE.to_string())
                                        .replace("{min_lvl}", &shared::BOSS_ATTACK_MIN_LEVEL.to_string())
                                        .replace("{min_missions}", &shared::BOSS_ATTACK_MIN_MISSIONS.to_string())
                                    }>
                                { locale.tr(MessageId::BossAttackBtn) }
                            </button>
                        </div>
                    }
                }
            }
        </section>
    }
}

/// Tokens panel (C2). Hides until the player has earned at
/// least one token (`last_awarded_boss_damage > 0`) so the
/// section stays out of the way until the loop is reachable.
fn render_tokens_panel(
    c: &Core,
    locale: Locale,
    core_cell: CoreCell,
    pending: PendingCell,
    bump: yew::UseStateSetter<u64>,
) -> Html {
    let inv = &c.inventory;
    if inv.tokens.last_awarded_boss_damage == 0 && inv.tokens.balance == 0 {
        return html! {};
    }
    html! {
        <section class="panel legacy">
            <h2>{ locale.tr(MessageId::PanelTokens) }</h2>
            <p class="muted small">
                { format!("{} {} · {}", inv.tokens.balance,
                    locale.tr(MessageId::ResTokens),
                    locale.tr(MessageId::TokensDesc)) }
            </p>
            <table class="legacy-grid">
                <thead>
                    <tr>
                        <th>{ locale.tr(MessageId::TokenColPerk) }</th>
                        <th class="num">{ locale.tr(MessageId::TokenColPrice) }</th>
                        <th>{ "" }</th>
                    </tr>
                </thead>
                <tbody>
                    { for shared::TokenPerk::ALL.iter().map(|perk| {
                        let owned = inv.tokens.owns(*perk);
                        let price = perk.price();
                        let disabled = owned || inv.tokens.balance < price;
                        let pid = perk.id();
                        let cb = {
                            let core = core_cell.clone();
                            let pending = pending.clone();
                            let bump = bump.clone();
                            Callback::from(move |_| {
                                crate::freenet::actions::activity::buy_token_perk_once(
                                    core.clone(), pending.clone(), bump.clone(), pid,
                                )
                            })
                        };
                        let name_key = format!("token_perk_name.{}", perk.key());
                        let desc_key = format!("token_perk_desc.{}", perk.key());
                        let name_tr = locale.tr_key(&name_key);
                        let desc_tr = locale.tr_key(&desc_key);
                        let perk_name = if name_tr.starts_with('?') { perk.name() } else { name_tr };
                        let perk_desc = if desc_tr.starts_with('?') { perk.description() } else { desc_tr };
                        html! {
                            <tr title={perk_desc}>
                                <td>
                                    <div>{ perk_name }</div>
                                    <div class="muted small">{ perk_desc }</div>
                                </td>
                                <td class="num">{ price }</td>
                                <td>
                                    <button onclick={cb} disabled={disabled}>
                                        { if owned {
                                            locale.tr(MessageId::TermOwned)
                                        } else {
                                            locale.tr(MessageId::BtnUnlock)
                                        } }
                                    </button>
                                </td>
                            </tr>
                        }
                    }) }
                </tbody>
            </table>
        </section>
    }
}

/// Per-zone activities panel (A1). Lists activities whose
/// `area_id` matches the player's current area; switching area
/// on the World Map clears the active activity server-side via
/// `set_area`, so this list always reflects what's available
/// right now.
fn render_activities_panel(
    c: &Core,
    locale: Locale,
    core_cell: CoreCell,
    pending: PendingCell,
    bump: yew::UseStateSetter<u64>,
) -> Html {
    let inv = &c.inventory;
    let lvl = shared::level_of(inv);
    let area_activities: Vec<&shared::ActivityDef> =
        shared::activities_for_area(inv.current_area).collect();
    if area_activities.is_empty() {
        return html! {};
    }
    let active = inv.active_activity;
    html! {
        <section class="panel activities">
            <h2>{ locale.tr(MessageId::PanelActivities) }</h2>
            <p class="muted small">{ locale.tr(MessageId::ActivitiesDesc) }</p>
            <table class="legacy-grid">
                <tbody>
                    { for area_activities.iter().map(|a| {
                        let is_active = active == a.id;
                        let unlocked = lvl >= a.min_level;
                        let res_label = match a.produces {
                            shared::ActivityResource::Wheat => locale.tr(MessageId::EstateResWheat),
                            shared::ActivityResource::Gold => locale.tr(MessageId::EstateResGold),
                            shared::ActivityResource::Essence => locale.tr(MessageId::EstateResEssence),
                            shared::ActivityResource::Insight => locale.tr(MessageId::ResInsight),
                        };
                        let next_id = if is_active { shared::ACTIVITY_NONE } else { a.id };
                        let label = if is_active {
                            locale.tr(MessageId::ActivityStop)
                        } else {
                            locale.tr(MessageId::ActivityStart)
                        };
                        let cb = {
                            let core = core_cell.clone();
                            let pending = pending.clone();
                            let bump = bump.clone();
                            Callback::from(move |_| {
                                crate::freenet::actions::activity::set_activity_once(
                                    core.clone(), pending.clone(), bump.clone(), next_id,
                                )
                            })
                        };
                        let disabled = !unlocked && !is_active;
                        html! {
                            <tr>
                                <td>{ i18n_shared::activity_name(locale, a) }</td>
                                <td class="num">
                                    { format!("{}/s {}", a.yield_per_sec, res_label) }
                                </td>
                                <td>
                                    <button onclick={cb} disabled={disabled}
                                            class={if is_active { "primary" } else { "" }}>
                                        { label }
                                    </button>
                                </td>
                            </tr>
                        }
                    }) }
                </tbody>
            </table>
        </section>
    }
}

/// Wilds map (C3b). Builds the per-player procedural graph
/// from the inventory's `plot_seed` (so each player gets a
/// stable personal map across reloads but everyone's name list
/// differs). Hidden until the entrance area's `min_level` is
/// met so it doesn't clutter the early game. Same depth-row
/// layout as the main graph; activity / mission paths see the
/// dynamic AreaDef the same way they see static `AREAS`.
fn render_wilds_panel_body<F>(
    c: &Core,
    locale: Locale,
    mk_set_area_cb: &F,
) -> Html
where
    F: Fn(u8) -> Callback<MouseEvent>,
{
    let _ = locale;
    let inv = &c.inventory;
    let lvl = shared::level_of(inv);
    let wilds = shared::wilds_areas(inv.plot_seed);
    let entrance = match wilds.iter().find(|a| a.id == shared::WILDS_AREA_BASE) {
        Some(e) => e,
        None => return html! {},
    };
    if lvl + 5 < entrance.min_level {
        return html! {};
    }
    // Group by depth = max(predecessor depth) + 1, same fixed-
    // point relaxation as the main graph.
    let mut depths: std::collections::BTreeMap<u8, u8> =
        std::collections::BTreeMap::new();
    for area in &wilds {
        if area.predecessors.is_empty() {
            depths.insert(area.id, 0);
        }
    }
    let mut changed = true;
    let mut guard = 0;
    while changed && guard < 32 {
        changed = false;
        for area in &wilds {
            if area.predecessors.is_empty() { continue; }
            let max_pred = area.predecessors.iter()
                .filter_map(|p| depths.get(p).copied())
                .max();
            if let Some(d) = max_pred {
                let new_d = d + 1;
                let entry = depths.entry(area.id).or_insert(new_d);
                if *entry != new_d {
                    *entry = new_d;
                    changed = true;
                }
            }
        }
        guard += 1;
    }
    let mut by_depth: std::collections::BTreeMap<u8, Vec<&shared::AreaDef>> =
        std::collections::BTreeMap::new();
    for area in &wilds {
        let d = depths.get(&area.id).copied().unwrap_or(0);
        by_depth.entry(d).or_default().push(area);
    }
    html! {
        <div id="area-graph-wilds" class="area-graph wilds">
            <crate::app::widgets::GraphEdgeOverlay
                host_id={"area-graph-wilds"}
                bump={inv.area_clears.values().copied().sum::<u64>()
                    .wrapping_add(inv.current_area as u64)
                    .wrapping_add(1_000_000)}
            />
            { for by_depth.iter().map(|(_depth, row_areas)| html! {
                <div class="graph-row">
                    { for row_areas.iter().map(|a| {
                        let has_parent = !a.predecessors.is_empty();
                        let parent_ids_csv = a.predecessors.iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<_>>()
                            .join(",");
                        let node_cls = if has_parent { "graph-node has-parent" } else { "graph-node starter" };
                        let area_id_str = a.id.to_string();
                        html! {
                            <div
                                class={node_cls}
                                data-area-id={area_id_str}
                                data-parent-ids={parent_ids_csv}
                            >
                                { render_area_card(locale, a, inv.current_area, lvl, inv, mk_set_area_cb) }
                            </div>
                        }
                    }) }
                </div>
            }) }
        </div>
    }
}
