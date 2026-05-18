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
                // Delegate's `LoadUiPrefs` overrides this on connect.
                // Defaulting here so first-paint has something to
                // show before the WS handshake completes.
                name: DEFAULT_NAME.into(),
                inventory: Inventory::default(),
                last_published: Inventory::default(),
                last_published_ms: None,
                mission_in_flight: false,
                catchup_progress: None,
                others: BTreeMap::new(),
                cumulative_damage: BTreeMap::new(),
                ws: None,
                contract_key,
                delegate_key,
                status: "connecting…".into(),
                prefs_loaded: false,
                current_tab: Tab::Home,
                current_theme: DEFAULT_THEME.into(),
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
                last_level_shown: None,
                last_form_shown: None,
                animate_skills: std::collections::BTreeSet::new(),
                last_skills_shown: None,
                revealed_animated: None,
                animate_reveal: 0,
                // Wizard always opens at step 0 on cold load. The
                // delegate's settings reply closes it for returning
                // players a few hundred ms later.
                onboarding_step: Some(0),
                last_seen_version: None,
                catchup_modal_dismissed: false,
                last_catchup_acked_started_ms: 0,
                map_view: crate::app::types::MapView::default(),
                pending_confirm: None,
            });
            // Apply the default theme for first paint; the delegate's
            // `LoadUiPrefs` reply re-applies the player's saved theme
            // once the WS handshake completes.
            apply_theme(DEFAULT_THEME);
            // §8 B4/D2/D3 visual prefs (reduced-motion, reduced-flash,
            // stash-density) — apply at first paint so CSS gates land
            // before any UI renders.
            let prefs_snapshot = core.borrow().as_ref().map(|c| c.prefs.clone());
            if let Some(p) = prefs_snapshot {
                crate::app::prefs::apply_visual_prefs(&p);
            }
            bump_setter.set(now_ms());

            connect_and_setup(core.clone(), pending.clone(), bump_setter.clone());

            // Fallback timer: if `LoadUiPrefs` never replies (delegate
            // unconfigured, WS handshake stalls, server unreachable),
            // flip the gate after PREFS_LOAD_TIMEOUT_MS so the app
            // still becomes usable on defaults instead of wedging on
            // the loader. If the real reply lands first, this is a
            // no-op (prefs_loaded already true).
            {
                let core = core.clone();
                let bump = bump_setter.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    gloo_timers::future::TimeoutFuture::new(PREFS_LOAD_TIMEOUT_MS).await;
                    let mut g = core.borrow_mut();
                    if let Some(c) = g.as_mut() {
                        if !c.prefs_loaded {
                            web_sys::console::warn_1(
                                &"LoadUiPrefs timeout — rendering with defaults".into(),
                            );
                            c.prefs_loaded = true;
                            drop(g);
                            bump.set(now_ms());
                        }
                    }
                });
            }

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
                                c.catchup_progress.is_some(),
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
                        catchup_in_progress,
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
                    // §8 B8 theme schedule. Cheap check on every tick:
                    // if the schedule produces a code that differs
                    // from the active <html data-theme>, re-apply.
                    // Hour transitions happen at most once per minute
                    // so this is a no-op the rest of the time.
                    if let Some(target) = crate::app::prefs::schedule_theme_for_now(&prefs) {
                        let current = web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| d.document_element())
                            .and_then(|r| r.get_attribute("data-theme"))
                            .unwrap_or_default();
                        if !target.is_empty() && current != target {
                            apply_theme(&target);
                        }
                    }

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
                        } else {
                            // While chunked catchup is in progress, override
                            // the user's pull cadence with a tight loop so
                            // the catchup completes in seconds instead of
                            // dragging across many normal heartbeats. We
                            // still gate on `last_pull` to avoid stacking
                            // unbounded in-flight calls — POLL_TICK_MS
                            // (~1s) is the minimum spacing.
                            let pull_interval = if catchup_in_progress {
                                POLL_TICK_MS as u64
                            } else {
                                prefs.sync_cadence.pull_ms()
                            };
                            if now.saturating_sub(last_pull) >= pull_interval {
                                if let Some(c) = core.borrow_mut().as_mut() {
                                    c.last_pull_tick_ms = now;
                                }
                                pull_inventory_once(core.clone(), pending.clone(), bump.clone());
                                // Workaround for the freenet-core regression where
                                // `UpdateNotification` is never delivered for the
                                // locally-hosted presence contract (observed
                                // 2026-05-15, see pull_presence.rs). A bare Get
                                // refreshes `c.others` from the local cache so
                                // the leaderboard advances at `pull_ms` cadence
                                // instead of staying frozen at the initial state.
                                crate::freenet::actions::pull_presence::pull_presence_state_once(
                                    core.clone(),
                                    pending.clone(),
                                    bump.clone(),
                                );
                            }
                        }
                    }

                    // C1+C2 era-advance auto-claim: when the
                    // cross-player cumulative damage crosses an
                    // era threshold, fire `ClaimBossKill` to
                    // award Legacy stars + Tokens. Frontend has
                    // the contract state, delegate doesn't —
                    // hence the client-side trigger. Idempotent:
                    // delegate rejects re-claims of the same era.
                    if has_pubkey {
                        let claim_args = {
                            let g = core.borrow();
                            let Some(c) = g.as_ref() else { return };
                            let (era, _hp_rem, max_hp, _total) =
                                crate::game::derived::world_boss_state(c);
                            if max_hp == 0 {
                                None
                            } else if era > c.inventory.boss_era_witnessed {
                                let my_pk = c.pubkey;
                                let my_dmg = my_pk
                                    .and_then(|pk| c.cumulative_damage.get(&pk).copied())
                                    .unwrap_or(0);
                                let rank = c
                                    .cumulative_damage
                                    .values()
                                    .filter(|d| **d > my_dmg)
                                    .count() as u8;
                                Some((era, max_hp, rank))
                            } else {
                                None
                            }
                        };
                        if let Some((era, max_hp, rank)) = claim_args {
                            crate::freenet::actions::activity::claim_boss_kill_once(
                                core.clone(), pending.clone(), bump.clone(),
                                era, max_hp, rank,
                            );
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
        // Hold rendering until the delegate's `LoadUiPrefs` reply has
        // landed (`prefs_loaded`), so returning users don't see a
        // flash of DEFAULT_NAME + parchment theme + onboarding step 0
        // before their saved values arrive. The timeout fallback
        // below flips the flag after PREFS_LOAD_TIMEOUT_MS so a
        // wedged delegate never leaves the app stuck on the loader.
        Some(c) if !c.prefs_loaded => boot_loader(c),
        Some(c) => render_core(c, (*core).clone(), (*pending).clone(), bump.setter()),
        None => html! { <p>{ "loading…" }</p> },
    };
    drop(guard);
    let _ = *bump;
    view
}

/// Boot-time loader rendered while the delegate's `LoadUiPrefs`
/// round-trip is in flight. Reads `locale` from the localStorage-backed
/// `c.prefs` (always populated synchronously on Core init) so the
/// loader text is in the player's language even before the WS
/// handshake completes. Theme is whatever `apply_theme(DEFAULT_THEME)`
/// already applied at first paint — a brief parchment-loader flash
/// before the saved theme takes over is far less jarring than the
/// previous full-UI-then-snap behavior.
fn boot_loader(c: &Core) -> Html {
    use crate::app::i18n::MessageId;
    let msg = c.prefs.locale.tr(MessageId::BootLoading);
    html! {
        <main class="boot-loader" aria-busy="true">
            <p>{ msg }</p>
        </main>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    set_shell_title("Freenet Idle PoC");
    install_keyboard_shortcuts();
    yew::Renderer::<App>::new().render();
}

/// §8 D4: document-level keydown listener that routes hotkeys to
/// buttons tagged with `data-shortcut="X"`. Off by default —
/// gated on `UserPrefs.keyboard_shortcuts` so a player who didn't
/// opt in doesn't get surprised. Ignores keys when focus is in
/// a text input / textarea / contenteditable so typing in the
/// name / motto field is unaffected.
fn install_keyboard_shortcuts() {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let doc_for_cb = document.clone();
    let cb = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        // Honour the opt-in pref.
        let prefs = crate::app::prefs::load_prefs();
        if !prefs.keyboard_shortcuts {
            return;
        }
        // Don't fight form inputs.
        if let Some(active) = doc_for_cb.active_element() {
            let tag = active.tag_name().to_lowercase();
            if tag == "input" || tag == "textarea" {
                return;
            }
            if active
                .get_attribute("contenteditable")
                .map(|v| v == "true")
                .unwrap_or(false)
            {
                return;
            }
        }
        if e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        let key = e.key();
        let normalized = key.to_uppercase();
        let target_key: String = normalized.chars().next().map(|c| c.to_string()).unwrap_or_default();
        if target_key.is_empty() {
            return;
        }
        let selector = format!("button[data-shortcut=\"{}\"]", target_key);
        let Ok(Some(node)) = doc_for_cb.query_selector(&selector) else { return };
        let Ok(btn) = node.dyn_into::<web_sys::HtmlButtonElement>() else { return };
        if !btn.disabled() {
            btn.click();
            e.prevent_default();
        }
    });
    let _ = window.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Tell the outer freenet gateway shell to display this string as the
/// browser tab title. The shell page (served by the gateway at
/// `/v1/contract/web/<id>/`) hosts our content in a sandboxed iframe;
/// without this postMessage the tab title stays "Freenet" (the shell's
/// own `<title>`). Protocol defined in freenet-core
/// `server/path_handlers.rs:694-697`; the shell truncates to 128 chars.
fn set_shell_title(title: &str) {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else { return };
    let parent = match win.parent() {
        Ok(Some(p)) => p,
        _ => return,
    };
    let msg = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &msg,
        &JsValue::from_str("__freenet_shell__"),
        &JsValue::TRUE,
    );
    let _ = js_sys::Reflect::set(
        &msg,
        &JsValue::from_str("type"),
        &JsValue::from_str("title"),
    );
    let _ = js_sys::Reflect::set(
        &msg,
        &JsValue::from_str("title"),
        &JsValue::from_str(title),
    );
    let _ = parent.post_message(&msg, "*");
}
