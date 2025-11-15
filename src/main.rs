#![no_std]
#![no_main]

use core::{arch::asm, fmt::Write, panic::PanicInfo, slice};

pub struct DebugConsoleWriter;

impl Write for DebugConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.as_bytes() {
            putchar(*b);
        }
        Ok(())
    }
}

#[allow(unused)]
macro_rules! dprint {
    ($($arg:tt)*) => {{
        write!(DebugConsoleWriter, "{}", format_args!($($arg)*)).ok();
    }}
}

#[allow(unused)]
macro_rules! dprintln {
    () => {{
        write!(DebugConsoleWriter, "\n").ok();
    }};

    ($($arg:tt)*) => {{
        write!(DebugConsoleWriter, "{}\n", format_args!($($arg)*)).ok();
    }}
}

macro_rules! read_csr {
    ($reg:tt) => {
        unsafe {
            let __tmp: u32;
            asm!(concat!("csrr {}, ", stringify!($reg)), out(reg) __tmp);
            __tmp
        }
    };
}

macro_rules! write_csr {
    ($reg:tt, $value:expr) => {
        unsafe {
            let __tmp = $value;
            asm!(concat!("csrw ", stringify!($reg), ", {}"), in(reg) __tmp);
        }
    };
}

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    dprint!("PANIC: ");
    if let Some(location) = panic_info.location() {
        dprint!("{}:{}: ", location.file(), location.line(),);
    } else {
        dprintln!("??:??: ");
    }
    dprintln!("{}", panic_info.message());
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

unsafe fn memcpy(mut dest: *mut u8, mut source: *mut u8, n: usize) {
    for _ in 0..n {
        unsafe {
            dest.write(source.read());
            dest = dest.wrapping_add(1);
            source = source.wrapping_add(1);
        }
    }
}

fn strcpy(mut dest: *mut u8, mut source: *const u8) -> *mut u8 {
    let result = dest.clone();
    unsafe {
        while source.read() != 0 {
            dest.write(source.read());
            dest = dest.add(1);
            source = source.add(1);
        }
    }
    return result;
}

fn strcmp(mut s1: *const u8, mut s2: *const u8) {
    unsafe {
        loop {
            let c1 = s1.read();
            let c2 = s2.read();
            if c1 == 0 || c2 == 0 || c1 != c2 {
                break;
            };
            s1 = s1.add(1);
            s2 = s2.add(2);
        }
    }
}

#[allow(unused)]
#[repr(packed)]
pub struct TrapFrame {
    ra: u8,
    gp: u8,
    tp: u8,
    t0: u8,
    t1: u8,
    t2: u8,
    t3: u8,
    t4: u8,
    t5: u8,
    t6: u8,
    a0: u8,
    a1: u8,
    a2: u8,
    a3: u8,
    a4: u8,
    a5: u8,
    a6: u8,
    a7: u8,
    s0: u8,
    s1: u8,
    s2: u8,
    s3: u8,
    s4: u8,
    s5: u8,
    s6: u8,
    s7: u8,
    s8: u8,
    s9: u8,
    s10: u8,
    s11: u8,
    sp: u8,
}

#[unsafe(no_mangle)]
pub fn handle_trap(_trap_frame: &TrapFrame) {
    let scause = read_csr!(scause);
    let stval = read_csr!(stval);
    let sepc = read_csr!(sepc);
    panic!(
        "unexpected trap scause={:x}, stval={:x}, sepc={:x}\n",
        scause, stval, sepc
    );
}

pub fn stvec_handler() {
    unsafe {
        asm!(
            ".align 4",
            "csrw sscratch, sp",
            "addi sp, sp, -4 * 31",
            "sw ra,  4 * 0(sp)",
            "sw gp,  4 * 1(sp)",
            "sw tp,  4 * 2(sp)",
            "sw t0,  4 * 3(sp)",
            "sw t1,  4 * 4(sp)",
            "sw t2,  4 * 5(sp)",
            "sw t3,  4 * 6(sp)",
            "sw t4,  4 * 7(sp)",
            "sw t5,  4 * 8(sp)",
            "sw t6,  4 * 9(sp)",
            "sw a0,  4 * 10(sp)",
            "sw a1,  4 * 11(sp)",
            "sw a2,  4 * 12(sp)",
            "sw a3,  4 * 13(sp)",
            "sw a4,  4 * 14(sp)",
            "sw a5,  4 * 15(sp)",
            "sw a6,  4 * 16(sp)",
            "sw a7,  4 * 17(sp)",
            "sw s0,  4 * 18(sp)",
            "sw s1,  4 * 19(sp)",
            "sw s2,  4 * 20(sp)",
            "sw s3,  4 * 21(sp)",
            "sw s4,  4 * 22(sp)",
            "sw s5,  4 * 23(sp)",
            "sw s6,  4 * 24(sp)",
            "sw s7,  4 * 25(sp)",
            "sw s8,  4 * 26(sp)",
            "sw s9,  4 * 27(sp)",
            "sw s10, 4 * 28(sp)",
            "sw s11, 4 * 29(sp)",
            "csrr a0, sscratch",
            "sw a0, 4 * 30(sp)",
            "mv a0, sp",
            "call handle_trap",
            "lw ra,  4 * 0(sp)",
            "lw gp,  4 * 1(sp)",
            "lw tp,  4 * 2(sp)",
            "lw t0,  4 * 3(sp)",
            "lw t1,  4 * 4(sp)",
            "lw t2,  4 * 5(sp)",
            "lw t3,  4 * 6(sp)",
            "lw t4,  4 * 7(sp)",
            "lw t5,  4 * 8(sp)",
            "lw t6,  4 * 9(sp)",
            "lw a0,  4 * 10(sp)",
            "lw a1,  4 * 11(sp)",
            "lw a2,  4 * 12(sp)",
            "lw a3,  4 * 13(sp)",
            "lw a4,  4 * 14(sp)",
            "lw a5,  4 * 15(sp)",
            "lw a6,  4 * 16(sp)",
            "lw a7,  4 * 17(sp)",
            "lw s0,  4 * 18(sp)",
            "lw s1,  4 * 19(sp)",
            "lw s2,  4 * 20(sp)",
            "lw s3,  4 * 21(sp)",
            "lw s4,  4 * 22(sp)",
            "lw s5,  4 * 23(sp)",
            "lw s6,  4 * 24(sp)",
            "lw s7,  4 * 25(sp)",
            "lw s8,  4 * 26(sp)",
            "lw s9,  4 * 27(sp)",
            "lw s10, 4 * 28(sp)",
            "lw s11, 4 * 29(sp)",
            "lw sp,  4 * 30(sp)",
            "sret"
        );
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

    for place in ["world", "house", "friends"] {
        dprintln!("Hello {}!", place);
        dprintln!();
    }

    write_csr!(stvec, stvec_handler as *const ());
    unsafe {
        asm!("unimp");
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
