use alloc::boxed::Box;
use async_task::Task;
use core::future::Future;
use lazy_static::*;
use log::*;
use queueue::queue::nonblocking::*;
use spin::Mutex;

type ExecutionTag = ();

pub struct Executor {
    queue: StaticSpinQueue<Task<ExecutionTag>, 16>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            queue: StaticSpinQueue::default(),
        }
    }

    pub fn spawn<F>(&'static self, fut: F) -> async_task::JoinHandle<(), ExecutionTag>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let prod = self.queue.producer();
        let schedule = move |task| {
            trace!("Pushed");
            prod.push(task).unwrap();
        };
        let (task, handle) = async_task::spawn(fut, schedule, ());
        task.schedule();
        handle
    }
}

lazy_static! {
    static ref GLOBAL_EXECUTOR: Mutex<Box<Executor>> = {
        let m = Executor::new();
        Mutex::new(Box::new(m))
    };
}

pub fn spawn<F>(fut: F) -> async_task::JoinHandle<(), ExecutionTag>
where
    F: Future<Output = ()> + Send + 'static,
{
    GLOBAL_EXECUTOR.lock().spawn(fut)
}

pub fn fun() -> ! {
    loop {
        if let Some(task) = GLOBAL_EXECUTOR.lock().queue.pop() {
            trace!("Popped");
            task.run();
            trace!("Run over");
        }
    }
}
