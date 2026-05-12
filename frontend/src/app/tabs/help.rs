//! Help-tab content. Pure static text — no callbacks, no inventory
//! references; just the in-app reference card for new players.

use shared::{
    ENCOUNTERS_PER_MISSION, FIREBALL_BOSS_DAMAGE, FIREBALL_PRICE, FORGE_COUNT,
    MISSION_DAMAGE, POTION_PRICE,
};
use yew::prelude::*;

pub fn render_help_tab() -> Html {
    html! {
        <>
            <section class="panel help">
                <h2>{ "how to play" }</h2>
                <h3>{ "the loop" }</h3>
                <p>
                    { "Click " }<strong>{ "Run Mission" }</strong>
                    { format!(" on the Farm tab to start a chain of up to {} encounters against the current area's enemies. Combat is tick-based: one turn fires every {} ms, and you can queue " ,
                        ENCOUNTERS_PER_MISSION, shared::TURN_COOLDOWN_MS) }
                    <strong>{ "Use Potion" }</strong>
                    { " or " }
                    <strong>{ "Use Fireball" }</strong>
                    { " mid-fight to react to a bad streak. Each win grants gold, essence, XP, and chips the shared World Boss. Lose, and you transform into the enemy that beat you." }
                </p>
                <p>
                    { "Toggle " }<strong>{ "auto: on" }</strong>
                    { " and the node-side delegate keeps running missions even after you close the tab. You'll return to a " }
                    <strong>{ "while you were away" }</strong>
                    { " banner summing up what happened (capped at ~1 hour of catch-up). Auto pauses when HP drops below the threshold you pick in " }
                    <strong>{ "Settings → auto-mission" }</strong>
                    { "." }
                </p>

                <h3>{ "stats" }</h3>
                <p>
                    { "Your " }<strong>{ "Level" }</strong>
                    { " comes from cumulative " }<strong>{ "XP" }</strong>
                    { ". XP per level rises 1.5× each step (100, 150, 225, 337, …). Base stats are static per level: HP = 20 + lvl×5, Attack = 5 + lvl×2, Defence = 5 + lvl×2. " }
                    <strong>{ "Equipment, form, and skills add on top" }</strong>
                    { "; nothing else (no gold/essence bleed-through)." }
                </p>
                <p>
                    <strong>{ "HP" }</strong>
                    { format!(" depletes in combat and regenerates over time (full regen in {}s of real time). Use a Potion to instantly fill it.", shared::HP_FULL_REGEN_MS / 1000) }
                </p>

                <h3>{ "forms & transformation" }</h3>
                <p>
                    { "Losing a combat to a non-mundane enemy " }
                    <strong>{ "permanently transforms you" }</strong>
                    { " into that monster. Each form has its own equipped-slot mask: a Slime can only wear Helm + Ring, a Cat keeps Helm/Cloak/Boots/Ring, etc. Stats shift to match the form. " }
                    <strong>{ "Every form you've touched leaves a permanent Skill" }</strong>
                    { " — even after you change back to Human, those bonuses carry. This is the prestige loop." }
                </p>

                <h3>{ "tabs" }</h3>
                <ul class="help-tab-list">
                    <li><strong>{ "🛡 Farm" }</strong>{ " — your hero, the live combat scene (HP bars, queue-action buttons), plot, World Boss, raw resources." }</li>
                    <li><strong>{ "🗺 World Map" }</strong>{ " — switch farming areas. Higher areas have a level gate but pay more (or differently — Forest is essence-rich, Mountain is gold-rich, Boss's Lair is damage-heavy)." }</li>
                    <li>
                        <strong>{ "🛒 Shop" }</strong>
                        { " — buy potions/fireballs, buy pre-rolled gear by slot+tier, sell stash items, forge 3-of-a-kind into the next tier, and Work the Farm (wheat → gold at 10:1)." }
                    </li>
                    <li><strong>{ "⚔ Guilds" }</strong>{ " — create or join a cooperative group. Membership is exclusive (one pubkey, one guild); leader auto-passes when they leave." }</li>
                    <li><strong>{ "🏆 Achievements" }</strong>{ " — milestones, skills you've unlocked, forms you've been, World Boss progress, leaderboard." }</li>
                    <li><strong>{ "⚙ Settings" }</strong>{ " — themes, sync cadence, auto-mission HP threshold, identity export / progress reset, advanced toggles + mailbox D2D test + debug overlay." }</li>
                    <li><strong>{ "❔ Help" }</strong>{ " — this page." }</li>
                </ul>

                <h3>{ "shop & gear" }</h3>
                <p>
                    { "Gear is grouped by 8 slots × 4 tiers. Tiers 1-3 are buyable (100g/250g/600g); Tier 4 (Legendary) only drops or forges. " }
                    <strong>{ "Forge" }</strong>
                    { format!(" needs {} copies of one item + tier-scaled essence to combine into a single piece of the next tier — your duplicates aren't trash, they're future legendaries.", FORGE_COUNT) }
                </p>
                <p>
                    <strong>{ "Auto-Equip Best" }</strong>
                    { " walks every form-allowed slot and equips the highest stat-sum unequipped piece you own." }
                </p>

                <h3>{ "consumables" }</h3>
                <p>
                    { format!("Potions (cost {}g) heal your HP to full. Fireballs (cost {}g) deal {} flat damage to the shared World Boss. Drop rates: a potion every 13 wins, a fireball every 19.", POTION_PRICE, FIREBALL_PRICE, FIREBALL_BOSS_DAMAGE) }
                </p>

                <h3>{ "world boss" }</h3>
                <p>
                    { format!("The World Boss has 500 HP in era 0, shared across every player connected to the contract. Each win contributes 1× to {}× the base (area-dependent). Once cumulative damage exceeds the era's HP, the boss respawns in the next era — bigger ({}×, {}×, …). Era scaling makes the gauge keep moving once dozens of players have chipped at it forever.", MISSION_DAMAGE * 5, 4, 9) }
                </p>

                <h3>{ "what does the delegate do?" }</h3>
                <p>
                    { "Everything that matters lives on your Freenet node, not in this browser tab. The delegate stores your Ed25519 identity, your inventory, every gear piece, every skill, your XP, your wheat, your shop counter, and your achievements — plus the active battle state so closing the tab pauses (not aborts) a fight. The browser is just a thin view." }
                </p>
                <p>
                    { "To move identity to another node, use " }
                    <strong>{ "Settings → Export seed" }</strong>
                    { " — it returns the 32-byte secret key; copy it once, paste on the new node. " }
                    <strong>{ "Reset progress" }</strong>
                    { " wipes the inventory (gold, gear, skills, achievements) but " }
                    <strong>{ "keeps the pubkey" }</strong>
                    { " — leaderboards still recognize you. To actually destroy the identity, wipe `~/.config/freenet/secrets/local/<delegate-key>/`." }
                </p>

                <h3>{ "guilds & mailbox (early)" }</h3>
                <p>
                    { "The " }
                    <strong>{ "Guilds" }</strong>
                    { " tab is the first cross-player interaction beyond the leaderboard: cooperative groups, one pubkey per guild, 50 members per guild. Create a new one or join existing — leader auto-handoff on leave. Gameplay layers (shared boss, member contributions) come later." }
                </p>
                <p>
                    { "The " }
                    <strong>{ "Mailbox" }</strong>
                    { " section in Settings → Advanced is the signed-log substrate for player-to-player messaging — gifts, invites, trade offers will plug in on top. Send a self-test message to verify the round-trip is working." }
                </p>
            </section>
        </>
    }
}
