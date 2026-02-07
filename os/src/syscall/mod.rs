//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_GET_TASKINFO: usize = 200;
const MAX_SYSCALL_ID: usize = 256;

mod fs;
mod process;

use fs::*;
use process::*;
use crate::sync::UPSafeCell;
use lazy_static::*;

struct SyscallStats {
    counts: [usize; MAX_SYSCALL_ID],
}

lazy_static! {
    static ref SYSCALL_STATS: UPSafeCell<SyscallStats> = unsafe {
        UPSafeCell::new(SyscallStats {
            counts: [0; MAX_SYSCALL_ID],
        })
    };
}

fn record_syscall(syscall_id: usize) {
    if syscall_id < MAX_SYSCALL_ID {
        let mut stats = SYSCALL_STATS.exclusive_access();
        stats.counts[syscall_id] += 1;
    }
}

/// print syscall statistics collected so far
pub fn print_syscall_stats() {
    let stats = SYSCALL_STATS.exclusive_access();
    let mut any = false;
    println!("[kernel] Syscall statistics:");
    for (id, count) in stats.counts.iter().enumerate() {
        if *count > 0 {
            any = true;
            println!("[kernel]  syscall {} -> {}", id, count);
        }
    }
    if !any {
        println!("[kernel]  (no syscalls recorded)");
    }
}

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    record_syscall(syscall_id);
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_GET_TASKINFO => sys_get_taskinfo(args[0] as *mut TaskInfo),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
