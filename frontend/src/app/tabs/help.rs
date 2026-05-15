//! Help-tab content. Pure static text — no callbacks, no inventory
//! references; just the in-app reference card for new players.
//!
//! The visible copy is pulled from `i18n::HelpBody` so the whole tab
//! mirrors the user's selected locale. Inline `<strong>` markers are
//! intentionally not preserved across translation: doing so would
//! require shipping HTML through `dangerously_set_inner_html`, which
//! isn't worth it for a printed reference.

use yew::prelude::*;

use crate::app::i18n::{Locale, MessageId};

pub fn render_help_tab(locale: Locale) -> Html {
    let body = locale.help_body();
    html! {
        <>
            <section class="panel help">
                <h2>{ locale.tr(MessageId::PanelHowToPlay) }</h2>

                <h3>{ locale.tr(MessageId::HelpTheLoop) }</h3>
                <p>{ body.loop_p1 }</p>
                <p>{ body.loop_p2 }</p>

                <h3>{ locale.tr(MessageId::HelpStats) }</h3>
                <p>{ body.stats_p1 }</p>
                <p>{ body.stats_p2 }</p>

                <h3>{ locale.tr(MessageId::HelpFormsTransformation) }</h3>
                <p>{ body.forms_p1 }</p>

                <h3>{ locale.tr(MessageId::HelpTabs) }</h3>
                <ul class="help-tab-list">
                    { for body.tabs.iter().map(|line| html! { <li>{ *line }</li> }) }
                </ul>

                <h3>{ locale.tr(MessageId::HelpShopGear) }</h3>
                <p>{ body.shop_p1 }</p>
                <p>{ body.shop_p2 }</p>

                <h3>{ locale.tr(MessageId::HelpConsumables) }</h3>
                <p>{ body.consumables_p1 }</p>

                <h3>{ locale.tr(MessageId::HelpWorldBoss) }</h3>
                <p>{ body.world_boss_p1 }</p>

                <h3>{ locale.tr(MessageId::HelpDelegateWhat) }</h3>
                <p>{ body.delegate_p1 }</p>
                <p>{ body.delegate_p2 }</p>

                <h3>{ locale.tr(MessageId::HelpGuildsMailbox) }</h3>
                <p>{ body.guilds_p1 }</p>
                <p>{ body.guilds_p2 }</p>
            </section>
        </>
    }
}
