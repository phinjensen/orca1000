#![no_std]
#![no_main]

use core::{arch::asm, panic::PanicInfo, slice};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[repr(C)]
struct SbiReturn {
    error: i32,
    value: i32,
}

fn sbi_call(
    mut arg0: i32,
    mut arg1: i32,
    arg2: i32,
    arg3: i32,
    arg4: i32,
    arg5: i32,
    fid: i32,
    eid: i32,
) -> SbiReturn {
    unsafe {
        asm!(
            "ecall",
            inout("a0") arg0,
            inout("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a6") fid,
            in("a7") eid
        );
    }
    SbiReturn {
        error: arg0,
        value: arg1,
    }
}

pub fn putchar(char: u8) {
    sbi_call(char as _, 0, 0, 0, 0, 0, 0, 1);
}

unsafe fn memset(buf: *mut u8, c: u8, n: usize) {
    let buf: &mut [u8] = unsafe { slice::from_raw_parts_mut(buf, n) };
    for i in 0..buf.len() {
        buf[i] = c;
    }
}

#[unsafe(no_mangle)]
pub fn kernel_main() -> ! {
    unsafe extern "C" {
        static mut __bss: u8;
        static __bss_end: u8;
    }

    unsafe {
        memset(
            &raw mut __bss,
            0,
            (&raw const __bss_end).offset_from_unsigned(&raw const __bss),
        );
    }

    for char in "\n\nhello world\n".bytes() {
        putchar(char);
    }

    loop {
        unsafe { asm!("wfi") }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn boot() -> ! {
    unsafe extern "C" {
        static __stack_top: u8;
    }

    unsafe {
        asm!(
            "mv sp, {}",
            "j kernel_main",
            in(reg) &raw const __stack_top
        );
    }
    loop {}
}
