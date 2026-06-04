//! View-engine initializer — sets up the Tera HTML template engine, with
//! translations (i18n) layered on when locale files are present.
//!
//! An "initializer" runs once during boot. This one builds the template engine
//! and attaches it to the app so controllers (like `home::index`) can render
//! pages. It also wires up Fluent translations so templates can call a `t(...)`
//! function to look up translated strings.

use async_trait::async_trait;
use axum::{Extension, Router as AxumRouter};
use fluent_templates::{ArcLoader, FluentLoader}; // the translation machinery
use loco_rs::{
    app::{AppContext, Initializer},
    controller::views::{engines, ViewEngine},
    Error, Result,
};
use tracing::info;

// Where the translation files live. `.ftl` = Fluent translation format.
const I18N_DIR: &str = "assets/i18n";
const I18N_SHARED: &str = "assets/i18n/shared.ftl";

#[allow(clippy::module_name_repetitions)]
pub struct ViewEngineInitializer;

#[async_trait]
impl Initializer for ViewEngineInitializer {
    // A label loco uses to identify this initializer in logs.
    fn name(&self) -> String {
        "view-engine".to_string()
    }

    // Runs after routes are set up. It returns the (possibly modified) router
    // with the template engine attached as a shared "extension".
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        // Only enable translations if the locale folder actually exists.
        let tera_engine = if std::path::Path::new(I18N_DIR).exists() {
            // Load every locale, defaulting to en-US, sharing common strings
            // from shared.ftl. `Arc` lets many threads share the loaded data
            // safely. `set_use_isolating(false)` drops the invisible Unicode
            // markers Fluent normally wraps variables in.
            let arc = std::sync::Arc::new(
                ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
                    .shared_resources(Some(&[I18N_SHARED.into()]))
                    .customize(|bundle| bundle.set_use_isolating(false))
                    .build()
                    // Turn any loader error into loco's error type.
                    .map_err(|e| Error::string(&e.to_string()))?,
            );
            info!("locales loaded");

            // Build Tera, then register a `t` function templates can call to
            // translate a key. `move` hands the closure its own copy of `arc`.
            engines::TeraView::build()?.post_process(move |tera| {
                tera.register_function("t", FluentLoader::new(arc.clone()));
                Ok(())
            })?
        } else {
            // No locales on disk → plain Tera with no translation function.
            engines::TeraView::build()?
        };

        // Attach the engine to the router so handlers can extract and use it.
        Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
    }
}
