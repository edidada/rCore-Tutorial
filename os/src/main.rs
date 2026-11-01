//! # 全局属性
//!
//! - `#![no_std]`
//!   禁用标准库
#![no_std]
//!
//! - `#![no_main]`
//!   不使用 `main` 函数等全部 Rust-level 入口点来作为程序入口
#![no_main]
//!
//! - `#![deny(missing_docs)]`
//!   任何没有注释的地方都会产生警告：这个属性用来压榨写实验指导的学长，同学可以删掉了
#![warn(missing_docs)]
//! # 一些 unstable 的功能需要在 crate 层级声明后才可以使用
//!
//! - `#![feature(alloc_error_handler)]`
//!   我们使用了一个全局动态内存分配器，以实现原本标准库中的堆内存分配。
//!   而语言要求我们同时实现一个错误回调，这里我们直接 panic
#![feature(alloc_error_handler)]
//!
//! - `#![feature(llvm_asm)]`
//!   内嵌汇编
#![feature(llvm_asm)]
//!
//! - `#![feature(global_asm)]`
//!   内嵌整个汇编文件
#![feature(global_asm)]
//!
//! - `#![feature(panic_info_message)]`
//!   panic! 时，获取其中的信息并打印
#![feature(panic_info_message)]
//!
//! - `#![feature(naked_functions)]`
//!   允许使用 naked 函数，即编译器不在函数前后添加出入栈操作。
//!   这允许我们在函数中间内联汇编使用 `ret` 提前结束，而不会导致栈出现异常
#![feature(naked_functions)]
//!
//! - `#![feature(slice_fill)]`
//!   允许将 slice 填充值
#![feature(slice_fill)]

#[macro_use]
mod console;
mod drivers;
mod fs;
pub mod interrupt;
mod kernel;
mod memory;
mod panic;
mod process;
mod sbi;
mod board;

extern crate alloc;

use alloc::sync::Arc;
use fs::{INodeExt, ROOT_INODE};
use memory::PhysicalAddress;
use process::*;
use xmas_elf::ElfFile;

// 汇编编写的程序入口，具体见该文件
global_asm!(include_str!("entry.asm"));

/// Rust 的入口函数
///
/// 在 `_start` 为我们进行了一系列准备之后，这是第一个被调用的 Rust 函数
#[no_mangle]
pub extern "C" fn rust_main(_hart_id: usize, dtb_pa: PhysicalAddress) -> ! {
    memory::clear_bss();
    memory::init();
    interrupt::init();
    crate::board::device_init(dtb_pa);
    fs::init();

    memory::mapping::clock_self_test();
    PROCESSOR.lock().add_thread(create_kernel_thread(
        Process::new_kernel().unwrap(), test_page_fault as usize, None,
    ));

    extern "C" {
        fn __restore(context: usize);
    }
    // 获取第一个线程的 Context
    let context = PROCESSOR.lock().prepare_next_thread();

    // 启动第一个线程
    unsafe {
        llvm_asm!("fence.i" :::: "volatile");
        __restore(context as usize);
    }
    unreachable!()
}

/// 测试缺页异常处理
///
/// 为了便于体现效果，只给每个线程分配了很低的可用物理页面限额，见 [`KERNEL_PROCESS_FRAME_QUOTA`]
///
/// [`KERNEL_PROCESS_FRAME_QUOTA`]: memory::config::KERNEL_PROCESS_FRAME_QUOTA
fn test_page_fault() {
    // 栈上 64 页数据，物理配额只有 16 页，必须真正换入换出。
    let mut array = [0usize; 32 * 1024];
    for round in 0..3 {
        for i in 0..array.len() {
            unsafe { core::ptr::write_volatile(&mut array[i], i ^ (round * 0x12345)); }
        }
        for i in (0..array.len()).rev() {
            assert_eq!(unsafe { core::ptr::read_volatile(&array[i]) }, i ^ (round * 0x12345));
        }
    }
    let faults = memory::mapping::PAGE_FAULTS.load(core::sync::atomic::Ordering::Relaxed);
    assert!(faults > 64);
    println!("Clock page faults: {}", faults);
    println!("LAB3+ SUCCESS");
    sbi::shutdown();
}

/// 创建一个内核进程
pub fn create_kernel_thread(
    process: Arc<Process>,
    entry_point: usize,
    arguments: Option<&[usize]>,
) -> Arc<Thread> {
    // 创建线程
    let thread = Thread::new(process, entry_point, arguments).unwrap();
    // 设置线程的返回地址为 kernel_thread_exit
    thread
        .as_ref()
        .inner()
        .context
        .as_mut()
        .unwrap()
        .set_ra(kernel_thread_exit as usize);

    thread
}

/// 创建一个用户进程，从指定的文件名读取 ELF
pub fn create_user_process(name: &str) -> Arc<Thread> {
    // 从文件系统中找到程序
    let app = ROOT_INODE.find(name).unwrap();
    // 读取数据
    let data = app.readall().unwrap();
    // 解析 ELF 文件
    let elf = ElfFile::new(data.as_slice()).unwrap();
    // 利用 ELF 文件创建线程，映射空间并加载数据
    let process = Process::from_elf(&elf, true).unwrap();
    // 再从 ELF 中读出程序入口地址
    Thread::new(process, elf.header.pt2.entry_point() as usize, None).unwrap()
}

/// 内核线程需要调用这个函数来退出
fn kernel_thread_exit() {
    // 当前线程标记为结束
    PROCESSOR.lock().current_thread().as_ref().inner().dead = true;
    // 制造一个中断来交给操作系统处理
    unsafe { llvm_asm!("ebreak" :::: "volatile") };
}
