//! Application definition — the central place where nERP tells the loco
//! framework how it is wired together.
//!
//! loco calls the functions in this file at well-defined moments (when it
//! boots, when it sets up routes, when it connects background workers, …). We
//! implement loco's `Hooks` "trait" to plug our app into those moments. A
//! "trait" in Rust is like an interface: a list of functions a type promises to
//! provide. By implementing `Hooks for App`, we fill in those blanks.

use async_trait::async_trait;
// Bring in the specific loco building blocks this file needs.
use loco_rs::{
    app::{AppContext, Hooks, Initializer}, // app context + the hooks we implement
    bgworker::{BackgroundWorker, Queue},   // background job machinery
    boot::{create_app, BootResult, StartMode}, // startup helpers
    config::Config,                        // parsed config/*.yaml settings
    controller::AppRoutes,                 // the collection of URL routes
    db::{self, truncate_table},            // database helpers (seeding, wiping)
    environment::Environment,              // which env we're in (dev/test/prod)
    task::Tasks,                           // registry of CLI tasks
    Result,                                // loco's Result type (success or error)
};
use migration::Migrator; // our DB migrations, so loco can run them on boot
use std::path::Path; // for filesystem paths (used by `seed` below)

// Pull in our own modules. `#[allow(unused_imports)]` silences the compiler
// warning for any of these that aren't referenced yet — loco's scaffold imports
// them up front so new features have them ready to use.
#[allow(unused_imports)]
use crate::{
    controllers, initializers, models::_entities::users, tasks, workers::downloader::DownloadWorker,
};

// `App` is an empty struct — it holds no data. It exists only as a "name" to
// hang all the framework hooks below onto.
pub struct App;

// `#[async_trait]` lets a trait contain `async` functions (Rust needs this
// helper for older-style async traits). Everything inside `impl Hooks for App`
// is us answering loco's questions about how the app should behave.
#[async_trait]
impl Hooks for App {
    // The app's name, taken at compile time from Cargo.toml's crate name.
    // `env!(...)` reads an environment variable *during compilation*.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    // A human-readable version string like "0.1.0 (abc1234)".
    // It tries to stamp in the git commit SHA from the build environment, and
    // falls back to "dev" when building locally without one.
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"), // version from Cargo.toml
            option_env!("BUILD_SHA") // a build-time var, may or may not exist
                .or(option_env!("GITHUB_SHA")) // …or the one CI provides
                .unwrap_or("dev")  // …or just "dev" if neither is set
        )
    }

    // Boot the application. loco hands us the start mode, the environment, and
    // the parsed config; we forward them to loco's `create_app`, telling it to
    // use *our* `App` (Self) and *our* `Migrator`. The `?`-free `.await` here
    // returns whatever `create_app` returns.
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    // Initializers are one-time setup steps run during boot. We register one:
    // the view engine (Tera + translations). It's boxed because loco stores a
    // list of differently-typed initializers behind a shared `dyn` interface.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![Box::new(
            initializers::view_engine::ViewEngineInitializer,
        )])
    }

    // Declare every URL route the app responds to. `with_default_routes()` adds
    // loco's built-ins (`/_ping`, `/_health`); then we bolt on the routes
    // defined by each controller. Order here doesn't imply priority.
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::auth::routes())
            .add_route(controllers::home::routes())
    }

    // Register background workers with the job queue. Workers run tasks
    // asynchronously, off the request thread (see src/workers/downloader.rs).
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?; // `?` = bail out on error
        Ok(()) // `Ok(())` means "succeeded, with no value to return"
    }

    // Register one-off CLI tasks here. None yet — the scaffold leaves a marker
    // comment so code generators know where to insert new ones.
    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }

    // Used by tests: empties the `users` table so each test starts clean.
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }

    // Used by tests/dev: loads starter rows from `users.yaml` into the database.
    // `base` is the folder the seed files live in; we join the filename onto it.
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string())
            .await?;
        Ok(())
    }
}
