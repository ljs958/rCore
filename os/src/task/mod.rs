//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;

#[allow(clippy::module_inception)]
mod task;

use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM };
use crate::loader::{get_num_app, init_app_cx};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::timer::get_time_ms;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus , SyscallInfo};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// Inner of Task Manager
pub struct TaskManagerInner {
    /// task list
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    current_task: usize,
    //  last stop time of current task, used to calculate user time and kernel time
    stop_time: usize,
}

/// 记录任务信息的结构体
#[repr(C)]
#[derive(Copy, Clone)]
pub struct TaskInfo {
    /// task id
    pub id: usize,
    /// task status
    pub status: TaskStatus,
    /// syscall statistics
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    /// total running time in ms
    pub time: usize,
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            user_time: 0,
            kernel_time: 0,
            call: [SyscallInfo { id: 0, times: 0 }; MAX_SYSCALL_NUM]
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                    stop_time: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        inner.refresh_stop_watch();
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].kernel_time += inner.refresh_stop_watch();
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].kernel_time += inner.refresh_stop_watch();
        println!("Task {} exited! User time: {} ms, Kernel time: {} ms",
            current, inner.tasks[current].user_time, inner.tasks[current].kernel_time);
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            //println!("Switching from task {} to task {}", current, next);
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            println!("All applications completed!");
            shutdown(false);
        }
    }

    /// 统计内核时间，从现在开始算的是用户时间
    fn user_time_start(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].kernel_time += inner.refresh_stop_watch();
    }

    /// 统计用户时间，从现在开始算的是内核时间
    fn user_time_end(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].user_time += inner.refresh_stop_watch();
    }

    /// 记录系统调用次数
    fn get_syscall_time(&self , id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        for i in 0..MAX_SYSCALL_NUM {
            if inner.tasks[current].call[i].id == id {
                inner.tasks[current].call[i].times += 1;
                return;
            }
        }
        for i in 0..MAX_SYSCALL_NUM {
            if inner.tasks[current].call[i].id == 0 {
                inner.tasks[current].call[i].id = id;
                inner.tasks[current].call[i].times = 1;
                return;
            }
        }
    }

    /// 获取任务信息
    fn get_task_info(&self , id: usize) -> Option<TaskInfo>{
        let inner = self.inner.exclusive_access();
        if id >= self.num_app {
            return None;
        }
        else {
            let task = &inner.tasks[id];
            Some(TaskInfo {
                id,
                status: task.task_status,
                call: task.call,
                time: task.user_time + task.kernel_time
            })
        }
    }

}

impl TaskManagerInner{
    /// Refresh stop watch and return the time duration since last refresh.
    fn refresh_stop_watch(&mut self) -> usize {
        let start_time = self.stop_time;
        self.stop_time = get_time_ms();
        self.stop_time - start_time
    }
}


/// run first task
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// rust next task
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// suspend current task
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// exit current task
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// exit current task,  then run next task
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// 统计内核时间，从现在开始算的是用户时间
pub fn user_time_start() {
    TASK_MANAGER.user_time_start()
}

/// 统计用户时间，从现在开始算的是内核时间
pub fn user_time_end() {
    TASK_MANAGER.user_time_end()
}

/// 记录系统调用次数
pub fn get_syscall_time(id: usize) {
    TASK_MANAGER.get_syscall_time(id);
}

/// 获取任务信息
pub fn get_task_info(id: usize) -> Option<TaskInfo> {
    TASK_MANAGER.get_task_info(id)
}
