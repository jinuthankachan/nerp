//! Migrator — the ordered list of all database migrations.
//!
//! loco/SeaORM ask this `Migrator` which migrations exist and in what order, so
//! it can apply any that haven't run yet. Add new migrations to the list below.

#![allow(elided_lifetimes_in_paths)] // relax two lint warnings that
#![allow(clippy::wildcard_imports)] //   the generated code would otherwise trip
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users; // the "create users table" migration

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    // Return every migration, oldest first. They run top-to-bottom.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
