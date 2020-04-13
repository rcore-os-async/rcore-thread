use queueue::timing_wheel::hierarchical::BoundedWheel;
use core::task::Waker;
use riscv;
use spin::Mutex;

use core::fmt::{self, Write};
use super::sbi;

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

const INFINITY_TO: u64= core::u64::MAX;
const MAX_TO: u64= 100;
const RT_CLK_FREQ: u64 = 10;

type Wheel = BoundedWheel<Waker, 2>; // TODO: use slab alloc
pub struct Timer {
    wheel: Wheel,
    cur_timeout: Option<usize>,
}

impl Timer {
    pub const fn const_new() -> Timer {
        Timer {
            wheel: Wheel::new_bounded(0),
            cur_timeout: None,
        }
    }

    pub fn new() -> Timer {
        let time = riscv::register::time::read();
        Timer {
            wheel: Wheel::new(time),
            cur_timeout: None,
        }
    }

    pub fn wakeup(&mut self) {
        let time = riscv::register::time::read();
        crate::println!("Wakeup at {}", time);
        self.wheel.fast_forward(time, |waker, _at| waker.wake());
    }

    fn schedule(&mut self, waker: Waker, tick: usize) -> bool {
        let elapsed = self.wheel.elapsed();
        crate::println!("Schd, {} -> {}", elapsed, tick);
        if elapsed >= tick {
            return true;
        }

        self.wheel.schedule(tick, waker).unwrap();
        let timeout = self.wheel.min_next_event();
        crate::println!("MinEv, {:?}", timeout);
        if timeout != self.cur_timeout {
            self.cur_timeout = timeout;
            let time = riscv::register::time::read();
            let to = timeout.map(|e| e as u64).unwrap_or(INFINITY_TO).min(time as u64 + MAX_TO);
            crate::println!("Schd at {}", to);
            super::sbi::set_timer(to);
        }

        return false;
    }
}

pub struct Timeout {
    target_tick: usize,
    timer: &'static Mutex<Timer>,
}

impl Timeout {
    pub fn from(timer: &'static Mutex<Timer>, dur: core::time::Duration) -> Timeout {
        puts("Creating timeout...");
        let tick_dur = dur.as_micros() / RT_CLK_FREQ as u128;
        let cur = riscv::register::time::read();
        let tick = tick_dur as usize + cur;
        crate::println!("Timeout created at {}", tick);
        Timeout {
            target_tick: tick,
            timer: timer,
        }
    }
}

impl core::future::Future for Timeout {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>)
    -> core::task::Poll<Self::Output> {
        crate::println!("Timeout polled");
        if self.timer.lock().schedule(cx.waker().clone(), self.target_tick) {
            core::task::Poll::Ready(())
        } else {
            core::task::Poll::Pending
        }
    }
}
