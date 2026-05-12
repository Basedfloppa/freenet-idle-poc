//! Idle PoC frontend (delegate-backed identity + inventory).
//!
//! The webapp is a thin client over an authoritative delegate. The
//! browser never owns mutable state:
//!
//!   - Identity (Ed25519 seed) lives in the node's secret store.
//!   - Inventory (gold, essence, mission count, boss damage) lives
//!     in the same secret store. The only mutation path is the
//!     `RunMission` delegate call; the browser cannot mint loot.
//!   - Other players' contributions arrive through a single Subscribe
//!     on the shared presence contract. Summing their `boss_damage`
//!     entries gives the global World Boss HP gauge — no extra WS
//!     connections, the contract is the fan-out.
//!
//! Browser data clear, incognito, fresh browser → same identity and
//! same inventory, because none of it lives in the browser.

mod app;
mod delegate_client;
mod freenet;
mod game;
mod identity;
mod ws_shim;

pub use app::*;
pub use freenet::actions::*;
pub use freenet::contract::*;
pub use freenet::heartbeat::*;
pub use freenet::reconnect::*;
pub use game::derived::*;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gloo_timers::callback::Interval;
use shared::Inventory;
use yew::prelude::*;

use crate::delegate_client::Pending;

#[function_component(App)]
fn app() -> Html {
    let core: UseStateHandle<CoreCell> = use_state(|| Rc::new(RefCell::new(None)));
    let pending: UseStateHandle<PendingCell> =
        use_state(|| Rc::new(RefCell::new(Pending::default())));
    let bump = use_state(|| 0u64);

    {
        let core = (*core).clone();
        let pending = (*pending).clone();
        let bump_setter = bump.setter();
        use_effect_with((), move |_| {
            // Seed Core with the baked-in defaults; connect_inner
            // overwrites them with values from dev-keys.json (if any).
            let delegate_key = match delegate_client::parse_delegate_key(
                DELEGATE_KEY_B58,
                DELEGATE_CODE_HASH_B58,
            ) {
                Ok(k) => k,
                Err(e) => {
                    web_sys::console::error_1(&format!("delegate key: {e}").into());
                    return Box::new(|| ()) as Box<dyn FnOnce()>;
                }
            };
            // Presence is now optional, mirroring mailbox/guilds: empty
            // ids leave us in single-player mode (delegate still works,
            // no leaderboard / World Boss). A non-empty pair that fails
            // to parse is still a hard error — it's a configuration
            // typo, not a "feature disabled" signal.
            let contract_key = parse_contract_key(CONTRACT_ID_B58, CODE_HASH_B58).ok();

            *core.borrow_mut() = Some(Core {
                pubkey: None,
                name: "player".into(),
                inventory: Inventory::default(),
                last_published: Inventory::default(),
                last_published_ms: None,
                mission_in_flight: false,
                others: BTreeMap::new(),
                cumulative_damage: BTreeMap::new(),
                ws: None,
                contract_key,
                delegate_key,
                status: "connecting…".into(),
                current_tab: Tab::Farm,
                current_theme: load_theme(),
                prefs: load_prefs(),
                last_auto_tick_ms: 0,
                last_heartbeat_tick_ms: 0,
                last_pull_tick_ms: 0,
                exported_seed_hex: None,
                mailbox_key: None,
                mailbox: Vec::new(),
                guilds_key: None,
                guilds: shared::GuildsState::default(),
                new_guild_name_input: String::new(),
                toasts: Vec::new(),
                shown_achievements: None,
                onboarding_step: if onboarding_dismissed() { None } else { Some(0) },
            });
            // Apply saved theme immediately so the first paint is in
            // the right palette (avoids a parchment → dark flash).
            apply_theme(&load_theme());
            bump_setter.set(now_ms());

            connect_and_setup(core.clone(), pending.clone(), bump_setter.clone());

            // One Interval drives auto-mission, heartbeat publish and
            // pull-refresh. Each action gates itself by comparing
            // `now_ms()` against its own `last_*_tick_ms` and the
            // matching cadence from `prefs`. That lets cadence
            // changes take effect on the next 1 s tick without
            // tearing down + recreating timers, and gives the
            // single chokepoint for adding more periodic work later.
            let unified_tick = {
                let core = core.clone();
                let pending = pending.clone();
                let bump = bump_setter.clone();
                Interval::new(POLL_TICK_MS, move || {
                    // Snapshot the bits we need without holding the
                    // borrow across the action calls (each action
                    // re-borrows `core` for itself).
                    // Prune expired toasts on every tick — cheap
                    // O(n) over a Vec capped at MAX_TOASTS, runs once
                    // a second, fine.
                    {
                        let mut g = core.borrow_mut();
                        if let Some(c) = g.as_mut() {
                            let cutoff = now_ms().saturating_sub(TOAST_TTL_MS);
                            c.toasts.retain(|t| t.created_ms >= cutoff);
                        }
                    }
                    let snapshot = {
                        let g = core.borrow();
                        g.as_ref().map(|c| {
                            (
                                c.inventory.auto_run_enabled,
                                c.mission_in_flight,
                                c.pubkey.is_some(),
                                c.inventory.current_hp,
                                max_hp_from(&c.inventory),
                                c.inventory.current_battle.is_some(),
                                c.prefs.clone(),
                                c.last_auto_tick_ms,
                                c.last_heartbeat_tick_ms,
                                c.last_pull_tick_ms,
                            )
                        })
                    };
                    let Some((
                        auto_run,
                        mission_in_flight,
                        has_pubkey,
                        current_hp,
                        hp_max,
                        in_battle,
                        prefs,
                        last_auto,
                        last_heartbeat,
                        last_pull,
                    )) = snapshot else { return };
                    let now = now_ms();

                    // Auto-mission: gated on auto-toggle + WS handshake
                    // + HP threshold (`prefs.auto_pause_hp_pct`). When
                    // a battle is already active, the tick path below
                    // advances it — we only fire a fresh RunMission
                    // (which starts a new battle) when idle.
                    if auto_run && !mission_in_flight && has_pubkey && !in_battle {
                        let hp_pct = if hp_max == 0 {
                            0
                        } else {
                            ((current_hp.saturating_mul(100)) / hp_max).min(100) as u8
                        };
                        let above_threshold = if prefs.auto_pause_hp_pct == 0 {
                            current_hp > 0
                        } else {
                            hp_pct > prefs.auto_pause_hp_pct
                        };
                        if above_threshold && now.saturating_sub(last_auto) >= AUTO_TICK_MS {
                            if let Some(c) = core.borrow_mut().as_mut() {
                                c.last_auto_tick_ms = now;
                            }
                            run_mission_once(core.clone(), pending.clone(), bump.clone());
                        }
                    }

                    // Heartbeat publish — only useful once we have a
                    // pubkey to sign with.
                    if has_pubkey
                        && now.saturating_sub(last_heartbeat) >= prefs.sync_cadence.heartbeat_ms()
                    {
                        if let Some(c) = core.borrow_mut().as_mut() {
                            c.last_heartbeat_tick_ms = now;
                        }
                        heartbeat_once(core.clone(), pending.clone(), bump.clone());
                    }

                    // During an active battle the regular pull cadence
                    // is too slow (HP bars would update every 10 s).
                    // Tick the battle every POLL_TICK_MS instead — the
                    // RPC returns the post-tick inventory, so this also
                    // doubles as the pull. Outside battle we fall back
                    // to the configured `pull_ms` cadence.
                    if has_pubkey {
                        if in_battle {
                            if now.saturating_sub(last_pull) >= POLL_TICK_MS as u64 {
                                if let Some(c) = core.borrow_mut().as_mut() {
                                    c.last_pull_tick_ms = now;
                                }
                                tick_battle_once(
                                    core.clone(),
                                    pending.clone(),
                                    bump.clone(),
                                );
                            }
                        } else if now.saturating_sub(last_pull) >= prefs.sync_cadence.pull_ms() {
                            if let Some(c) = core.borrow_mut().as_mut() {
                                c.last_pull_tick_ms = now;
                            }
                            pull_inventory_once(core.clone(), pending.clone(), bump.clone());
                        }
                    }
                })
            };

            Box::new(move || {
                drop(unified_tick);
            }) as Box<dyn FnOnce()>
        });
    }

    let guard = (*core).borrow();
    let view = match guard.as_ref() {
        Some(c) => render_core(c, (*core).clone(), (*pending).clone(), bump.setter()),
        None => html! { <p>{ "loading…" }</p> },
    };
    drop(guard);
    let _ = *bump;
    view
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
