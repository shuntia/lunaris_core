pub mod worker;

use futures::future::BoxFuture;
use lunaris_api::request::{AsyncJob, DynOrchestrator, Job, OrchestratorHandle, Priority};
use lunaris_api::util::error::NResult;

use self::worker::{SchedulerConfig, WorkerPool};

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
    pub fn submit_job<T: FnOnce() + Send + 'static>(&self, job: Job<T>) -> NResult {
        self.scheduler.add_job(job)
    }
    pub fn submit_async<F, Fut>(&self, job: AsyncJob<F, Fut>) -> NResult
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: core::future::Future<Output = ()> + Send + 'static,
    {
        self.scheduler.add_job_async(job)
    }
    pub fn join_foreground(&self) -> NResult {
        self.scheduler.join_sync()
    }
    /// Not reccomended. bg threads don't have an obligation to join.
    pub fn join_all(&self) -> NResult {
        self.scheduler.join_all()
    }
    /// reconfigure amount of threads available at runtime
    pub fn set_threads(&self, default: usize, frame: usize, background: usize) {
        self.scheduler
            .reconfigure_threads(default, frame, background)
    }
}

impl OrchestratorHandle for Orchestrator {
    fn submit_job<T: FnOnce() + Send + 'static>(&self, job: Job<T>) -> NResult {
        Orchestrator::submit_job(self, job)
    }
    fn submit_async<F, Fut>(&self, job: AsyncJob<F, Fut>) -> NResult
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: core::future::Future<Output = ()> + Send + 'static,
    {
        Orchestrator::submit_async(self, job)
    }
    fn join_foreground(&self) -> NResult {
        Orchestrator::join_foreground(self)
    }
    fn join_all(&self) -> NResult {
        Orchestrator::join_all(self)
    }
    fn set_threads(&self, default: usize, frame: usize, background: usize) {
        Orchestrator::set_threads(self, default, frame, background)
    }
}

impl DynOrchestrator for Orchestrator {
    fn submit_job_boxed(
        &self,
        job: Box<dyn FnOnce() + Send + 'static>,
        priority: Priority,
    ) -> lunaris_api::util::error::NResult {
        self.submit_job(Job {
            inner: move || (job)(),
            priority,
        })
    }
    fn submit_async_boxed(
        &self,
        fut: BoxFuture<'static, ()>,
        priority: Priority,
    ) -> lunaris_api::util::error::NResult {
        self.submit_async(AsyncJob::new(|| async move { fut.await }).with_priority(priority))
    }
    fn join_foreground(&self) -> lunaris_api::util::error::NResult {
        Orchestrator::join_foreground(self)
    }
    fn set_threads(&self, default: usize, frame: usize, background: usize) {
        Orchestrator::set_threads(self, default, frame, background)
    }
}
