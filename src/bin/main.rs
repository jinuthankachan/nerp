//! Program entry point — the binary that gets run when you start nERP.
//!
//! In Rust, a `main` function is where execution begins. This file is tiny on
//! purpose: it hands everything over to the loco framework, which knows how to
//! parse command-line arguments (`start`, `db migrate`, …) and boot the app.

// Pull in the pieces we need from other crates (a "crate" is a Rust package):
use loco_rs::cli; // loco's command-line interface (parses `start`, `db`, `task`, …)
use migration::Migrator; // our database migrations (see the `migration/` folder)
use nerp::app::App; // our application definition (see src/app.rs)

// `#[tokio::main]` turns this plain `main` into an async runtime. Tokio is the
// engine that lets the server handle many requests at once without blocking.
#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    // Hand control to loco's CLI. It reads the command you typed (e.g.
    // `cargo loco start`), wires up our `App` and `Migrator`, and runs it.
    // The trailing `.await` waits for that async work to finish.
    // Returning the `Result` means: if anything fails, the process exits
    // with an error instead of pretending it succeeded.
    cli::main::<App, Migrator>().await
}
