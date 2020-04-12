use crate::scheduler::Scheduler;
use crate::timer::Timer;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context as FutureContext, Poll, Waker};
use spin::{Mutex, MutexGuard};

struct SleepFuture {
    pub shared_state: ArcMutexSleepSharedState,
}

struct SleepSharedState {
    completed: bool,
    waker: Option<Waker>,
}

#[derive(Clone)]
struct ArcMutexSleepSharedState(Arc<Mutex<SleepSharedState>>);

impl PartialEq for ArcMutexSleepSharedState {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ArcMutexSleepSharedState {}

impl SleepFuture {
    pub fn new() -> Self {
        let shared_state = Arc::new(Mutex::new(SleepSharedState {
            completed: false,
            waker: None,
        }));
        SleepFuture {
            shared_state: ArcMutexSleepSharedState(shared_state),
        }
    }
}

impl Future for SleepFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut FutureContext<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.0.lock();
        if shared_state.completed {
            Poll::Ready(())
        } else {
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

struct ThreadFuture {
    pub shared_state: ArcMutexThreadSharedState,
}

struct ThreadSharedState {
    completed: bool,
    waker: Option<Waker>,
    context: Option<Box<dyn ThreadContext>>,
}

#[derive(Clone)]
struct ArcMutexThreadSharedState(Arc<Mutex<ThreadSharedState>>);

impl PartialEq for ArcMutexThreadSharedState {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ArcMutexThreadSharedState {}

impl ThreadFuture {
    pub fn new(context: Box<dyn ThreadContext>) -> Self {
        let shared_state = Arc::new(Mutex::new(ThreadSharedState {
            completed: false,
            waker: None,
            context: Some(context),
        }));
        ThreadFuture {
            shared_state: ArcMutexThreadSharedState(shared_state),
        }
    }
}

impl Future for ThreadFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut FutureContext<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.0.lock();
        if shared_state.completed {
            Poll::Ready(())
        } else {
            // TODO: Call context to switch?
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

// TODO: consider what api should a thread context expose
pub trait ThreadContext {}

pub struct WakerPool {
    threads: Vec<Mutex<Option<ArcMutexThreadSharedState>>>,
    scheduler: Box<dyn Scheduler>,
    timer: Mutex<Timer<ArcMutexSleepSharedState>>,
}

pub type Tid = usize;

impl WakerPool {
    pub fn new(scheduler: impl Scheduler, max_proc_num: usize) -> Self {
        WakerPool {
            threads: new_vec_default(max_proc_num),
            scheduler: Box::new(scheduler),
            timer: Mutex::new(Timer::new()),
        }
    }

    fn alloc_tid(&self) -> (Tid, MutexGuard<Option<ArcMutexThreadSharedState>>) {
        for (i, proc) in self.threads.iter().enumerate() {
            let thread = proc.lock();
            if thread.is_none() {
                return (i, thread);
            }
        }
        panic!("Thread number exceeded");
    }

    pub fn add(&self, context: Box<dyn ThreadContext>) -> Tid {
        let (tid, mut thread) = self.alloc_tid();
        let future = ThreadFuture::new(context);
        *thread = Some(future.shared_state.clone());
        self.scheduler.push(tid);
        tid
    }

    pub(crate) fn tick(&self, cpu_id: usize, tid: Option<Tid>) -> bool {
        if cpu_id == 0 {
            let mut timer = self.timer.lock();
            timer.tick();
            while let Some(event) = timer.pop() {
                let mut shared_state = event.0.lock();
                if let Some(waker) = shared_state.waker.take() {
                    waker.wake()
                }
            }
        }
        match tid {
            Some(tid) => self.scheduler.tick(tid),
            None => false,
        }
    }

    /// Set the priority of thread `tid`
    pub fn set_priority(&self, tid: Tid, priority: u8) {
        self.scheduler.set_priority(tid, priority);
    }

    pub(crate) fn run(&self, cpu_id: usize) -> Option<impl Future> {
        self.scheduler.pop(cpu_id).map(|tid| {
            let proc_lock = self.threads[tid].lock();
            let proc = proc_lock.as_ref().expect("thread not exist");
            ThreadFuture {
                shared_state: proc.clone(),
            }
        })
    }

    pub fn sleep(&self, time: usize) -> impl Future {
        let future = SleepFuture::new();
        if time != 0 {
            self.timer.lock().start(time, future.shared_state.clone());
        }
        future
    }
}

fn new_vec_default<T: Default>(size: usize) -> Vec<T> {
    let mut vec = Vec::new();
    vec.resize_with(size, Default::default);
    vec
}
