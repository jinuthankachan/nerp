use axum::response::Html;
use loco_rs::prelude::*;

/// Render the nERP home page (server-side rendered Tabler + htmx shell).
async fn index(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "home/hello.html", data!({}))
}

/// Minimal htmx round-trip target: returns an HTML fragment that the home page
/// swaps in, proving the htmx wiring works end-to-end.
async fn htmx_probe() -> Html<&'static str> {
    Html(
        r#"<span class="text-green"><i class="ti ti-circle-check me-1"></i>htmx is working — this fragment came from the server.</span>"#,
    )
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(index))
        .add("/htmx-probe", get(htmx_probe))
}
