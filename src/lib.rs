#![cfg_attr(not(test), no_std)]
#![feature(linkage)]
#![feature(llvm_asm)]
#![feature(naked_functions)]
#![feature(global_asm)]
#![deny(warnings)]

extern crate alloc;

pub mod asynchronous;
pub mod scheduler;
