pub mod worker;

use futures::FutureExt;
use lunaris_api::render::RawImage;
use lunaris_ecs::{bevy_ecs, prelude::*};

// --- Components for Render Job Lifecycle ---

/// A component that signals a request to render an entity.
#[derive(Component, Clone)]
pub struct RenderRequest {
    /// Specific frame to render
    frame: u64,
}

/// A component to hold the final output of a completed render.
#[derive(Component)]
pub struct RenderOutput {
    pub image: RawImage,
}

use futures::future::BoxFuture;
use lunaris_api::request::{AsyncJob, DynOrchestrator, Job, Priority};
use lunaris_api::util::error::Result;
use lunaris_ecs::Resource;

use self::worker::{SchedulerConfig, WorkerPool};

#[derive(Resource)]
pub struct Orchestrator {
    scheduler: WorkerPool,
}

impl Default for Orchestrator {
    fn default() -> Self {
        let parallelism = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let cfg = SchedulerConfig::balanced(parallelism);
        Orchestrator {
            scheduler: WorkerPool::new(cfg),
        }
    }
}

impl Orchestrator {
    pub fn submit_job<T: FnOnce() + Send + 'static>(&self, job: Job<T>) -> Result {
        self.scheduler.add_job(job)
    }
    pub fn submit_async<F, Fut>(&self, job: AsyncJob<F, Fut>) -> Result
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: core::future::Future<Output = ()> + Send + 'static,
    {
        self.scheduler.add_job_async(job)
    }
    pub fn join_foreground(&self) -> Result {
        self.scheduler.join_sync()
    }
    /// Not reccomended. bg threads don't have an obligation to join.
    pub fn join_all(&self) -> Result {
        self.scheduler.join_all()
    }
    /// reconfigure amount of threads available at runtime
    pub fn set_threads(&self, default: usize, frame: usize, background: usize) {
        self.scheduler
            .reconfigure_threads(default, frame, background)
    }
}

impl DynOrchestrator for Orchestrator {
    fn submit_job_boxed(
        &self,
        job: Box<dyn FnOnce() + Send + 'static>,
        priority: Priority,
    ) -> lunaris_api::util::error::Result {
        self.submit_job(Job {
            inner: move || (job)(),
            priority,
        })
    }
    fn submit_async_boxed(
        &self,
        fut: BoxFuture<'static, ()>,
        priority: Priority,
    ) -> lunaris_api::util::error::Result {
        self.submit_async(AsyncJob::new(|| fut).with_priority(priority))
    }
    fn join_foreground(&self) -> lunaris_api::util::error::Result {
        Orchestrator::join_foreground(self)
    }
    fn set_threads(&self, default: usize, frame: usize, background: usize) {
        Orchestrator::set_threads(self, default, frame, background)
    }
    fn profile(&self) -> lunaris_api::request::OrchestratorProfile {
        self.scheduler.profile()
    }
}
