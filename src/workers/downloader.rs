//! Download worker — a placeholder background job.
//!
//! Background workers do work *outside* the normal request/response cycle: a
//! controller can enqueue a job and return immediately, while the worker runs
//! the slow part later. This one is scaffolding — its `perform` body is empty —
//! kept as a working example of the worker shape to copy for real jobs.

use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

// Holds whatever the worker needs to do its job. `ctx` gives it access to the
// database, config, etc.
pub struct DownloadWorker {
    pub ctx: AppContext,
}

// The input data for one job run. It's serializable so it can be stored on the
// queue and read back when the job actually runs.
#[derive(Deserialize, Debug, Serialize)]
pub struct DownloadWorkerArgs {
    pub user_guid: String,
}

// Wires this struct up as a loco background worker that accepts the args above.
#[async_trait]
impl BackgroundWorker<DownloadWorkerArgs> for DownloadWorker {
    // How to construct the worker (loco calls this when registering it in app.rs).
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
    // The actual job. Called once per queued task. `_args` is unused for now
    // (the leading underscore tells the compiler that's intentional).
    async fn perform(&self, _args: DownloadWorkerArgs) -> Result<()> {
        // TODO: Some actual work goes here...

        Ok(())
    }
}
