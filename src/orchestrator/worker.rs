/*
Planned edits and design notes (2025-09):

- Replace rayon-dependent worker with a custom, priority-aware scheduler:
  - Separate queues for Frame, Default(Immediate/Normal/Deferred), and Background tasks.
  - Dedicated worker threads per queue group; workers prefer higher-priority sub-queues.
  - Atomic foreground/background job counters with a condvar to implement join semantics.
  - Async tasks executed on a dedicated Tokio runtime; counters wrap futures to maintain join behavior.
  - Expose dynamic reconfiguration by stopping and respawning worker threads based on SchedulerConfig.

- Naming: avoid single-letter types. Use descriptive names for structs and variables.
- Concurrency: guard against spurious wakeups; notify when counters transition to zero.
- API surface:
  - WorkerPool::new(config)
  - add_job(Job<T>) for sync closures
  - add_job_async(AsyncJob<F, Fut>) for async closures
  - join_sync() waits for foreground tasks to complete
  - join_all() waits for both foreground and background tasks
  - reconfigure_threads(default, frame, background)
*/

use parking_lot::{Condvar, Mutex};
use std::collections::VecDeque;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread::{self, JoinHandle, available_parallelism};

use lunaris_api::request::{AsyncJob, Job, OrchestratorProfile, Priority};
use lunaris_api::util::error::LunarisError;
use lunaris_api::util::error::Result;

use crossbeam_queue::ArrayQueue;

type Task = Box<dyn FnOnce() + Send + 'static>;

const FRAME_QUEUE_CAPACITY: usize = 1024;

struct PriorityQueues {
    immediate: VecDeque<Task>,
    normal: VecDeque<Task>,
    deferred: VecDeque<Task>,
}

impl PriorityQueues {
    fn new() -> Self {
        Self {
            immediate: VecDeque::new(),
            normal: VecDeque::new(),
            deferred: VecDeque::new(),
        }
    }
    fn push(&mut self, p: Priority, task: Task) {
        match p {
            Priority::Immediate => self.immediate.push_back(task),
            Priority::Normal => self.normal.push_back(task),
            Priority::Deferred => self.deferred.push_back(task),
            Priority::VideoFrame => {
                unreachable!("VideoFrame tasks are enqueued on the dedicated frame queue")
            }
            Priority::Background => {
                unreachable!("Background tasks are enqueued on the background queue")
            }
        }
    }
    fn pop(&mut self) -> Option<Task> {
        self.immediate
            .pop_front()
            .or_else(|| self.normal.pop_front())
            .or_else(|| self.deferred.pop_front())
    }
    fn is_empty(&self) -> bool {
        self.immediate.is_empty() && self.normal.is_empty() && self.deferred.is_empty()
    }
}

struct CondVarQueue<T> {
    queue: Mutex<T>,
    cv: Condvar,
}

impl<T> CondVarQueue<T> {
    fn new(inner: T) -> Self {
        Self {
            queue: Mutex::new(inner),
            cv: Condvar::new(),
        }
    }
}

struct BlockingArrayQueue<T> {
    q: ArrayQueue<T>,
    cv: Condvar,
    lock: Mutex<()>,
}

impl<T> BlockingArrayQueue<T> {
    fn with_capacity(cap: usize) -> Self {
        Self {
            q: ArrayQueue::new(cap.max(1)),
            cv: Condvar::new(),
            lock: Mutex::new(()),
        }
    }
}

pub struct SchedulerConfig {
    pub default_threads: usize,
    pub frame_threads: usize,
    pub background_threads: usize,
    pub async_threads: usize,
}

impl SchedulerConfig {
    pub fn balanced(parallelism: usize) -> Self {
        let p = parallelism.max(1);
        let background = 1usize;
        let default_threads = p.max(1);
        let frame_threads = (p / 2).max(1);
        let async_threads = p.clamp(1, 4);
        Self {
            default_threads,
            frame_threads,
            background_threads: background,
            async_threads,
        }
    }
}

pub struct WorkerPool {
    default_q: Arc<CondVarQueue<PriorityQueues>>,
    frame_q: Arc<BlockingArrayQueue<Task>>,
    bg_q: Arc<CondVarQueue<VecDeque<Task>>>,

    // Workers
    default_workers: Mutex<Vec<JoinHandle<()>>>,
    frame_workers: Mutex<Vec<JoinHandle<()>>>,
    background_workers: Mutex<Vec<JoinHandle<()>>>,

    // Counters
    fg_jobs: Arc<AtomicU64>,
    bg_jobs: Arc<AtomicU64>,
    zero_cv_lock: Arc<Mutex<()>>,
    zero_cv: Arc<Condvar>,

    // Control
    stopping: Arc<AtomicBool>,

    // Async runtime
    rt: tokio::runtime::Runtime,
}

impl WorkerPool {
    pub fn new(cfg: SchedulerConfig) -> Self {
        let pool = Self {
            default_q: Arc::new(CondVarQueue::new(PriorityQueues::new())),
            frame_q: Arc::new(BlockingArrayQueue::<Task>::with_capacity(
                FRAME_QUEUE_CAPACITY,
            )),
            bg_q: Arc::new(CondVarQueue::new(VecDeque::new())),
            default_workers: Mutex::new(Vec::new()),
            frame_workers: Mutex::new(Vec::new()),
            background_workers: Mutex::new(Vec::new()),
            fg_jobs: Arc::new(AtomicU64::new(0)),
            bg_jobs: Arc::new(AtomicU64::new(0)),
            zero_cv_lock: Arc::new(Mutex::new(())),
            zero_cv: Arc::new(Condvar::new()),
            stopping: Arc::new(AtomicBool::new(false)),
            rt: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(cfg.async_threads.max(1))
                .enable_all()
                .build()
                .expect("failed to build tokio runtime"),
        };
        pool.spawn_workers(cfg);
        pool
    }

    fn spawn_workers(&self, cfg: SchedulerConfig) {
        // Default workers: drain PriorityQueues in priority order
        let mut d = self.default_workers.lock();
        for _ in 0..cfg.default_threads.max(1) {
            let q = self.default_q.clone();
            let stopping = self.stopping.clone();
            let fg = self.fg_jobs.clone();
            let zero_cv = self.zero_cv.clone();
            let zero_lock = self.zero_cv_lock.clone();
            d.push(thread::spawn(move || {
                while !stopping.load(Ordering::Acquire) {
                    let mut guard = q.queue.lock();
                    loop {
                        if let Some(task) = guard.pop() {
                            drop(guard);
                            // Execute foreground task
                            task();
                            // Decrement foreground counter and notify if zero
                            if fg.fetch_sub(1, Ordering::AcqRel) == 1 {
                                let _g = zero_lock.lock();
                                zero_cv.notify_all();
                                drop(_g);
                            }
                            break;
                        }
                        q.cv.wait(&mut guard);
                        if stopping.load(Ordering::Acquire) {
                            break;
                        }
                    }
                }
            }));
        }
        drop(d);

        // Frame workers: drain dedicated lock-free queue
        let mut f = self.frame_workers.lock();
        for _ in 0..cfg.frame_threads.max(1) {
            let q = self.frame_q.clone();
            let stopping = self.stopping.clone();
            let fg = self.fg_jobs.clone();
            let zero_cv = self.zero_cv.clone();
            let zero_lock = self.zero_cv_lock.clone();
            f.push(thread::spawn(move || {
                while !stopping.load(Ordering::Acquire) {
                    // Fast path: try to pop without locking
                    if let Some(task) = q.q.pop() {
                        task();
                        if fg.fetch_sub(1, Ordering::AcqRel) == 1 {
                            let _g = zero_lock.lock();
                            zero_cv.notify_all();
                            drop(_g);
                        }
                        continue;
                    }
                    // Slow path: wait for signal
                    let mut guard = q.lock.lock();
                    loop {
                        if let Some(task) = q.q.pop() {
                            drop(guard);
                            task();
                            if fg.fetch_sub(1, Ordering::AcqRel) == 1 {
                                let _g = zero_lock.lock();
                                zero_cv.notify_all();
                                drop(_g);
                            }
                            break;
                        }
                        q.cv.wait(&mut guard);
                        if stopping.load(Ordering::Acquire) {
                            break;
                        }
                    }
                }
            }));
        }
        drop(f);

        // Background workers
        let mut b = self.background_workers.lock();
        for _ in 0..cfg.background_threads.max(1) {
            let q = self.bg_q.clone();
            let stopping = self.stopping.clone();
            let bg = self.bg_jobs.clone();
            let zero_cv = self.zero_cv.clone();
            let zero_lock = self.zero_cv_lock.clone();
            b.push(thread::spawn(move || {
                while !stopping.load(Ordering::Acquire) {
                    let mut guard = q.queue.lock();
                    loop {
                        if let Some(task) = guard.pop_front() {
                            drop(guard);
                            task();
                            if bg.fetch_sub(1, Ordering::AcqRel) == 1 {
                                let _g = zero_lock.lock();
                                zero_cv.notify_all();
                                drop(_g);
                            }
                            break;
                        }
                        q.cv.wait(&mut guard);
                        if stopping.load(Ordering::Acquire) {
                            break;
                        }
                    }
                }
            }));
        }
    }

    pub fn add_job<T>(&self, job: Job<T>) -> Result
    where
        T: FnOnce() + Send + 'static,
    {
        match job.priority {
            Priority::Background => {
                self.bg_jobs.fetch_add(1, Ordering::Release);
                let mut guard = self.bg_q.queue.lock();
                guard.push_back(Box::new(job.inner));
                drop(guard);
                self.bg_q.cv.notify_one();
                Ok(())
            }
            Priority::VideoFrame => {
                self.fg_jobs.fetch_add(1, Ordering::Release);
                // Try to enqueue into the bounded frame queue
                match self.frame_q.q.push(Box::new(job.inner)) {
                    Ok(()) => {
                        // Wake one waiter
                        self.frame_q.cv.notify_one();
                        Ok(())
                    }
                    Err(_task) => Err(LunarisError::RenderQueueFull),
                }
            }
            // Immediate/Normal/Deferred
            p => {
                self.fg_jobs.fetch_add(1, Ordering::Release);
                let mut guard = self.default_q.queue.lock();
                guard.push(p, Box::new(job.inner));
                drop(guard);
                self.default_q.cv.notify_one();
                Ok(())
            }
        }
    }

    pub fn add_job_async<F, Fut>(&self, job: AsyncJob<F, Fut>) -> Result
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: core::future::Future<Output = ()> + Send + 'static,
    {
        let is_bg = matches!(job.priority, Priority::Background);
        if is_bg {
            self.bg_jobs.fetch_add(1, Ordering::Release);
        } else {
            self.fg_jobs.fetch_add(1, Ordering::Release);
        }

        let priority = job.priority;
        let fg = self.fg_jobs.clone();
        let bg = self.bg_jobs.clone();
        let zero_cv = self.zero_cv.clone();
        let zero_lock = self.zero_cv_lock.clone();

        // Spawn on runtime; we could bias priority by spawning onto local sets
        self.rt.spawn(async move {
            (job.inner)().await;
            // decrement and notify
            if matches!(priority, Priority::Background) {
                if bg.fetch_sub(1, Ordering::AcqRel) == 1 {
                    let _g = zero_lock.lock();
                    zero_cv.notify_all();
                    drop(_g);
                }
            } else if fg.fetch_sub(1, Ordering::AcqRel) == 1 {
                let _g = zero_lock.lock();
                zero_cv.notify_all();
                drop(_g);
            }
        });

        Ok(())
    }

    pub fn join_sync(&self) -> Result {
        let mut g = self.zero_cv_lock.lock();
        while self.fg_jobs.load(Ordering::Acquire) != 0 {
            self.zero_cv.wait(&mut g);
        }
        Ok(())
    }

    pub fn join_all(&self) -> Result {
        let mut g = self.zero_cv_lock.lock();
        while self.fg_jobs.load(Ordering::Acquire) != 0 || self.bg_jobs.load(Ordering::Acquire) != 0
        {
            self.zero_cv.wait(&mut g);
        }
        Ok(())
    }

    pub fn reconfigure_threads(&self, default: usize, frame: usize, background: usize) {
        // Stop all workers and respawn with new counts
        self.stopping.store(true, Ordering::Release);
        {
            let mut v = self.default_workers.lock();
            for h in v.drain(..) {
                let _ = h.join();
            }
        }
        {
            let mut v = self.frame_workers.lock();
            for h in v.drain(..) {
                let _ = h.join();
            }
        }
        {
            let mut v = self.background_workers.lock();
            for h in v.drain(..) {
                let _ = h.join();
            }
        }
        self.stopping.store(false, Ordering::Release);
        self.spawn_workers(SchedulerConfig {
            default_threads: default.max(1),
            frame_threads: frame.max(1),
            background_threads: background.max(1),
            async_threads: 1, // unchanged; reconfiguring async would need rebuilding the runtime
        });
    }
    pub fn profile(&self) -> OrchestratorProfile {
        let q = self.default_q.queue.lock();
        OrchestratorProfile {
            immediate: q.immediate.len() as u64,
            normal: q.normal.len() as u64,
            deferred: q.deferred.len() as u64,
            frame: self.frame_q.q.len() as u64,
            running_tasks: (self.frame_workers.lock().len()
                + self.default_workers.lock().len()
                + self.background_workers.lock().len()) as u64,
        }
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        // Wake all workers so they can exit
        self.default_q.cv.notify_all();
        self.frame_q.cv.notify_all();
        self.bg_q.cv.notify_all();
        for h in self.default_workers.get_mut().drain(..) {
            let _ = h.join();
        }
        for h in self.frame_workers.get_mut().drain(..) {
            let _ = h.join();
        }
        for h in self.background_workers.get_mut().drain(..) {
            let _ = h.join();
        }
        // Runtime drops and waits its tasks naturally
    }
}
