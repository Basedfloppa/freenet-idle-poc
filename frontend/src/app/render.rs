//! Top-level renderer. `render_core` is the giant view builder
//! that produces the entire page DOM for one frame: it bakes per-
//! callback closures (Yew can't take params directly), reads
//! authoritative state from `Core`, and dispatches to per-tab
//! sub-views composed from `widgets`.

use shared::{
    area_of, form_name, form_slot_mask, form_sprite, format_si, level_of, plot_tuple,
    shop_buy_price, skill_blurb, skill_buy_price, skill_name, PresencePayload, PubKey,
    AREAS, CONSUMABLE_FIREBALL, CONSUMABLE_POTION, ENCOUNTERS_PER_MISSION,
    FIREBALL_BOSS_DAMAGE, FIREBALL_PRICE, MISSION_DAMAGE, MISSION_ESSENCE, MISSION_GOLD,
    POTION_PRICE, SKILL_DRAGON_SCALES, SKILL_FELINE_GRACE, SKILL_SLIME_BODY,
    SKILL_STEED_HEART, SLOT_COUNT, SLOT_NAMES, STATUS_ADVENTURING, STATUS_DEFEATED,
    STATUS_FOCUSING, STATUS_RECOVERING, WHEAT_PER_GOLD,
};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::freenet::actions::{
    auto_equip_once, buy_gear_once, buy_item_once, buy_skill_once, equip_gear_once,
    export_seed_once, forge_upgrade_once, guild_op_once, queue_battle_action_once,
    reset_inventory_once, run_mission_once, sell_gear_once, sell_wheat_once,
    send_message_once, set_area_once, set_auto_run_once, unequip_slot_once,
    use_consumable_once, work_farm_once,
};
use crate::game::derived::{
    area_of_name, attack_from, current_chapter, defence_from, equipped_bonuses, max_hp_from,
    player_speed_evasion, status_code, status_text, world_boss_state, xp_in_level,
};

use super::core::{ingest_inventory, Core, ONBOARDING_STEPS};
use super::util::DEFAULT_WS;
use super::prefs::{apply_theme, clear_all_prefs, save_prefs, SyncCadence, THEMES};
use super::types::{Tab, ToggleField};
use super::util::{now_ms, truncate};
use super::widgets::{
    render_area_card, render_battle_queue, render_battle_stage, render_catchup_banner,
    render_combat_history, render_debug_overlay, render_equipped_slot, render_mailbox_panel,
    render_onboarding, render_stash_grouped, render_toasts, top_actions,
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
            crate::freenet::actions::ui_prefs::save_ui_prefs_once(
                core.clone(),
                pending.clone(),
                bump.clone(),
                Some(new_name),
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
                    .confirm_with_message(&format!(
                        "Disband \"{guild_name}\"?\n\nThis removes the guild entirely and \
                         every member loses their membership immediately. Only you (the \
                         current leader) can do this; if you change your mind, just don't \
                         click OK.",
                    ))
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
                crate::freenet::actions::ui_prefs::save_ui_prefs_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    None,
                    Some(theme_id.to_string()),
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
                .confirm_with_message(
                    "Reset all inventory progress?\n\n\
                     Your identity (pubkey) stays the same — leaderboards \
                     keep recognizing you — but every counter, item, skill, \
                     ending, and achievement goes back to zero.",
                )
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
                .confirm_with_message(
                    "Reveal your Ed25519 seed?\n\n\
                     Anyone holding it can impersonate you. Only paste it \
                     into trusted backup storage; never into chat or screenshots.",
                )
                .unwrap_or(false);
            if !confirmed { return }
            let core_for_cb = core.clone();
            let bump_for_cb = bump.clone();
            export_seed_once(core.clone(), pending.clone(), move |result| {
                if let Some(c) = core_for_cb.borrow_mut().as_mut() {
                    match result {
                        Ok(seed) => {
                            c.exported_seed_hex = Some(hex::encode(seed));
                            c.status = "seed exported — copy and hide promptly".into();
                        }
                        Err(e) => {
                            c.status = format!("export failed: {e}");
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
    // (`save_ui_prefs_once`) so it survives reload — localStorage
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
                crate::freenet::actions::ui_prefs::save_ui_prefs_once(
                    core.clone(),
                    pending.clone(),
                    bump.clone(),
                    None,
                    None,
                    Some(true),
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
            crate::freenet::actions::ui_prefs::save_ui_prefs_once(
                core.clone(),
                pending.clone(),
                bump.clone(),
                None,
                None,
                Some(true),
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

    let publish_age = c
        .last_published_ms
        .map(|ms| format!("{}s ago", (now.saturating_sub(ms)) / 1000))
        .unwrap_or_else(|| "never".into());

    let pubkey_text = my
        .map(|pk| format!("pubkey (from delegate): {}", crate::short_id(&pk)))
        .unwrap_or_else(|| "pubkey: pending delegate response".into());

    let auto_label = if c.inventory.auto_run_enabled { "auto: on" } else { "auto: off" };
    let mission_disabled = my.is_none() || c.mission_in_flight;

    let inv = &c.inventory;
    let lvl = level_of(inv);
    let hp_max = max_hp_from(inv);
    let atk = attack_from(inv);
    let def = defence_from(inv);
    let (chap_no, chap_title, chap_body) = current_chapter(inv);
    let area = area_of(inv.current_area);
    let _mission_gold = MISSION_GOLD.saturating_mul(area.gold_mult);
    let mission_essence = MISSION_ESSENCE.saturating_mul(area.essence_mult);
    let mission_damage = MISSION_DAMAGE.saturating_mul(area.damage_mult);
    let (eq_atk, eq_def, eq_hp) = equipped_bonuses(inv);
    let stash_count = inv.unequipped.len();
    let (xp_cur, xp_req) = xp_in_level(inv);
    let xp_pct = if xp_req == 0 { 100 } else { (xp_cur * 100 / xp_req).min(100) };
    let (p_speed, p_evasion) = player_speed_evasion(inv);
    let status_pill_cls = match status_code(c) {
        STATUS_DEFEATED => "pill defeated",
        STATUS_FOCUSING => "pill casting",
        STATUS_ADVENTURING => "pill auto",
        STATUS_RECOVERING => "pill recovering",
        _ => "pill idle",
    };
    let _ = area_of_name;

    html! {
        <main>
            { render_toasts(&c.toasts, now) }
            { render_onboarding(c.onboarding_step, on_onboarding_next, on_onboarding_skip) }
            <header class="page-head">
                <div class="title-row">
                    <h1>{ "Freenet Idle PoC" }</h1>
                    <span class={status_pill_cls}>{ status_text(c) }</span>
                    <a class="repo-link"
                       href="https://github.com/Basedfloppa/freenet-idle-poc"
                       target="_blank"
                       rel="noopener noreferrer">
                        { "source ↗" }
                    </a>
                </div>
                <p class="status">{ &c.status }</p>
            </header>

            <nav class="top-actions">
                { for top_actions().iter().map(|(icon, label, tab)| {
                    let is_active = c.current_tab == *tab;
                    let cls = if is_active { "icon-btn active" } else { "icon-btn" };
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
                        { render_catchup_banner(&inv.last_catchup) }
                        {
                            if inv.mission_count == 0 {
                                html! {
                                    <section class="panel tutorial">
                                        <h2>{ "welcome, wanderer" }</h2>
                                        <p>
                                            { "Click " }<strong>{ "Run Mission" }</strong>
                                            { " to fight the area's enemy. Every 5 wins drop gear (manage at the Shop tab), every 13 wins drop a potion, every 19 a fireball." }
                                        </p>
                                        <p class="muted small">
                                            { "Take damage in combat? HP regenerates over time, or use a potion to heal instantly. Pick a different battlefield from the World Map when you out-level the current one." }
                                        </p>
                                    </section>
                                }
                            } else {
                                html! {}
                            }
                        }
                        <section class="grid-3">
                            <article class="panel stats">
                                <h2>{ "hero" }</h2>
                                <div class="stat-row">
                                    <label>{ "Name " }
                                        <input type="text" value={c.name.clone()} oninput={on_name} />
                                    </label>
                                </div>
                                <table class="statgrid">
                                    <tbody>
                                        <tr>
                                            <th>{"Form"}</th>
                                            <td class="num">
                                                <span class="form-name">
                                                    { format!("{} {}", form_sprite(inv.current_form), form_name(inv.current_form)) }
                                                </span>
                                            </td>
                                        </tr>
                                        <tr><th>{"Level"}</th><td class="num">{ lvl }</td></tr>
                                        <tr>
                                            <th>{"XP"}</th>
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
                                            <th>{"HP"}</th>
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
                                        <tr><th>{"Attack"}</th><td class="num">{ atk }</td></tr>
                                        <tr><th>{"Defence"}</th><td class="num">{ def }</td></tr>
                                        <tr><th>{"Speed"}</th><td class="num">{ p_speed }</td></tr>
                                        <tr><th>{"Evasion"}</th><td class="num">{ format!("{p_evasion}%") }</td></tr>
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
                                        render_battle_stage(battle, inv, hp_max)
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
                                                    "fight in progress — wait for the current battle to end"
                                                } else { "" }
                                            }>
                                        { "Run Mission" }
                                    </button>
                                    <button onclick={on_toggle_auto}
                                            disabled={my.is_none()}
                                            title={
                                                if inv.current_battle.is_some() {
                                                    "auto toggle still works during a fight — the new setting takes effect once the current battle ends"
                                                } else { "" }
                                            }>
                                        { auto_label }
                                    </button>
                                </div>
                                {
                                    // Mid-fight queue + recent-turns ticker live
                                    // here, below the action row, so the player
                                    // can react without losing the auto / Run
                                    // controls.
                                    if let Some(battle) = inv.current_battle.as_ref() {
                                        render_battle_queue(battle, inv)
                                    } else {
                                        html! {
                                            <p class="tooltip muted">
                                                {
                                                    if mission_damage == 0 {
                                                        format!("Mission in {}: up to {} encounters, ~{} essence per win, no World Boss contribution from this area — gold scales by enemy",
                                                                area.name, ENCOUNTERS_PER_MISSION, mission_essence)
                                                    } else {
                                                        format!("Mission in {}: up to {} encounters, ~{} essence + ~{} boss damage per win — gold scales by enemy",
                                                                area.name, ENCOUNTERS_PER_MISSION, mission_essence, mission_damage)
                                                    }
                                                }
                                            </p>
                                        }
                                    }
                                }
                                <p class="muted small">
                                    { format!("last publish: {publish_age} · published gold {} · published damage {}",
                                              format_si(c.last_published.gold), format_si(c.last_published.boss_damage)) }
                                </p>
                                { render_combat_history(&inv.combat_history) }
                            </article>

                            <article class="panel equipment">
                                <h2>{ "equipment" }</h2>
                                <p class="muted small">{ format!("equipped bonus: +{eq_atk} atk · +{eq_def} def · +{eq_hp} hp") }</p>
                                <div class="action-row">
                                    <button
                                        onclick={on_auto_equip}
                                        disabled={inv.unequipped.is_empty()}
                                        title="walk every slot and equip the highest stat-sum piece you own"
                                    >
                                        { "Auto-Equip Best" }
                                    </button>
                                </div>
                                <div class="slot-grid">
                                    { for (0..SLOT_COUNT).map(|i| render_equipped_slot(i, inv, &mk_unequip_cb)) }
                                </div>
                                {
                                    if stash_count == 0 {
                                        html! {
                                            <p class="muted small">
                                                { format!("no spare loot yet — gear drops every {} missions", shared::GEAR_DROP_EVERY) }
                                            </p>
                                        }
                                    } else {
                                        html! {
                                            <p class="muted small">
                                                { format!("{stash_count} item{} in stash — manage at the Shop tab",
                                                          if stash_count == 1 { "" } else { "s" }) }
                                            </p>
                                        }
                                    }
                                }
                                <h3>{ "consumables" }</h3>
                                // Single canonical position. Clicks route via
                                // `mk_use_cb`: idle → `UseConsumable`, mid-battle
                                // → `QueueBattleAction` (queue for next turn).
                                // The tooltip flips to match the routed semantic.
                                <div class="consumable-row">
                                    <span class="consumable">
                                        <span class="name">{ "Potion" }</span>
                                        <span class="qty">{ inv.potions }</span>
                                        <button
                                            onclick={on_use_potion}
                                            disabled={inv.potions == 0}
                                            title={
                                                if inv.current_battle.is_some() {
                                                    "queue: heal to full on the next combat turn"
                                                } else {
                                                    "heals HP fully"
                                                }
                                            }
                                        >
                                            { "Use" }
                                        </button>
                                    </span>
                                    <span class="consumable">
                                        <span class="name">{ "Fireball" }</span>
                                        <span class="qty">{ inv.fireballs }</span>
                                        <button
                                            onclick={on_use_fireball}
                                            disabled={inv.fireballs == 0}
                                            title={
                                                if inv.current_battle.is_some() {
                                                    "queue: bonus damage on the next combat turn".to_string()
                                                } else {
                                                    format!("deals {FIREBALL_BOSS_DAMAGE} damage to the World Boss")
                                                }
                                            }
                                        >
                                            { "Use" }
                                        </button>
                                    </span>
                                </div>
                            </article>
                        </section>

                        <section class="panel plot">
                            <h2>{ "The Plot So Far…" }</h2>
                            {
                                if inv.plot_seed != 0 {
                                    let (home, mac, vil, mthd, dest) = plot_tuple(inv.plot_seed);
                                    html! {
                                        <p class="plot-backstory">
                                            { format!(
                                                "You were abandoned in the {home} as a baby. Then one day, the {mac} disappeared! Surely the {vil} used the {mthd} to take it! Now you must journey to the {dest} to confront them."
                                            ) }
                                        </p>
                                    }
                                } else {
                                    html! {}
                                }
                            }
                            <p class="chapter-no muted">{ format!("Chapter {chap_no}") }</p>
                            <p>{ chap_body }</p>
                        </section>

                        <section class="panel boss">
                            <h2>{ "World Boss" }</h2>
                            <div class="hp-bar">
                                <div class="hp-fill" style={format!("width: {boss_pct}%")}></div>
                            </div>
                            <p class="muted">
                                { format!("Era {boss_era} · {} / {} HP — {} total damage from {} players",
                                          format_si(boss_hp), format_si(boss_max_hp), format_si(total_dmg), rows.len()) }
                            </p>
                        </section>

                        <section class="panel resources">
                            <h2>{ "resources" }</h2>
                            <table class="inventory">
                                <tbody>
                                    <tr><th>{"gold"}</th><td class="num">{ format_si(inv.gold) }</td></tr>
                                    <tr><th>{"essence"}</th><td class="num">{ format_si(inv.essence) }</td></tr>
                                    <tr><th>{"missions"}</th><td class="num">{ format_si(inv.mission_count) }</td></tr>
                                    <tr><th>{"boss damage"}</th><td class="num">{ format_si(inv.boss_damage) }</td></tr>
                                </tbody>
                            </table>
                        </section>
                    </>
                },
                Tab::WorldMap => html! {
                    <>
                        <section class="panel world-map">
                            <h2>{ "world map" }</h2>
                            <p class="muted small">
                                { format!("currently farming: {} · level {lvl}", area.name) }
                            </p>
                            <div class="area-grid">
                                { for AREAS.iter().map(|a| render_area_card(a, inv.current_area, lvl, &mk_set_area_cb)) }
                            </div>
                        </section>
                        <section class="panel plot">
                            <h2>{ "The Plot So Far…" }</h2>
                            <p class="chapter-no muted">{ format!("Chapter {chap_no}") }</p>
                            <p>{ chap_body }</p>
                        </section>
                    </>
                },
                Tab::Shop => html! {
                    <>
                        <section class="panel shop">
                            <h2>{ "shop" }</h2>
                            <p class="muted small">
                                { format!("gold balance: {} · potions: {} · fireballs: {}",
                                          format_si(inv.gold), inv.potions, inv.fireballs) }
                            </p>
                            <div class="shop-items">
                                <div class="shop-item">
                                    <span class="name">{ "Potion" }</span>
                                    <span class="desc muted">
                                        { "fully heals your HP" }
                                    </span>
                                    <button
                                        onclick={on_buy_potion}
                                        disabled={inv.gold < POTION_PRICE}
                                    >
                                        { format!("Buy ({POTION_PRICE}g)") }
                                    </button>
                                </div>
                                <div class="shop-item">
                                    <span class="name">{ "Fireball" }</span>
                                    <span class="desc muted">
                                        { format!("{FIREBALL_BOSS_DAMAGE} instant boss damage") }
                                    </span>
                                    <button
                                        onclick={on_buy_fireball}
                                        disabled={inv.gold < FIREBALL_PRICE}
                                    >
                                        { format!("Buy ({FIREBALL_PRICE}g)") }
                                    </button>
                                </div>
                            </div>
                        </section>
                        <section class="panel stash">
                            <h2>{ format!("stash ({})", inv.unequipped.len()) }</h2>
                            <p class="muted small">
                                { "items grouped by slot — equip to wear, sell back to the merchant for tier-priced gold" }
                            </p>
                            { render_stash_grouped(inv, &mk_equip_cb, &mk_sell_cb, &mk_forge_cb) }
                        </section>

                        <section class="panel buy-gear">
                            <h2>{ "buy gear" }</h2>
                            <p class="muted small">
                                { "pre-rolled equipment at the smithy. each click of Buy adds one piece of the requested slot+tier to your stash. legendary (T4) only via forge or drop." }
                            </p>
                            <table class="buy-grid">
                                <thead>
                                    <tr>
                                        <th>{"slot"}</th>
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
                                                <th>{ SLOT_NAMES[slot_idx] }</th>
                                                { for [1u8, 2, 3].iter().map(|t| {
                                                    let price = shop_buy_price(*t);
                                                    html! {
                                                        <td class="num">
                                                            <button
                                                                onclick={mk_buy_gear_cb(slot_u8, *t)}
                                                                disabled={inv.gold < price}
                                                            >{ "Buy" }</button>
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
                            <h2>{ "the sage (buy skills)" }</h2>
                            <p class="muted small">
                                { "the Sage trades permanent skill lore for essence. Veteran/Champion still come from level milestones — those aren't for sale." }
                            </p>
                            <ul class="skill-shop">
                                { for [SKILL_SLIME_BODY, SKILL_FELINE_GRACE, SKILL_DRAGON_SCALES, SKILL_STEED_HEART].iter().map(|sid| {
                                    let owned = inv.skills_unlocked.contains_key(sid);
                                    let price = skill_buy_price(*sid).unwrap_or(u64::MAX);
                                    let cant_afford = inv.essence < price;
                                    let disabled = owned || cant_afford;
                                    let label = if owned {
                                        "owned".to_string()
                                    } else {
                                        format!("Buy ({price}e)")
                                    };
                                    html! {
                                        <li class={if owned { "skill-shop-row owned" } else { "skill-shop-row" }}>
                                            <span class="skill-name">{ skill_name(*sid) }</span>
                                            <span class="skill-blurb muted small">{ skill_blurb(*sid) }</span>
                                            <button onclick={mk_buy_skill_cb(*sid)} disabled={disabled}>{ label }</button>
                                        </li>
                                    }
                                }) }
                            </ul>
                        </section>

                        <section class="panel farm">
                            <h2>{ "farm" }</h2>
                            <p class="muted small">
                                { format!("safe non-combat income. each Work click yields +1 wheat; the merchant pays 1 gold per {} wheat.", WHEAT_PER_GOLD) }
                            </p>
                            <p>
                                { format!("wheat: {} · would sell for {}g",
                                          format_si(inv.wheat), format_si(inv.wheat / WHEAT_PER_GOLD)) }
                            </p>
                            <div class="action-row">
                                <button onclick={on_work_farm}>{ "Work the Farm (+1 wheat)" }</button>
                                <button
                                    onclick={on_sell_all_wheat}
                                    disabled={inv.wheat < WHEAT_PER_GOLD}
                                    title={format!("convert all wheat to gold at 1:{}", WHEAT_PER_GOLD)}
                                >
                                    { "Sell All Wheat" }
                                </button>
                            </div>
                        </section>
                        <section class="panel resources">
                            <h2>{ "resources" }</h2>
                            <table class="inventory">
                                <tbody>
                                    <tr><th>{"gold"}</th><td class="num">{ format_si(inv.gold) }</td></tr>
                                    <tr><th>{"essence"}</th><td class="num">{ format_si(inv.essence) }</td></tr>
                                    <tr><th>{"potions"}</th><td class="num">{ inv.potions }</td></tr>
                                    <tr><th>{"fireballs"}</th><td class="num">{ inv.fireballs }</td></tr>
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
                                <h2>{ "guilds" }</h2>
                                <p class="muted small">
                                    { "Cooperative groups — early scaffolding. Create one, others can join by id. Each player is in at most one guild; leaders auto-handoff on leave." }
                                </p>
                                {
                                    if !configured {
                                        html! {
                                            <p class="muted small">
                                                { "Guilds contract not configured. Publish " }
                                                <code>{ "guilds-contract" }</code>
                                                { " via " }
                                                <code>{ "scripts/dev-publish.sh" }</code>
                                                { " (extension WIP) or override the keys in " }
                                                <code>{ "dev-keys.json" }</code>
                                                { "." }
                                            </p>
                                        }
                                    } else if my_guild_idx.is_none() {
                                        html! {
                                            <div class="guild-create">
                                                <h3>{ "create a guild" }</h3>
                                                <div class="action-row">
                                                    <input
                                                        type="text"
                                                        placeholder="guild name (≤ 32 bytes)"
                                                        value={c.new_guild_name_input.clone()}
                                                        oninput={on_guild_name_input}
                                                    />
                                                    <button class="primary"
                                                            onclick={on_create_guild}
                                                            disabled={c.new_guild_name_input.trim().is_empty()}>
                                                        { "Create" }
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
                                        html! {
                                            <div class="guild-mine">
                                                <h3>{ format!("you are in: {}", g.name) }</h3>
                                                <p class="muted small">
                                                    { format!("members: {} / {} · leader: {}",
                                                              g.members.len(), shared::MAX_GUILD_MEMBERS,
                                                              if is_leader { "you".to_string() } else { crate::short_id(&g.leader) }) }
                                                </p>
                                                <div class="action-row">
                                                    <button onclick={leave_cb}>{ "Leave guild" }</button>
                                                    {
                                                        if is_leader {
                                                            html! {
                                                                <button onclick={disband_cb}
                                                                        title="leader-only: delete the guild for everyone">
                                                                    { "Disband guild" }
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
                                <h2>{ format!("directory ({})", c.guilds.guilds.len()) }</h2>
                                {
                                    if c.guilds.guilds.is_empty() {
                                        html! { <p class="muted small">{ "(no guilds yet — be the first)" }</p> }
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
                                                                    html! { <span class="badge live">{ "you" }</span> }
                                                                } else {
                                                                    html! {
                                                                        <button onclick={join_cb} disabled={!can_join}>
                                                                            { "Join" }
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
                    inv, now, boss_era, boss_hp, boss_max_hp, boss_pct, total_dmg, &rows,
                ),
                Tab::Help => crate::app::tabs::render_help_tab(),
                Tab::Settings => html! {
                    <>
                        <section class="panel settings">
                            <h2>{ "settings" }</h2>

                            <h3>{ "theme" }</h3>
                            <p class="muted small">
                                { "Pick a palette. Saved to this browser's local storage; takes effect immediately and persists across reloads." }
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

                            <h3>{ "sync cadence" }</h3>
                            <p class="muted small">
                                { "How often the webapp talks to your local node. Aggressive = snappier leaderboard, more node traffic. Easy = lighter, but the contract prunes you after 60 s of silence so don't go past that." }
                            </p>
                            <div class="theme-picker">
                                { for [SyncCadence::Aggressive, SyncCadence::Normal, SyncCadence::Easy].iter().map(|cad| {
                                    let is_active = c.prefs.sync_cadence == *cad;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    html! {
                                        <button class={cls} onclick={mk_cadence_cb(*cad)} disabled={is_active}>
                                            { cad.label() }
                                        </button>
                                    }
                                }) }
                            </div>

                            <h3>{ "auto-mission" }</h3>
                            <p class="muted small">
                                { "Pause the auto-loop when HP drops below this fraction of your maximum. 0% keeps the old behaviour — only stop at 0 HP. Higher values save you from losing HP/forms/consumables to a string of bad rolls." }
                            </p>
                            <div class="theme-picker">
                                { for [0u8, 25, 50].iter().map(|pct| {
                                    let is_active = c.prefs.auto_pause_hp_pct == *pct;
                                    let cls = if is_active { "theme-btn active" } else { "theme-btn" };
                                    let label = if *pct == 0 { "0% (only at 0 HP)".to_string() } else { format!("{pct}%") };
                                    html! {
                                        <button class={cls} onclick={mk_hp_pause_cb(*pct)} disabled={is_active}>
                                            { label }
                                        </button>
                                    }
                                }) }
                            </div>

                            <h3>{ "publish behavior" }</h3>
                            <label class="setting-toggle">
                                <input
                                    type="checkbox"
                                    checked={c.prefs.reactive_publish}
                                    onclick={mk_toggle_cb(ToggleField::ReactivePublish)}
                                />
                                { " publish immediately after a mission (in addition to the periodic heartbeat)" }
                            </label>

                            <h3>{ "identity & backup" }</h3>
                            <p class="muted small">
                                {
                                    if c.prefs.hide_pubkey {
                                        "pubkey hidden (toggle in advanced to reveal)".to_string()
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
                                        html! { <p><span class="muted">{ "pubkey pending..." }</span></p> }
                                    }
                                } else { html! {} }
                            }
                            <p class="muted small">
                                { "Export the Ed25519 seed to move identity to another node, or wipe inventory back to a fresh-character state. " }
                                <strong>{ "Reset progress is destructive" }</strong>
                                { " — pubkey survives but every counter, item, skill, and ending goes to zero." }
                            </p>
                            <div class="action-row">
                                <button onclick={on_export_seed} disabled={my.is_none()}>
                                    { "Export seed" }
                                </button>
                                <button onclick={on_reset_progress} disabled={my.is_none()}>
                                    { "Reset progress" }
                                </button>
                            </div>
                            {
                                if let Some(hex) = c.exported_seed_hex.as_ref() {
                                    html! {
                                        <div class="seed-reveal">
                                            <p class="muted small">
                                                { "Copy this once. Anyone with these bytes can impersonate you on the contract." }
                                            </p>
                                            <code class="pubkey-full">{ hex.clone() }</code>
                                            <div class="action-row">
                                                <button onclick={on_hide_seed.clone()}>{ "Hide" }</button>
                                            </div>
                                        </div>
                                    }
                                } else { html! {} }
                            }

                            <details class="settings-advanced">
                                <summary>{ "advanced" }</summary>
                                <p class="muted small">
                                    { "Lower-traffic / privacy / debug switches. Defaults are fine for most players." }
                                </p>

                                <label class="setting-toggle">
                                    <input
                                        type="checkbox"
                                        checked={c.prefs.hide_pubkey}
                                        onclick={mk_toggle_cb(ToggleField::HidePubkey)}
                                    />
                                    { " hide pubkey (Hero panel + Settings)" }
                                </label>

                                <label class="setting-toggle">
                                    <input
                                        type="checkbox"
                                        checked={c.prefs.hide_stale_players}
                                        onclick={mk_toggle_cb(ToggleField::HideStale)}
                                    />
                                    { " hide stale players from leaderboard (last seen > 30 s ago)" }
                                </label>

                                <label class="setting-text">
                                    <span>{ "WS URL override (empty = use ?ws= or default; takes effect after page reload):" }</span>
                                    <input
                                        type="text"
                                        value={c.prefs.ws_url_override.clone()}
                                        oninput={on_ws_input}
                                        placeholder={DEFAULT_WS.to_string()}
                                    />
                                </label>

                                <h3 class="advanced-subhead">{ "reset UI preferences" }</h3>
                                <p class="muted small">
                                    { "Clears theme + cadence + auto-pause + advanced toggles and reloads the page. Doesn't touch your inventory — that lives on the node." }
                                </p>
                                <div class="action-row">
                                    <button onclick={on_reset_prefs}>{ "Reset to defaults" }</button>
                                </div>

                                <h3 class="advanced-subhead">{ "mailbox (D2D test)" }</h3>
                                { render_mailbox_panel(c, on_mailbox_self_test) }

                                { render_debug_overlay(c, now) }
                            </details>

                            <h3>{ "where state lives" }</h3>
                            <p class="muted small">
                                { "Local view is just a cache of what lives on the node. Reload the page — identity and inventory come straight back from the delegate. To actually delete your save, wipe `~/.config/freenet/secrets/local/<delegate-key>/` on the node." }
                            </p>
                        </section>
                    </>
                },
            } }
        </main>
    }
}
