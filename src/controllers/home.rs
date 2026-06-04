//! Home controller — serves the public, server-rendered web pages.
//!
//! A "controller" is a group of functions ("handlers") that each answer one
//! kind of HTTP request. These two render HTML (unlike the auth controller,
//! which speaks JSON).

use axum::response::Html; // a wrapper that tells the browser "this is HTML"
use loco_rs::prelude::*; // common loco types/helpers (Routes, Response, get, …)

/// Render the nERP home page (server-side rendered Tabler + htmx shell).
///
/// `ViewEngine(v): ViewEngine<TeraView>` is loco "extracting" the template
/// engine for us — `v` is the thing that turns a template + data into HTML.
/// The `async` keyword means this function can pause while doing work; loco
/// `.await`s it. It returns a `Result<Response>`: either an HTTP response, or
/// an error loco will turn into an error page.
async fn index(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    // Render the template at assets/views/home/hello.html. `data!({})` passes an
    // empty set of variables — this page needs no dynamic data yet.
    format::render().view(&v, "home/hello.html", data!({}))
}

/// Minimal htmx round-trip target: returns an HTML fragment that the home page
/// swaps in, proving the htmx wiring works end-to-end.
///
/// `&'static str` is a string baked into the program that lives for the whole
/// run. We return a *fragment* (not a full page) because htmx drops it straight
/// into the existing page.
async fn htmx_probe() -> Html<&'static str> {
    Html(
        // `r#"..."#` is a "raw string": quotes and backslashes inside need no
        // escaping, which is handy for HTML attributes.
        r#"<span class="text-green"><i class="ti ti-circle-check me-1"></i>htmx is working — this fragment came from the server.</span>"#,
    )
}

/// Map URL paths to the handler functions above. loco calls this from
/// `App::routes` (in src/app.rs) to learn which code answers which URL.
pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(index)) // GET /            → render the home page
        .add("/htmx-probe", get(htmx_probe)) // GET /htmx-probe  → return the fragment
}
