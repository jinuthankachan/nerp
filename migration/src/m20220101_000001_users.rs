//! Migration: create the `users` table.
//!
//! A "migration" is a versioned, repeatable change to the database schema.
//! Each migration has an `up` (apply the change) and a `down` (undo it). The
//! filename's timestamp prefix sets the order migrations run in. loco runs any
//! pending migrations on startup in development.

use loco_rs::schema::*; // helpers like `create_table`, `drop_table`, `ColType`
use sea_orm_migration::prelude::*;

// `DeriveMigrationName` derives this migration's unique name from the filename.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Apply the migration: create the `users` table with all its columns. Each
    // tuple is (column name, column type). `ColType` describes the SQL type:
    //   PkAuto                       = auto-incrementing integer primary key
    //   Uuid / String                = a UUID / a required text column
    //   StringUniq                   = required text that must be unique
    //   StringNull                   = optional text (may be NULL/empty)
    //   TimestampWithTimeZoneNull    = optional date-time
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "users",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::Uuid),
                ("email", ColType::StringUniq),
                ("password", ColType::String),
                ("api_key", ColType::StringUniq),
                ("name", ColType::String),
                ("reset_token", ColType::StringNull),
                ("reset_sent_at", ColType::TimestampWithTimeZoneNull),
                ("email_verification_token", ColType::StringNull),
                (
                    "email_verification_sent_at",
                    ColType::TimestampWithTimeZoneNull,
                ),
                ("email_verified_at", ColType::TimestampWithTimeZoneNull),
                ("magic_link_token", ColType::StringNull),
                ("magic_link_expiration", ColType::TimestampWithTimeZoneNull),
            ],
            &[], // no foreign keys to other tables
        )
        .await?;
        Ok(())
    }

    // Undo the migration: drop the whole `users` table. Used when rolling back.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "users").await?;
        Ok(())
    }
}
