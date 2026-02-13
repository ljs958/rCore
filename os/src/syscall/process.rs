//! Process management syscalls

use crate::mm::translated_byte_buffer;
use crate::config::PAGE_SIZE;
use crate::task::{
    change_program_brk, current_user_token, exit_current_and_run_next, mmap, munmap,
    set_current_priority, suspend_current_and_run_next,
};
use crate::mm::MapPermission;
use crate::timer::get_time_ms;
use core::mem::size_of;

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// set priority of current task
pub fn sys_set_priority(prio: isize) -> isize {
    set_current_priority(prio)
}

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// get current time
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    if ts.is_null() {
        return -1;
    }
    let time_ms = get_time_ms();
    let timeval = TimeVal {
        sec: time_ms / 1000,
        usec: (time_ms % 1000) * 1000,
    };
    let data = unsafe {
        core::slice::from_raw_parts((&timeval as *const TimeVal) as *const u8, size_of::<TimeVal>())
    };
    let mut offset = 0usize;
    for buf in translated_byte_buffer(current_user_token(), ts as *const u8, size_of::<TimeVal>()) {
        let end = offset + buf.len();
        buf.copy_from_slice(&data[offset..end]);
        offset = end;
    }
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// map memory
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if prot & !0x7 != 0 || (prot & 0x7) == 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let len_rounded = (len + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
    let mut perm = MapPermission::U;
    if (prot & 0x1) != 0 {
        perm |= MapPermission::R;
    }
    if (prot & 0x2) != 0 {
        perm |= MapPermission::W;
    }
    if (prot & 0x4) != 0 {
        perm |= MapPermission::X;
    }
    match mmap(start, len, perm) {
        Ok(()) => len_rounded as isize,
        Err(()) => -1,
    }
}

/// unmap memory
pub fn sys_munmap(start: usize, len: usize) -> isize {
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if len % PAGE_SIZE != 0 {
        return -1;
    }
    match munmap(start, len) {
        Ok(()) => len as isize,
        Err(()) => -1,
    }
}
