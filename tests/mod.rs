//! Integration test suite root.
//!
//! Files under `tests/` are compiled and run by `cargo test`. This file lists
//! the test sub-modules; each line pulls in one folder of tests.

mod models; // tests that exercise the model layer directly (DB logic)
mod requests; // tests that send real HTTP requests to the running app
mod tasks; // (placeholder) tests for CLI tasks
mod workers; // (placeholder) tests for background workers
