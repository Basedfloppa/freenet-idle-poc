//! WebSocket connect handshake + reconnect-on-drop. Failures along
//! the chain bubble up to `schedule_reconnect` for backoff retry.

use std::cell::RefCell;
use std::rc::Rc;

use freenet_stdlib::client_api::{
    ClientRequest, ContractRequest, Error as WebApiError,
};
use gloo_timers::callback::Timeout;
use shared::{DelegateRequest as AppRequest, DelegateResponse as AppResponse, Inventory};
use wasm_bindgen_futures::spawn_local;

use crate::app::i18n::MessageId;
use crate::delegate_client;
use crate::freenet::contract::{handle_response, parse_contract_key};
use crate::identity;
use crate::ws_shim::WsShim;
use crate::{
    load_dev_keys, now_ms, ws_url, CoreCell, PendingCell, CODE_HASH_B58, CONTRACT_ID_B58,
    DELEGATE_CODE_HASH_B58, DELEGATE_KEY_B58, GUILDS_CODE_HASH_B58, GUILDS_CONTRACT_ID_B58,
    MAILBOX_CODE_HASH_B58, MAILBOX_CONTRACT_ID_B58, WS_RECONNECT_BACKOFF_MS,
};
use yew::UseStateSetter;

pub fn connect_and_setup(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    connect_and_setup_attempt(core, pending, bump, 0);
}

/// Spawn the connect handshake; on failure, schedule a backoff
/// retry. Same path is reused for both the first connect and every
/// subsequent reconnect, so the retry count cleanly increments.
pub fn connect_and_setup_attempt(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    attempt: usize,
) {
    let err_core = core.clone();
    let err_pending = pending.clone();
    let err_bump = bump.clone();
    spawn_local(async move {
        if let Err(e) = connect_inner(core, pending, bump).await {
            if let Some(c) = err_core.borrow_mut().as_mut() {
                c.status = format!("error: {e}");
            }
            schedule_reconnect(err_core, err_pending, err_bump, attempt + 1);
        }
    });
}

/// Drop the dead WS and schedule another connect attempt. Backoff
/// is bounded by `WS_RECONNECT_BACKOFF_MS` — repeated failures
/// settle on the longest interval rather than growing without
/// bound.
pub fn schedule_reconnect(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    attempt: usize,
) {
    let idx = attempt.min(WS_RECONNECT_BACKOFF_MS.len() - 1);
    let delay = WS_RECONNECT_BACKOFF_MS[idx];
    if let Some(c) = core.borrow_mut().as_mut() {
        c.ws = None;
        c.pubkey = None;
        c.status = format!("disconnected — reconnect attempt {} in {}ms", attempt, delay);
    }
    bump.set(now_ms());
    let core_ = core.clone();
    let pending_ = pending.clone();
    let bump_ = bump.clone();
    Timeout::new(delay, move || {
        connect_and_setup_attempt(core_, pending_, bump_, attempt);
    })
    .forget();
}

async fn connect_inner(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
) -> Result<(), String> {
    let dev = load_dev_keys().await;
    // All three aggregator contracts are now optional: empty / unparseable
    // ids leave the corresponding feature disabled. The delegate is the
    // only hard dependency — without it we can't even resolve identity.
    let contract_key = parse_contract_key(
        &dev.contract_or(CONTRACT_ID_B58),
        &dev.code_or(CODE_HASH_B58),
    )
    .ok();
    let delegate_key = delegate_client::parse_delegate_key(
        &dev.delegate_or(DELEGATE_KEY_B58),
        &dev.delegate_code_or(DELEGATE_CODE_HASH_B58),
    )?;
    let mailbox_key = parse_contract_key(
        &dev.mailbox_contract_or(MAILBOX_CONTRACT_ID_B58),
        &dev.mailbox_code_or(MAILBOX_CODE_HASH_B58),
    )
    .ok();
    let guilds_key = parse_contract_key(
        &dev.guilds_contract_or(GUILDS_CONTRACT_ID_B58),
        &dev.guilds_code_or(GUILDS_CODE_HASH_B58),
    )
    .ok();
    if let Some(c) = core.borrow_mut().as_mut() {
        c.contract_key = contract_key.clone();
        c.delegate_key = delegate_key.clone();
        c.mailbox_key = mailbox_key.clone();
        c.guilds_key = guilds_key.clone();
    }
    let socket = web_sys::WebSocket::new(&ws_url()).map_err(|e| format!("ws: {e:?}"))?;

    let (open_tx, open_rx) = crate::oneshot();
    let open_tx_err = open_tx.clone();

    // Tracks whether `onopen` has fired. The error handler keys
    // off this to decide between two lifecycle states:
    // before open → report failure to the awaiting future;
    // after open → schedule a reconnect (dropped network).
    let opened = Rc::new(RefCell::new(false));
    let opened_for_open = opened.clone();
    let opened_for_err = opened.clone();

    let handler_core = core.clone();
    let handler_pending = pending.clone();
    let handler_bump = bump.clone();
    let reconn_core = core.clone();
    let reconn_pending = pending.clone();
    let reconn_bump = bump.clone();
    let shim = WsShim::start(
        socket,
        move |resp| {
            handle_response(
                resp,
                handler_core.clone(),
                handler_pending.clone(),
                handler_bump.clone(),
            )
        },
        move |e: WebApiError| {
            if !*opened_for_err.borrow() {
                let _ = open_tx_err.send(Err(format!("{e:?}")));
            } else {
                schedule_reconnect(
                    reconn_core.clone(),
                    reconn_pending.clone(),
                    reconn_bump.clone(),
                    0,
                );
            }
        },
        move || {
            *opened_for_open.borrow_mut() = true;
            let _ = open_tx.send(Ok(()));
        },
    );

    open_rx.await?;

    let ws = Rc::new(RefCell::new(shim));
    {
        let mut g = core.borrow_mut();
        let c = g.as_mut().ok_or("no core")?;
        c.ws = Some(ws.clone());
        c.status = c.prefs.locale.tr(MessageId::StatusAskingDelegate).into();
    }
    bump.set(now_ms());

    // Register-then-probe: send the bundled-WASM RegisterDelegate
    // unconditionally first, then probe. Probe-then-register would
    // be cheaper for warm nodes, but if the delegate is missing the
    // node may not return a ClientError on ApplicationMessages to an
    // unknown key — `call().await` then hangs forever and the
    // register path never fires. Always-register is idempotent
    // (duplicate registration of an already-stored delegate is a
    // no-op on the node) and the bundled WASM is local-served, so
    // the cost is one cached fetch + one WS frame per (re)connect.
    {
        let mut g = core.borrow_mut();
        if let Some(c) = g.as_mut() {
            c.status = c.prefs.locale.tr(MessageId::StatusRegisteringDelegate).into();
        }
    }
    bump.set(now_ms());
    delegate_client::ensure_delegate_registered(ws.clone()).await?;
    {
        let mut g = core.borrow_mut();
        if let Some(c) = g.as_mut() {
            c.status = c.prefs.locale.tr(MessageId::StatusAskingDelegate).into();
        }
    }
    bump.set(now_ms());

    // WS FIFO ordering ensures the register lands before this probe.
    let seed = identity::random_seed_candidate();
    let pubkey = match delegate_client::call(
        ws.clone(),
        pending.clone(),
        &delegate_key,
        AppRequest::GetPubkey { seed_if_missing: seed },
    )
    .await?
    {
        AppResponse::Pubkey { pubkey } => pubkey,
        AppResponse::Error(e) => return Err(format!("delegate: {e}")),
        other => return Err(format!("unexpected delegate response: {other:?}")),
    };

    let inventory = match delegate_client::call(
        ws.clone(),
        pending.clone(),
        &delegate_key,
        AppRequest::LoadInventory { now_ms: now_ms() },
    )
    .await?
    {
        AppResponse::Inventory(inv) => inv,
        AppResponse::Error(e) => {
            web_sys::console::warn_1(&format!("LoadInventory: {e}").into());
            Inventory::default()
        }
        other => {
            web_sys::console::warn_1(&format!("LoadInventory unexpected: {other:?}").into());
            Inventory::default()
        }
    };

    {
        let mut g = core.borrow_mut();
        if let Some(c) = g.as_mut() {
            c.pubkey = Some(pubkey);
            c.last_published = inventory.clone();
            // Funnel through `ingest_inventory` so the achievement
            // toast logic establishes its baseline on this first load
            // (it intentionally does NOT toast pre-existing unlocks).
            crate::ingest_inventory(c, inventory);
            c.status = c.prefs.locale.tr(MessageId::StatusSubscribing).into();
        }
    }
    bump.set(now_ms());

    // Settings from the delegate (display name / theme / locale) —
    // sandboxed iframe has null origin so localStorage doesn't
    // survive reload; delegate is the only durable store.
    crate::freenet::actions::settings::load_settings_once(
        core.clone(),
        pending.clone(),
        bump.clone(),
    );

    // Presence contract is optional (single-player when None).
    if let Some(p_key) = contract_key.as_ref() {
        let instance = *p_key.id();
        // Re-prime the local contract store with the bundled WASM
        // so Subscribe/Update don't fail with "missing contract"
        // on nodes that lost the code (e.g. after manual wipe).
        if let Err(e) =
            delegate_client::ensure_presence_contract_published(ws.clone(), CONTRACT_ID_B58).await
        {
            web_sys::console::warn_1(&format!("presence auto-publish: {e:?}").into());
        }
        let get = ClientRequest::ContractOp(ContractRequest::Get {
            key: instance,
            return_contract_code: true,
            subscribe: true,
            blocking_subscribe: false,
        });
        if let Err(e) = ws.borrow_mut().send(get).await {
            web_sys::console::warn_1(&format!("initial Get+Subscribe: {e:?}").into());
        }
    }

    // Mailbox + guilds are optional contracts; failures log but
    // don't kill the connection — presence-only is a valid fallback.
    if let Some(mb_key) = mailbox_key.as_ref() {
        let mb_instance = *mb_key.id();
        let mb_sub = ClientRequest::ContractOp(ContractRequest::Subscribe {
            key: mb_instance,
            summary: None,
        });
        if let Err(e) = ws.borrow_mut().send(mb_sub).await {
            web_sys::console::warn_1(&format!("mailbox subscribe: {e:?}").into());
        }
        let mb_get = ClientRequest::ContractOp(ContractRequest::Get {
            key: mb_instance,
            return_contract_code: false,
            subscribe: false,
            blocking_subscribe: false,
        });
        if let Err(e) = ws.borrow_mut().send(mb_get).await {
            web_sys::console::warn_1(&format!("mailbox initial Get: {e:?}").into());
        }
    }

    if let Some(gl_key) = guilds_key.as_ref() {
        let gl_instance = *gl_key.id();
        let gl_sub = ClientRequest::ContractOp(ContractRequest::Subscribe {
            key: gl_instance,
            summary: None,
        });
        if let Err(e) = ws.borrow_mut().send(gl_sub).await {
            web_sys::console::warn_1(&format!("guilds subscribe: {e:?}").into());
        }
        let gl_get = ClientRequest::ContractOp(ContractRequest::Get {
            key: gl_instance,
            return_contract_code: false,
            subscribe: false,
            blocking_subscribe: false,
        });
        if let Err(e) = ws.borrow_mut().send(gl_get).await {
            web_sys::console::warn_1(&format!("guilds initial Get: {e:?}").into());
        }
    }

    {
        let mut g = core.borrow_mut();
        if let Some(c) = g.as_mut() {
            c.status = "subscribed".into();
        }
    }
    bump.set(now_ms());
    Ok(())
}
