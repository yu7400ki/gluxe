//! Custom Boa [`JobExecutor`] that makes timers (`setTimeout`/`setInterval`)
//! cooperate with the event-driven render pump instead of freezing it.
//!
//! Boa's default [`SimpleJobExecutor`] drains *every* pending job including
//! future clock jobs, so a self-re-enqueuing `setInterval` makes `run_jobs()`
//! busy-wait forever — hanging the UI thread, since the pump
//! (`state::run_boa_jobs`) and startup (`lib::run`) both assume it returns
//! promptly. [`GpuiJobExecutor`] copies `SimpleJobExecutor` but dispatches only
//! **already-due** clock jobs, then returns, leaving future timers parked. The
//! runtime arms a GPUI background timer for [`GpuiJobExecutor::next_due`] (see
//! `state::arm_next_timer`) to wake the pump when the earliest one comes due.
//! Net effect: timers run incrementally, event-driven, zero wakeups while idle.
//!
//! [`SimpleJobExecutor`]: boa_engine::job::SimpleJobExecutor

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::mem;
use std::rc::Rc;

use boa_engine::context::time::JsInstant;
use boa_engine::job::{
    GenericJob, IntervalJob, Job, JobExecutor, NativeAsyncJob, PromiseJob, TimeoutJob,
};
use boa_engine::{Context, JsResult};
use futures_concurrency::future::FutureGroup;
use futures_lite::{StreamExt, future};

/// A clock job parked until its deadline: either a one-shot `setTimeout` or a
/// repeating `setInterval`. Mirrors the (private) equivalent in Boa.
enum ClockJob {
    Timeout(TimeoutJob),
    Interval(IntervalJob),
}

impl ClockJob {
    fn cancelled(&self) -> bool {
        match self {
            ClockJob::Timeout(t) => t.cancelled(),
            ClockJob::Interval(i) => i.cancelled(),
        }
    }
}

/// Runs promise/async/generic jobs to completion but fires only already-due
/// clock jobs (see module docs for the rationale).
#[derive(Default)]
pub(crate) struct GpuiJobExecutor {
    promise_jobs: RefCell<VecDeque<PromiseJob>>,
    async_jobs: RefCell<VecDeque<NativeAsyncJob>>,
    finalization_registry_jobs: RefCell<VecDeque<NativeAsyncJob>>,
    /// Timeout/interval jobs keyed by their absolute due instant. A `BTreeMap`
    /// keeps them ordered so `next_due` is the cheapest possible lookup.
    clock_jobs: RefCell<BTreeMap<JsInstant, Vec<ClockJob>>>,
    generic_jobs: RefCell<VecDeque<GenericJob>>,
}

impl GpuiJobExecutor {
    fn clear(&self) {
        // Destructure without `..` so adding a queue field becomes a compile
        // error here until it is cleared too. The previous version forgot
        // `finalization_registry_jobs`, an omission that was invisible at
        // runtime (it only leaks on dev-mode hot reload).
        let Self {
            promise_jobs,
            async_jobs,
            finalization_registry_jobs,
            clock_jobs,
            generic_jobs,
        } = self;
        promise_jobs.borrow_mut().clear();
        async_jobs.borrow_mut().clear();
        finalization_registry_jobs.borrow_mut().clear();
        clock_jobs.borrow_mut().clear();
        generic_jobs.borrow_mut().clear();
    }

    /// The earliest parked clock-job deadline, if any. Used by the runtime to arm
    /// a GPUI timer that wakes the pump when this instant arrives.
    pub(crate) fn next_due(&self) -> Option<JsInstant> {
        self.clock_jobs.borrow().keys().next().copied()
    }

    /// True when nothing could run *right now*. Future clock jobs are ignored —
    /// they don't keep the loop alive; the pump re-enters once its timer fires.
    fn no_immediate_work(&self, now: JsInstant) -> bool {
        self.promise_jobs.borrow().is_empty()
            && self.async_jobs.borrow().is_empty()
            && self.generic_jobs.borrow().is_empty()
            && self
                .clock_jobs
                .borrow()
                .keys()
                .next()
                .is_none_or(|earliest| *earliest >= now)
    }
}

impl JobExecutor for GpuiJobExecutor {
    fn enqueue_job(self: Rc<Self>, job: Job, context: &mut Context) {
        match job {
            Job::PromiseJob(p) => self.promise_jobs.borrow_mut().push_back(p),
            Job::AsyncJob(a) => self.async_jobs.borrow_mut().push_back(a),
            Job::TimeoutJob(t) => {
                let now = context.clock().now();
                self.clock_jobs
                    .borrow_mut()
                    .entry(now + t.timeout())
                    .or_default()
                    .push(ClockJob::Timeout(t));
            }
            Job::IntervalJob(i) => {
                let now = context.clock().now();
                self.clock_jobs
                    .borrow_mut()
                    .entry(now + i.interval())
                    .or_default()
                    .push(ClockJob::Interval(i));
            }
            Job::GenericJob(g) => self.generic_jobs.borrow_mut().push_back(g),
            Job::FinalizationRegistryCleanupJob(fr) => {
                self.finalization_registry_jobs.borrow_mut().push_back(fr);
            }
            // `Job` is `#[non_exhaustive]`; a future Boa rev may add variants we
            // don't yet know how to schedule. Dropping is the safe default — the
            // job simply never runs rather than corrupting our queues.
            _ => {}
        }
    }

    fn run_jobs(self: Rc<Self>, context: &mut Context) -> JsResult<()> {
        future::block_on(self.run_jobs_async(&RefCell::new(context)))
    }

    async fn run_jobs_async(self: Rc<Self>, context: &RefCell<&mut Context>) -> JsResult<()>
    where
        Self: Sized,
    {
        let mut group = FutureGroup::new();
        let mut fr_group = FutureGroup::new();
        loop {
            for job in mem::take(&mut *self.async_jobs.borrow_mut()) {
                group.insert(job.call(context));
            }

            for job in mem::take(&mut *self.finalization_registry_jobs.borrow_mut()) {
                fr_group.insert(job.call(context));
            }

            // Dispatch every clock job whose deadline has already passed. Jobs
            // exactly at `now` (and any in the future) stay parked; an armed GPUI
            // timer re-enters this loop once they come due. Intervals re-enqueue
            // themselves at `now + interval`.
            {
                let now = context.borrow().clock().now();
                let jobs_to_run = {
                    let mut clock_jobs = self.clock_jobs.borrow_mut();
                    // `split_off(&now)` keeps deadlines >= now (still future);
                    // the returned remainder is everything strictly before now.
                    let mut jobs_to_keep = clock_jobs.split_off(&now);
                    jobs_to_keep.retain(|_, jobs| {
                        jobs.retain(|job| !job.cancelled());
                        !jobs.is_empty()
                    });
                    mem::replace(&mut *clock_jobs, jobs_to_keep)
                };

                for jobs in jobs_to_run.into_values() {
                    for job in jobs {
                        if job.cancelled() {
                            continue;
                        }
                        match job {
                            ClockJob::Timeout(job) => {
                                if let Err(err) = job.call(&mut context.borrow_mut()) {
                                    self.clear();
                                    return Err(err);
                                }
                            }
                            ClockJob::Interval(job) => {
                                let context = &mut context.borrow_mut();
                                let now = context.clock().now();
                                if let Err(err) = job.call(context) {
                                    self.clear();
                                    return Err(err);
                                }
                                self.clock_jobs
                                    .borrow_mut()
                                    .entry(now + job.interval())
                                    .or_default()
                                    .push(ClockJob::Interval(job));
                            }
                        }
                    }
                }
            }

            // Termination: break once nothing can run right now and no async
            // futures are pending. Parked future timers do NOT keep us spinning.
            let now = context.borrow().clock().now();
            if self.no_immediate_work(now) && group.is_empty() {
                match future::poll_once(fr_group.next()).await.flatten() {
                    Some(Err(err)) => {
                        self.clear();
                        return Err(err);
                    }
                    _ if !self.no_immediate_work(context.borrow().clock().now()) => {}
                    _ => break,
                }
            }

            if let Some(Err(err)) = future::poll_once(group.next()).await.flatten() {
                self.clear();
                return Err(err);
            }

            let jobs = mem::take(&mut *self.promise_jobs.borrow_mut());
            for job in jobs {
                if let Err(err) = job.call(&mut context.borrow_mut()) {
                    self.clear();
                    return Err(err);
                }
            }

            let jobs = mem::take(&mut *self.generic_jobs.borrow_mut());
            for job in jobs {
                if let Err(err) = job.call(&mut context.borrow_mut()) {
                    self.clear();
                    return Err(err);
                }
            }
            context.borrow_mut().clear_kept_objects();
            future::yield_now().await;
        }

        Ok(())
    }
}
