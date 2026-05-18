//! SVG-overlay edge drawer for `.area-graph` containers (§2.1).
//!
//! Renders an absolutely-positioned `<svg>` inside an `.area-graph`
//! that hosts `<line>` elements between each child node's
//! top-center and every parent's bottom-center. Mounted as a sibling
//! of the `.graph-row` divs.
//!
//! Mechanics:
//!   1. Renders an empty `<svg>` on first paint so the DOM has the
//!      container.
//!   2. `use_effect_with` reads bounding rects of every
//!      `.graph-node[data-area-id]` inside the same `.area-graph`
//!      ancestor, computes (parent_bottom_center → child_top_center)
//!      pairs from each node's `data-parent-ids` CSV attribute.
//!   3. Stores the resulting edge list in `use_state`, triggers a
//!      re-render where the SVG actually has `<line>` children.
//!   4. Re-measures on every prop change (caller passes a `bump`
//!      token derived from the graph identity + layout-shaping deps
//!      like the area-clears watermark).
//!
//! Pointer events disabled on the SVG so card clicks pass through.

use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct GraphEdgeOverlayProps {
    /// CSS selector that uniquely identifies the host `.area-graph`
    /// in the document. Caller stamps something like
    /// `"area-graph-linear"` / `"area-graph-wilds"` and the overlay
    /// inserts itself inside that container.
    pub host_id: AttrValue,
    /// Caller-provided dependency token — bump on inventory /
    /// area_clears / map_view changes so the overlay re-measures.
    pub bump: u64,
}

#[derive(Clone, PartialEq, Debug)]
struct EdgeSpec {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

#[derive(Clone, PartialEq, Debug, Default)]
struct OverlayLayout {
    width: f64,
    height: f64,
    edges: Vec<EdgeSpec>,
}

#[function_component(GraphEdgeOverlay)]
pub fn graph_edge_overlay(props: &GraphEdgeOverlayProps) -> Html {
    let layout = use_state(OverlayLayout::default);
    {
        let host_id = props.host_id.clone();
        let layout = layout.clone();
        let bump = props.bump;
        use_effect_with((host_id.to_string(), bump), move |(host_id, _)| {
            let layout = layout.clone();
            // measure() is sync — the DOM has just been written by
            // Yew. Use requestAnimationFrame would smooth resizes
            // but on first paint sync read is fine.
            if let Some(measured) = measure_edges(host_id) {
                if *layout != measured {
                    layout.set(measured);
                }
            }
            || ()
        });
    }
    // Window resize listener — re-measure on every viewport change
    // so SVG edges follow the wrapped card layout.
    {
        let host_id = props.host_id.clone();
        let layout = layout.clone();
        use_effect_with(host_id.to_string(), move |host_id| {
            let host_id = host_id.clone();
            let layout = layout.clone();
            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                if let Some(measured) = measure_edges(&host_id) {
                    if *layout != measured {
                        layout.set(measured);
                    }
                }
            }) as Box<dyn FnMut()>);
            let win = web_sys::window();
            if let Some(w) = &win {
                let _ = w.add_event_listener_with_callback(
                    "resize",
                    cb.as_ref().unchecked_ref(),
                );
            }
            // Leak the closure for the page lifetime — there's only
            // one overlay per host_id and they live as long as the
            // SPA does. A proper cleanup would store the closure in
            // RefCell and remove it on drop.
            cb.forget();
            || ()
        });
    }

    let w = layout.width.max(1.0);
    let h = layout.height.max(1.0);
    // Both viewBox AND width/height attributes so the SVG canvas
    // matches the scroll-content extent of `.area-graph`. With only
    // viewBox the SVG still inherits CSS width:100%/height:100% and
    // clips at the visible viewport; with concrete width/height
    // attributes the SVG renders at the full content size, which is
    // what overflow-x:auto's scroll machinery expects.
    let view_box = format!("0 0 {} {}", w, h);
    let w_str = w.to_string();
    let h_str = h.to_string();
    let style = format!("width: {w}px; height: {h}px;");
    let lines: Vec<Html> = layout
        .edges
        .iter()
        .map(|e| {
            html! {
                <line
                    x1={e.x1.to_string()}
                    y1={e.y1.to_string()}
                    x2={e.x2.to_string()}
                    y2={e.y2.to_string()}
                />
            }
        })
        .collect();
    html! {
        <svg
            class="graph-edges"
            viewBox={view_box}
            width={w_str}
            height={h_str}
            style={style}
            preserveAspectRatio="xMinYMin meet"
            xmlns="http://www.w3.org/2000/svg"
        >
            { for lines }
        </svg>
    }
}

/// Measure all `.graph-node` rects inside the `.area-graph` whose
/// container id matches `host_id`. Builds edges from each child's
/// `data-parent-ids` CSV.
fn measure_edges(host_id: &str) -> Option<OverlayLayout> {
    let win = web_sys::window()?;
    let doc = win.document()?;
    let host = doc.get_element_by_id(host_id)?;
    let host_el = host.dyn_ref::<web_sys::HtmlElement>()?;
    let host_rect = host_el.get_bounding_client_rect();
    let scroll_left = host_el.scroll_left() as f64;
    let scroll_top = host_el.scroll_top() as f64;
    let nodes = host.query_selector_all(".graph-node[data-area-id]").ok()?;
    let mut centers: std::collections::HashMap<String, (f64, f64, f64, f64)> =
        std::collections::HashMap::new();
    for i in 0..nodes.length() {
        let Some(item) = nodes.item(i) else { continue };
        let Ok(node_el) = item.dyn_into::<web_sys::HtmlElement>() else { continue };
        let id = node_el.get_attribute("data-area-id").unwrap_or_default();
        // Measure the visible `.area-card` button, not the
        // `.graph-node` flex-wrapper. The wrapper has flex-gap
        // padding around the card; using its rect makes the line
        // endpoint sit in that empty gap. On Wilds the
        // variable-height cards make this gap visually obvious —
        // some connectors look "detached". The `.area-card` rect
        // is what the player actually sees as the container.
        let card_el = node_el
            .query_selector(".area-card")
            .ok()
            .flatten()
            .and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok());
        let target = card_el.as_ref().unwrap_or(&node_el);
        let rect = target.get_bounding_client_rect();
        let left = rect.left() - host_rect.left() + scroll_left;
        let top = rect.top() - host_rect.top() + scroll_top;
        let width = rect.width();
        let height = rect.height();
        centers.insert(id, (left, top, width, height));
    }
    let mut edges: Vec<EdgeSpec> = Vec::new();
    for i in 0..nodes.length() {
        let Some(item) = nodes.item(i) else { continue };
        let Ok(el) = item.dyn_into::<web_sys::HtmlElement>() else { continue };
        let Some(parent_csv) = el.get_attribute("data-parent-ids") else { continue };
        if parent_csv.is_empty() {
            continue;
        }
        let child_id = el.get_attribute("data-area-id").unwrap_or_default();
        let Some(&(cx, cy, cw, ch)) = centers.get(&child_id) else { continue };
        for parent_id in parent_csv.split(',').filter(|s| !s.is_empty()) {
            let Some(&(px, py, pw, ph)) = centers.get(parent_id) else { continue };
            // Pick anchor points based on parent/child relative
            // position. Default top↔bottom for the common
            // top-down DAG case; switch to side connectors when
            // the parent is at the same Y-row (lateral edge in a
            // multi-parent Wilds-style graph) so the line doesn't
            // crawl diagonally through the child card.
            let parent_below_or_equal = py + ph * 0.5 >= cy + ch * 0.5;
            let (x1, y1, x2, y2) = if parent_below_or_equal {
                // Same-row or lower parent: draw side-edge from
                // parent's nearest vertical edge to child's
                // nearest vertical edge, keeping y at the
                // mid-height of each so the line lands inside
                // the cards' visible silhouettes.
                if px < cx {
                    (px + pw, py + ph * 0.5, cx, cy + ch * 0.5)
                } else {
                    (px, py + ph * 0.5, cx + cw, cy + ch * 0.5)
                }
            } else {
                // Standard vertical: parent-bottom-center →
                // child-top-center.
                (
                    px + pw / 2.0,
                    py + ph,
                    cx + cw / 2.0,
                    cy,
                )
            };
            edges.push(EdgeSpec { x1, y1, x2, y2 });
        }
    }
    // Use scroll dimensions (= full content extent) so the SVG
    // covers the whole scrollable canvas, not just the visible
    // window. clientWidth/Height would clip everything past the
    // first horizontal overflow.
    let content_width = host_el.scroll_width() as f64;
    let content_height = host_el.scroll_height() as f64;
    Some(OverlayLayout {
        width: content_width.max(host_rect.width()),
        height: content_height.max(host_rect.height()),
        edges,
    })
}
