use queueue::timing_wheel::hierarchical::BoundedWheel;
use core::task::Waker;
use riscv;
use spin::Mutex;

const INFINITY_TO: u64= core::u64::MAX;
const RT_CLK_FREQ: u64 = 1;

type Wheel = BoundedWheel<Waker, 256>; // TODO: use slab alloc
pub struct Timer {
    wheel: Wheel,
    cur_timeout: Option<usize>,
}

impl Timer {
    pub fn new() -> Timer {
        let time = riscv::register::time::read();
        Timer {
            wheel: Wheel::new(time),
            cur_timeout: None,
        }
    }

    pub fn wakeup(&mut self, moment: usize) {
        let time = riscv::register::time::read();
        self.wheel.fast_forward(time, |waker, _at| waker.wake());
    }

    fn schedule(&mut self, waker: Waker, tick: usize) -> bool {
        let elapsed = self.wheel.elapsed();
        if elapsed >= tick {
            return true;
        }

        self.wheel.schedule(tick, waker).unwrap();
        let timeout = self.wheel.min_next_event();
        if timeout != self.cur_timeout {
            self.cur_timeout = timeout;
            let to = timeout.map(|e| e as u64).unwrap_or(INFINITY_TO);
            super::sbi::set_timer(to);
        }

        return false;
    }

    pub fn create_timeout(timer: &'static Mutex<Timer>, dur: core::time::Duration) -> Timeout {
        let tick_dur = dur.as_micros() / RT_CLK_FREQ as u128;
        Timeout {
            target_tick: tick_dur as usize,
            timer: timer,
        }
    }
}

pub struct Timeout {
    target_tick: usize,
    timer: &'static Mutex<Timer>,
}

impl core::future::Future for Timeout {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>)
    -> core::task::Poll<Self::Output> {
        if self.timer.lock().schedule(cx.waker().clone(), self.target_tick) {
            core::task::Poll::Ready(())
        } else {
            core::task::Poll::Pending
        }
    }
}
