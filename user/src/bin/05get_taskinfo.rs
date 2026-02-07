#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{get_taskinfo, TaskInfo, TASK_NAME_LEN};

#[unsafe(no_mangle)]
fn main() -> i32 {
    let mut info = TaskInfo {
        id: 0,
        name: [0; TASK_NAME_LEN],
    };
    let ret = get_taskinfo(&mut info);
    if ret < 0 {
        println!("get_taskinfo failed: {}", ret);
        return -1;
    }
    let len = info.name.iter().position(|&b| b == 0).unwrap_or(TASK_NAME_LEN);
    let name = core::str::from_utf8(&info.name[..len]).unwrap_or("invalid");
    println!("task id = {}, task name = {}", info.id, name);
    0
}
