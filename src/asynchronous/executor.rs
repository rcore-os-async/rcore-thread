use queueue::queue::nonblocking::*;
use core::future::Future;
use async_task::Task;

type ExecutionTag = ();

pub struct Executor {
    queue: StaticSpinQueue<Task<ExecutionTag>, 16>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            queue: StaticSpinQueue::default()
        }
    }

    pub fn spawn<F>(&'static self, fut: F) -> async_task::JoinHandle<(), ExecutionTag>
    where F: Future<Output=()> + Send + 'static {
        let prod = self.queue.producer();
        let schedule = move |task| {
            prod.push(task).unwrap();
        };
        let (task, handle) = async_task::spawn(fut, schedule, ());
        task.schedule();
        handle
    }

    pub fn run_forever(&self) -> ! {
        loop {
            if let Some(task) = self.queue.pop() {
                task.run();
            }

            // TODO: steal from other queues, and read from global queue
        }
    }
}
