use queueue::queue::nonblocking::*;
use core::future::Future;
use async_task::Task;

use super::sbi;
use core::fmt;
use core::fmt::Write;

pub fn putchar(ch: char) {
    sbi::console_putchar(ch as u8 as usize);
}

pub fn puts(s: &str) {
    for ch in s.chars() {
        putchar(ch);
    }
}

struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        puts(s);
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        _print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

pub fn getchar() -> char {
    let c = sbi::console_getchar() as u8;

    match c {
        255 => '\0',
        c => c as char,
    }
}
// 调用 OpenSBI 接口
pub fn getchar_option() -> Option<char> {
    let c = sbi::console_getchar() as isize;
    match c {
        -1 => None,
        c => Some(c as u8 as char),
    }
}


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
            println!("Pushed");
            prod.push(task).unwrap();
        };
        let (task, handle) = async_task::spawn(fut, schedule, ());
        task.schedule();
        handle
    }

    pub fn run_forever(&self) -> ! {
        loop {
            if let Some(task) = self.queue.pop() {
                println!("Popped");
                task.run();
                println!("Run over");
            }

            // TODO: steal from other queues, and read from global queue
        }
    }
}
