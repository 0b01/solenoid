#![no_main]

use std::os::raw;
mod bindings;

#[no_mangle]
pub extern fn main(argc: i32, argv: *const *const u8) -> i32 {
    let mut ret_offset = 0;
    let mut ret_len = 0;
    let ptr: *mut u8 = unsafe { std::mem::transmute(&bindings::storage as *const u8) };
    let stack_ptr: *mut u8 = unsafe { std::mem::transmute(&bindings::stack as *const u8) };

    unsafe { bindings::SimpleStorage_constructor(0 as *mut u8, 0, &mut ret_offset, &mut ret_len, ptr); }
    let sp = unsafe { bindings::sp };
    for i in 0..sp {
        unsafe { bindings::prt(stack_ptr.offset(32 * i as isize)); }
    }
    0
}

