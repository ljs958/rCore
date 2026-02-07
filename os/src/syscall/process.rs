//! App management syscalls
use crate::batch::{current_task_info, run_next_app};

const TASK_NAME_LEN: usize = 32;

#[repr(C)]
pub struct TaskInfo {
    pub id: usize,
    pub name: [u8; TASK_NAME_LEN],
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    run_next_app()
}

/// get current task info into user buffer
pub fn sys_get_taskinfo(taskinfo: *mut TaskInfo) -> isize {
    if taskinfo.is_null() {
        return -1;
    }
    let (id, name) = current_task_info();
    let bytes = name.as_bytes();
    unsafe {
        (*taskinfo).id = id;
        (*taskinfo).name.fill(0);
        let len = core::cmp::min(bytes.len(), TASK_NAME_LEN - 1);
        (*taskinfo).name[..len].copy_from_slice(&bytes[..len]);
    }
    0
}
