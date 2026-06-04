//! User model — the business logic for the `users` table.
//!
//! loco/SeaORM split a database table into two shapes:
//!   * `Model`       — a read-only snapshot of a row you fetched.
//!   * `ActiveModel`  — an editable version used to insert or update a row.
//! The raw, auto-generated definitions live in `_entities/users.rs`; this file
//! adds the *behavior* — validation, lookups, password handling, tokens — by
//! attaching extra methods to those types via `impl` blocks.

use async_trait::async_trait;
use chrono::{offset::Local, Duration}; // working with dates/times
use loco_rs::{auth::jwt, hash, prelude::*}; // JWT creation + password hashing helpers
use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid; // for generating random unique IDs

// Re-export the generated entity types so the rest of the app can write
// `models::users::Model` instead of reaching into the `_entities` folder.
pub use super::_entities::users::{self, ActiveModel, Entity, Model};

// Tunable constants for the magic-link feature.
pub const MAGIC_LINK_LENGTH: i8 = 32; // characters in a magic-link token
pub const MAGIC_LINK_EXPIRATION_MIN: i8 = 5; // how many minutes the link stays valid

/// Body of `POST /api/auth/login` — what a client must send to log in.
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginParams {
    pub email: String,
    pub password: String,
}

/// Body of `POST /api/auth/register` — what a client must send to sign up.
#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterParams {
    pub email: String,
    pub password: String,
    pub name: String,
}

/// Rules a user must satisfy before being saved. The `#[validate(...)]`
/// attributes come from the `validator` crate and are checked automatically.
#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
    pub name: String,
    #[validate(email(message = "invalid email"))]
    pub email: String,
}

// Teaches an `ActiveModel` how to produce its validation rules. loco calls this
// to know *what* to validate; we copy the fields into a `Validator` and hand it
// back. `.as_ref().to_owned()` pulls the current value out and clones it.
impl Validatable for ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            name: self.name.as_ref().to_owned(),
            email: self.email.as_ref().to_owned(),
        })
    }
}

// `ActiveModelBehavior` lets us hook into the save lifecycle. `before_save`
// runs automatically every time a user row is about to be inserted or updated.
#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::users::ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        // Always run validation first; an invalid model never reaches the DB.
        self.validate()?;
        // `insert` is true only for brand-new rows. For those, assign a public
        // id (`pid`, a random UUID we expose to the world instead of the
        // numeric primary key) and a random API key.
        if insert {
            let mut this = self;
            this.pid = ActiveValue::Set(Uuid::new_v4());
            this.api_key = ActiveValue::Set(format!("lo-{}", Uuid::new_v4()));
            Ok(this)
        } else {
            // For updates, leave those identifiers untouched.
            Ok(self)
        }
    }
}

// Implements loco's `Authenticable` trait so the framework can find a user when
// authenticating an incoming request.
#[async_trait]
impl Authenticable for Model {
    // Look up a user by their API key (used for API-token auth).
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ApiKey, api_key)
                    .build(),
            )
            .one(db)
            .await?;
        // `.one(db)` returns `Option` (maybe a row, maybe none). Turn "none"
        // into a typed "not found" error so callers can handle it uniformly.
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    // For JWT auth, the token carries the user's `pid`, so reuse find_by_pid.
    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        Self::find_by_pid(db, claims_key).await
    }
}

// Read-only helpers that look users up in various ways. These live on `Model`
// because they return a fetched row. They all follow the same pattern: build a
// filtered query, fetch at most one row, and convert "no row" into an error.
impl Model {
    /// finds a user by the provided email
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Email, email) // WHERE email = ?
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided verification token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_verification_token(
        db: &DatabaseConnection,
        token: &str,
    ) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::EmailVerificationToken, token)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the magic token and verify and token expiration
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error ot token expired
    pub async fn find_by_magic_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        // Step 1: find the row that has this magic-link token.
        let user = users::Entity::find()
            .filter(
                query::condition()
                    .eq(users::Column::MagicLinkToken, token)
                    .build(),
            )
            .one(db)
            .await?;

        // Step 2: there must actually be such a user.
        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;
        // Step 3: the token must carry an expiry, and that expiry must be in the
        // future. Otherwise the link is treated as invalid/expired.
        if let Some(expired_at) = user.magic_link_expiration {
            if expired_at >= Local::now() {
                Ok(user)
            } else {
                tracing::debug!(
                    user_pid = user.pid.to_string(),
                    token_expiration = expired_at.to_string(),
                    "magic token expired for the user."
                );
                Err(ModelError::msg("magic token expired"))
            }
        } else {
            // No expiry stored at all — shouldn't happen, so log it as an error.
            tracing::error!(
                user_pid = user.pid.to_string(),
                "magic link expiration time not exists"
            );
            Err(ModelError::msg("expiration token not exists"))
        }
    }

    /// finds a user by the provided reset token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_reset_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ResetToken, token)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided pid
    ///
    /// # Errors
    ///
    /// When could not find user  or DB query error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        // `pid` arrives as text; parse it into a real UUID first (rejecting
        // anything that isn't a valid UUID) before querying.
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Pid, parse_uuid)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided api key
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ApiKey, api_key)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Verifies whether the provided plain password matches the hashed password
    ///
    /// # Errors
    ///
    /// when could not verify password
    #[must_use]
    pub fn verify_password(&self, password: &str) -> bool {
        // Passwords are stored only as one-way hashes. This re-hashes the
        // submitted password the same way and checks it matches the stored one.
        hash::verify_password(password, &self.password)
    }

    /// Asynchronously creates a user with a password and saves it to the
    /// database.
    ///
    /// # Errors
    ///
    /// When could not save the user into the DB
    pub async fn create_with_password(
        db: &DatabaseConnection,
        params: &RegisterParams,
    ) -> ModelResult<Self> {
        // Open a transaction so the "check email is free" and "insert user"
        // steps happen atomically — either both succeed or neither does.
        let txn = db.begin().await?;

        // Refuse to create a second account with the same email.
        if users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Email, &params.email)
                    .build(),
            )
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists {});
        }

        // Never store the raw password — hash it first.
        let password_hash =
            hash::hash_password(&params.password).map_err(|e| ModelError::Any(e.into()))?;
        // Build the new row. `..Default::default()` fills every field we didn't
        // set (tokens, timestamps, …) with its default/empty value.
        let user = users::ActiveModel {
            email: ActiveValue::set(params.email.to_string()),
            password: ActiveValue::set(password_hash),
            name: ActiveValue::set(params.name.to_string()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // Commit the transaction to make the insert permanent.
        txn.commit().await?;

        Ok(user)
    }

    /// Creates a JWT
    ///
    /// # Errors
    ///
    /// when could not convert user claims to jwt token
    pub fn generate_jwt(&self, secret: &str, expiration: u64) -> ModelResult<String> {
        // Build a signed token that encodes this user's `pid` and an expiry.
        // `Map::new()` is an empty set of extra custom claims.
        jwt::JWT::new(secret)
            .generate_token(expiration, self.pid.to_string(), Map::new())
            .map_err(ModelError::from)
    }
}

// Write helpers that *modify* a user. These live on `ActiveModel` (the editable
// shape). Each takes `mut self` (it consumes and edits the model), sets some
// fields, saves with `.update(db)`, and returns the freshly-saved `Model`.
impl ActiveModel {
    /// Sets the email verification information for the user and
    /// updates it in the database.
    ///
    /// This method is used to record the timestamp when the email verification
    /// was sent and generate a unique verification token for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_email_verification_sent(
        mut self,
        db: &DatabaseConnection,
    ) -> ModelResult<Model> {
        // Record "sent just now" and a fresh random token to put in the email.
        self.email_verification_sent_at = ActiveValue::set(Some(Local::now().into()));
        self.email_verification_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Sets the information for a reset password request,
    /// generates a unique reset password token, and updates it in the
    /// database.
    ///
    /// This method records the timestamp when the reset password token is sent
    /// and generates a unique token for the user.
    ///
    /// # Arguments
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_forgot_password_sent(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.reset_sent_at = ActiveValue::set(Some(Local::now().into()));
        self.reset_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Records the verification time when a user verifies their
    /// email and updates it in the database.
    ///
    /// This method sets the timestamp when the user successfully verifies their
    /// email.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn verified(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        // Stamp the moment the email was confirmed; its presence means "verified".
        self.email_verified_at = ActiveValue::set(Some(Local::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Resets the current user password with a new password and
    /// updates it in the database.
    ///
    /// This method hashes the provided password and sets it as the new password
    /// for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error or could not hashed the given password
    pub async fn reset_password(
        mut self,
        db: &DatabaseConnection,
        password: &str,
    ) -> ModelResult<Model> {
        // Store the new password as a hash, then clear the reset token so the
        // same reset link can't be used twice.
        self.password =
            ActiveValue::set(hash::hash_password(password).map_err(|e| ModelError::Any(e.into()))?);
        self.reset_token = ActiveValue::Set(None);
        self.reset_sent_at = ActiveValue::Set(None);
        self.update(db).await.map_err(ModelError::from)
    }

    /// Creates a magic link token for passwordless authentication.
    ///
    /// Generates a random token with a specified length and sets an expiration time
    /// for the magic link. This method is used to initiate the magic link authentication flow.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn create_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        // A random secret string for the link, plus an expiry a few minutes out.
        let random_str = hash::random_string(MAGIC_LINK_LENGTH as usize);
        let expired = Local::now() + Duration::minutes(MAGIC_LINK_EXPIRATION_MIN.into());

        self.magic_link_token = ActiveValue::set(Some(random_str));
        self.magic_link_expiration = ActiveValue::set(Some(expired.into()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Verifies and invalidates the magic link after successful authentication.
    ///
    /// Clears the magic link token and expiration time after the user has
    /// successfully authenticated using the magic link.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn clear_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        // Wipe both fields so the consumed link can never be reused.
        self.magic_link_token = ActiveValue::set(None);
        self.magic_link_expiration = ActiveValue::set(None);
        self.update(db).await.map_err(ModelError::from)
    }
}
