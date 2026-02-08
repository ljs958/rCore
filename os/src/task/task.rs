//! Types related to task management

use super::TaskContext;
use crate::config::{MAX_SYSCALL_NUM };
/// 系统调用信息结构体
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SyscallInfo {
    pub id: usize,
    pub times: usize
}

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub user_time: usize,
    pub kernel_time: usize,
    pub call: [SyscallInfo; MAX_SYSCALL_NUM]
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}
