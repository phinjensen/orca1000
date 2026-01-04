#![no_std]
#![no_main]

use core::{
    arch::{asm, naked_asm},
    fmt::Write,
    panic::PanicInfo,
    ptr, slice,
    sync::atomic::{AtomicPtr, Ordering},
};

unsafe extern "C" {
    static mut __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;
    static mut __free_ram: u8;
    static mut __free_ram_end: u8;
    static __kernel_base: u8;
}

const SATP_SV32: usize = 1usize << 31;
const PAGE_V: usize = 1usize;
const PAGE_R: usize = 1usize << 1;
const PAGE_W: usize = 1usize << 2;
const PAGE_X: usize = 1usize << 3;
const PAGE_U: usize = 1usize << 4;

type PageTable = [usize; PAGE_SIZE];

fn map_page(table1: &mut PageTable, vaddr: *mut PageTable, paddr: *mut PageTable, flags: usize) {
    if !vaddr.is_aligned() {
        panic!("unaligned vaddr {:p}", vaddr);
    }
    if !paddr.is_aligned() {
        panic!("unaligned paddr {:p}", paddr);
    }
    let vpn1 = (vaddr as usize) >> 22 & 0b1111111111;
    if (table1[vpn1] & PAGE_V) == 0 {
        let page_table = alloc_pages(1);
        table1[vpn1] = (((page_table as usize) / PAGE_SIZE) << 10) | PAGE_V;
    }

    let vpn0 = (vaddr as usize) >> 12 & 0b1111111111;
    let table0 = unsafe {
        (((table1[vpn1] >> 10) * PAGE_SIZE) as *mut PageTable)
            .as_mut()
            .expect("table1[vpn1] must point to a valid page table")
    };
    table0[vpn0] = (((paddr as usize) / PAGE_SIZE) << 10) | flags | PAGE_V;
}

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

const PAGE_SIZE: usize = 4096;

#[allow(unused)]
fn alloc_pages(n: usize) -> *mut u8 {
    static NEXT_PADDR: AtomicPtr<u8> = AtomicPtr::new(&raw mut __free_ram);
    let paddr = NEXT_PADDR.load(Ordering::Relaxed);
    NEXT_PADDR.store(unsafe { paddr.add(n * PAGE_SIZE) }, Ordering::Relaxed);
    unsafe {
        memset(paddr, 0, n * PAGE_SIZE);
    }
    return paddr;
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
            // Retrieve the kernel stack of the running process from sscratch.
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
            // Retrieve and save the sp at the time of exception.
            "csrr a0, sscratch",
            "sw a0, 4 * 30(sp)",
            // Reset the kernel stack
            "addi a0, sp, 4 * 31",
            "csrw sscratch, a0",
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

const MAX_PROCESSES: usize = 8;
const PROCESS_STACK_SIZE: usize = 8192;

//enum ProcessState {
//    UNUSED,
//    RUNNABLE,
//}

struct Process {
    _pid: usize,
    //state: ProcessState,
    page_table: *mut PageTable,
    stack_pointer: *mut u8,
    stack: [u8; PROCESS_STACK_SIZE],
}

impl Process {
    fn new(_pid: usize) -> Self {
        Self {
            _pid,
            page_table: ptr::null_mut(),
            stack_pointer: ptr::null_mut(),
            stack: [0; PROCESS_STACK_SIZE],
        }
    }

    #[allow(unused)]
    fn stack_push_u8(&mut self, val: u8) {
        unsafe {
            self.stack_pointer = self.stack_pointer.sub(1);
            self.stack_pointer.write(val);
        }
    }

    fn stack_push_usize(&mut self, val: usize) {
        unsafe {
            let p = (self.stack_pointer as *mut usize).sub(1);
            p.write(val);
            self.stack_pointer = p as *mut u8;
        }
    }
}

#[unsafe(naked)]
unsafe extern "C" fn switch_context(previous: &*mut u8, next: &*mut u8) {
    naked_asm!(
        // Save callee-saved registers onto the current process's stack.
        "addi sp, sp, -13 * 4", // Allocate stack space for 13 4-byte registers
        "sw ra,  0  * 4(sp)",   // Save callee-saved registers only
        "sw s0,  1  * 4(sp)",
        "sw s1,  2  * 4(sp)",
        "sw s2,  3  * 4(sp)",
        "sw s3,  4  * 4(sp)",
        "sw s4,  5  * 4(sp)",
        "sw s5,  6  * 4(sp)",
        "sw s6,  7  * 4(sp)",
        "sw s7,  8  * 4(sp)",
        "sw s8,  9  * 4(sp)",
        "sw s9,  10 * 4(sp)",
        "sw s10, 11 * 4(sp)",
        "sw s11, 12 * 4(sp)",
        // Switch the stack pointer.
        "sw sp, (a0)", // *prev_sp = sp;
        "lw sp, (a1)", // Switch stack pointer (sp) here
        // Restore callee-saved registers from the next process's stack.
        "lw ra,  0  * 4(sp)", // Restore callee-saved registers only
        "lw s0,  1  * 4(sp)",
        "lw s1,  2  * 4(sp)",
        "lw s2,  3  * 4(sp)",
        "lw s3,  4  * 4(sp)",
        "lw s4,  5  * 4(sp)",
        "lw s5,  6  * 4(sp)",
        "lw s6,  7  * 4(sp)",
        "lw s7,  8  * 4(sp)",
        "lw s8,  9  * 4(sp)",
        "lw s9,  10 * 4(sp)",
        "lw s10, 11 * 4(sp)",
        "lw s11, 12 * 4(sp)",
        "addi sp, sp, 13 * 4", // We've popped 13 4-byte registers from the stack
        "ret"
    );
}

struct ProcessHandler {
    idle_process: usize,
    current_process: usize,
    processes: [Option<Process>; MAX_PROCESSES],
}

impl ProcessHandler {
    fn new() -> Self {
        Self {
            idle_process: 0,
            current_process: 0,
            processes: [const { None }; MAX_PROCESSES],
        }
    }

    fn create_process(&mut self, pc: *const u8) -> usize {
        let (pid, slot) = self
            .processes
            .iter_mut()
            .enumerate()
            .find(|(_, p)| p.is_none())
            .expect("no free process slots");
        let new_process = Process::new(pid);
        *slot = Some(new_process);
        let new_process = (*slot).as_mut().unwrap();
        new_process.stack_pointer =
            unsafe { (&raw mut new_process.stack as *mut u8).add(new_process.stack.len()) };
        for _ in 0..12 {
            new_process.stack_push_usize(0); //s0-s11
        }
        new_process.stack_push_usize(pc as usize);

        let page_table = alloc_pages(1) as *mut PageTable;

        let mut paddr = &raw const __kernel_base as *const usize;
        while paddr < &raw const __free_ram_end as *const usize {
            map_page(
                unsafe { page_table.as_mut().unwrap() },
                paddr as *mut PageTable,
                paddr as *mut PageTable,
                PAGE_R | PAGE_W | PAGE_X,
            );
            unsafe {
                paddr = paddr.add(1);
            }
        }

        new_process.page_table = page_table;
        pid
    }

    //fn get_process(&self, pid: usize) -> &Option<Process> {
    //    return self.processes.get(pid).unwrap_or(&None);
    //}
}

fn delay() {
    let mut i = 0;
    loop {
        if i >= 30000000 {
            break;
        }
        unsafe { asm!("nop") };
        i += 1;
    }
}

static PROCESS_HANDLER: AtomicPtr<ProcessHandler> = AtomicPtr::new(ptr::null_mut());

fn _yield() {
    let ph = unsafe { PROCESS_HANDLER.load(Ordering::Relaxed).as_mut() }.unwrap();
    let back_half = ph.processes.iter().enumerate().skip(ph.current_process + 1);
    let front_half = ph.processes.iter().enumerate().take(ph.current_process);
    if let Some((i, next)) = back_half.chain(front_half).find(|(_, p)| p.is_some()) {
        let previous = &ph.processes[ph.current_process]
            .as_ref()
            .expect("ProcessHandler.current_process should point to a process");
        ph.current_process = i;
        let next = &next.as_ref().unwrap();
        unsafe {
            asm!(
                "sfence.vma",
                "csrw satp, {}",
                "sfence.vma",
                "csrw sscratch, {}",
                in(reg) SATP_SV32 | ((next.page_table as usize) / PAGE_SIZE),
                in(reg) (&raw const next.stack).add(next.stack.len())
            );
            switch_context(&previous.stack_pointer, &next.stack_pointer);
        }
    }
}

fn proc_a_entry() {
    dprintln!("starting process A");
    loop {
        putchar('A' as u8);
        _yield();
        delay();
    }
}

fn proc_b_entry() {
    dprintln!("starting process B");
    loop {
        putchar('B' as u8);
        _yield();
        delay();
    }
}

#[unsafe(no_mangle)]
pub fn kernel_main() -> ! {
    write_csr!(stvec, stvec_handler as *const ());

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

    let mut ph = ProcessHandler::new();
    PROCESS_HANDLER.store(&raw mut ph, Ordering::Relaxed);
    ph.create_process(proc_a_entry as *const u8);
    ph.create_process(proc_b_entry as *const u8);
    proc_a_entry();

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
