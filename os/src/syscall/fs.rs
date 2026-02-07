//! File and filesystem-related syscalls

const FD_STDOUT: usize = 1;

fn is_user_buffer_valid(buf: *const u8, len: usize) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }
    let start = buf as usize;
    let end = match start.checked_add(len) {
        Some(v) => v,
        None => return false,
    };
    let (app_start, app_end) = crate::batch::app_memory_range();
    let (stack_start, stack_end) = crate::batch::user_stack_range();
    let in_app = start >= app_start && end <= app_end;
    let in_stack = start >= stack_start && end <= stack_end;
    in_app || in_stack
}

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            if !is_user_buffer_valid(buf, len) {
                return -1;
            }
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => -1,
    }
}
