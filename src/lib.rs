//! Crate root — the "table of contents" for the nERP library.
//!
//! Every Rust file (module) that should be part of this app has to be declared
//! here with `pub mod <name>`. `pub` means "public", so other code (and the
//! `main.rs` binary above) can reach into these modules. Each name below maps
//! to either a `<name>.rs` file or a `<name>/mod.rs` folder.

pub mod app; // App definition: routes, workers, startup hooks (src/app.rs)
pub mod controllers; // Request handlers — the code that answers HTTP requests
pub mod data; // Place for loading static/seed data (currently empty)
pub mod initializers; // One-time setup that runs while the app boots
pub mod mailers; // Email-sending logic and templates
pub mod models; // Database tables + the business logic that operates on them
pub mod tasks; // One-off CLI commands you can run (currently none registered)
pub mod views; // Shapes of the JSON we send back (the "response" structs)
pub mod workers; // Background jobs that run outside the request/response cycle
