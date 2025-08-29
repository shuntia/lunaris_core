use std::sync::{Arc, Condvar, LazyLock, Mutex, atomic::AtomicU64};

use futures::{future::BoxFuture, stream::FuturesUnordered};
use lunaris_api::{
    request::{AsyncJob, Job, Priority},
    utils::errors::NResult,
};
use rayon::ThreadPool;

pub static JOBS: AtomicU64 = AtomicU64::new(0);
pub static ZERO_NOTIFY: LazyLock<Arc<(Mutex<()>, Condvar)>> =
    LazyLock::new(|| Arc::new((Mutex::new(()), Condvar::new())));
pub static BG_JOBS: AtomicU64 = AtomicU64::new(0);

pub struct WorkerPool<F: Future<Output = ()> + Send + 'static> {
    frame_threadpool: ThreadPool,
    default_threadpool: ThreadPool,
    bg_threadpool: ThreadPool,
    async_default_pool: FuturesUnordered<F>,
    async_immediate_pool: FuturesUnordered<F>,
    async_bg_pool: FuturesUnordered<F>,
    async_frame_pool: FuturesUnordered<F>,
}

impl<F: Future<Output = ()> + Send + 'static> WorkerPool<F> {
    pub fn add_job_async<A, Fut>(&self, job: AsyncJob<A>) -> NResult
    where
        A: FnOnce() -> F + Send + 'static + AsyncFnOnce() -> (),
    {
        match job.priority {
            Priority::Normal => &self.async_default_pool,
            Priority::Deferred => &self.async_default_pool,
            Priority::Immediate => &self.async_immediate_pool,
            Priority::VideoFrame => &self.async_frame_pool,
            Priority::Background => &self.async_bg_pool,
        }
        .push((job.inner)());
        Ok(())
    }
    pub fn add_job<T: FnOnce() -> () + Send + 'static>(&self, job: Job<T>) -> NResult {
        match job.priority {
            Priority::Normal => &self.default_threadpool,
            Priority::Deferred => &self.default_threadpool,
            Priority::Immediate => &self.default_threadpool,
            Priority::VideoFrame => &self.frame_threadpool,
            Priority::Background => &self.bg_threadpool,
        }
        .spawn(move || {
            if let Priority::Background = job.priority {
                BG_JOBS.fetch_add(1, std::sync::atomic::Ordering::Release);
            } else {
                JOBS.fetch_add(1, std::sync::atomic::Ordering::Release);
            }
            let priority = job.priority;
            job.exec();
            if let Priority::Background = priority {
                BG_JOBS.fetch_sub(1, std::sync::atomic::Ordering::Release);
            } else {
                if JOBS.fetch_sub(1, std::sync::atomic::Ordering::Release) == 0 {
                    ZERO_NOTIFY.1.notify_all();
                }
            }
        });
        Ok(())
    }
    pub fn join_sync(&self) -> NResult {
        ZERO_NOTIFY.1.wait(ZERO_NOTIFY.0.lock().unwrap()).unwrap();
        Ok(())
    }
}
