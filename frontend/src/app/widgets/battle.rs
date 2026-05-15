//! Live-battle widgets: combatant cards with HP bars, encounter
//! progress + queued-action notice, and the recent-turns ticker.

use shared::{form_sprite, format_si, Inventory, ENCOUNTERS_PER_MISSION};
use yew::prelude::*;

use crate::app::i18n::{Locale, MessageId};
use crate::app::i18n_shared;

/// Live battle stage — combatant cards with HP bars. Replaces the
/// static-emoji stage when a battle is in flight; the action row
/// (Run Mission / auto) is rendered separately by the caller so it
/// stays visible both idle and mid-fight.
pub fn render_battle_stage(
    locale: Locale,
    battle: &shared::BattleState,
    inv: &Inventory,
    player_max_hp: u64,
) -> Html {
    let enemy_def = shared::enemy_def(battle.enemy_id);
    let enemy_name = enemy_def.map(|e| i18n_shared::enemy_name(locale, e)).unwrap_or("?");
    let enemy_sprite = enemy_def.map(|e| e.sprite).unwrap_or("👹");
    let enemy_pct = if battle.enemy_max_hp == 0 {
        0
    } else {
        (battle.enemy_hp * 100 / battle.enemy_max_hp).min(100)
    };
    let player_pct = if player_max_hp == 0 {
        0
    } else {
        (inv.current_hp * 100 / player_max_hp).min(100)
    };
    html! {
        <div class="battle-stage">
            <div class="combatant hero">
                <div class="combatant-sprite">{ form_sprite(inv.current_form) }</div>
                <div class="combatant-name">{ locale.tr(MessageId::TermYouBattle) }</div>
                <div class="hp-bar">
                    <div class="hp-fill" style={format!("width: {player_pct}%")}></div>
                </div>
                <div class="combatant-hp muted small">
                    { format!("{} / {}", format_si(inv.current_hp), format_si(player_max_hp)) }
                </div>
            </div>
            <div class="combatant-vs">{ "⚔" }</div>
            <div class="combatant enemy">
                <div class="combatant-sprite">{ enemy_sprite }</div>
                <div class="combatant-name">{ enemy_name }</div>
                <div class="hp-bar">
                    <div class="hp-fill" style={format!("width: {enemy_pct}%")}></div>
                </div>
                <div class="combatant-hp muted small">
                    { format!("{} / {}", format_si(battle.enemy_hp), format_si(battle.enemy_max_hp)) }
                </div>
            </div>
        </div>
    }
}

/// Mid-fight queue panel — encounter progress, queued-action
/// notice, recent-turn ticker. Rendered below the action row when
/// a battle is active. **The "Use Potion / Use Fireball" buttons
/// live in the equipment panel, always in the same position** —
/// during a battle they queue, otherwise they consume directly.
pub fn render_battle_queue(
    locale: Locale,
    battle: &shared::BattleState,
    _inv: &Inventory,
) -> Html {
    let queued = match battle.queued_action {
        shared::BATTLE_ACTION_POTION => Some(locale.tr(MessageId::BattlePotionQueued)),
        shared::BATTLE_ACTION_FIREBALL => Some(locale.tr(MessageId::BattleFireballQueued)),
        _ => None,
    };
    html! { <>
        <p class="muted small">
            { locale.fmt_encounter_progress(
                battle.encounter_idx as u32 + 1,
                ENCOUNTERS_PER_MISSION,
            ) }
        </p>
        {
            if let Some(msg) = queued {
                html! { <p class="muted small">{ msg }</p> }
            } else { html! {} }
        }
        { render_battle_turns(locale, &battle.recent_turns) }
    </>}
}

pub fn render_battle_turns(locale: Locale, turns: &[shared::BattleTurn]) -> Html {
    if turns.is_empty() {
        return html! { <p class="muted small">{ locale.tr(MessageId::BattleOpeningTurn) }</p> };
    }
    html! {
        <ul class="battle-turns">
            { for turns.iter().rev().take(5).map(|t| {
                let mut bits: Vec<String> = Vec::new();
                if t.action == shared::BATTLE_ACTION_POTION { bits.push(locale.tr(MessageId::ItemPotion).to_lowercase()); }
                if t.action == shared::BATTLE_ACTION_FIREBALL { bits.push(format!("{} +{}", locale.tr(MessageId::ItemFireball).to_lowercase(), format_si(shared::FIREBALL_BOSS_DAMAGE))); }
                if t.player_dmg > 0 { bits.push(format!("{} → -{}", locale.tr(MessageId::TermYouBattle), format_si(t.player_dmg as u64))); }
                if t.enemy_dmg > 0 { bits.push(format!("{} → -{}", locale_enemy(locale), format_si(t.enemy_dmg as u64))); }
                if bits.is_empty() { bits.push(locale.tr(MessageId::BattleMissed).into()); }
                html! { <li class="battle-turn-row">{ bits.join(" · ") }</li> }
            }) }
        </ul>
    }
}

/// Generic "enemy" word for the battle turns ticker. The enemy name
/// itself (the specific monster) is locale-static in the shared crate;
/// this method translates only the role label used in turn summaries.
fn locale_enemy(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "enemy",
        Locale::Ru => "враг",
    }
}
