//! # 全局属性
//!
//! - `#![no_std]`
//!   禁用标准库
#![no_std]
//!
//! - `#![no_main]`
//!   不使用 `main` 函数等全部 Rust-level 入口点来作为程序入口
#![no_main]
//! # 一些 unstable 的功能需要在 crate 层级声明后才可以使用
//!
//! - `#![feature(alloc_error_handler)]`
//!   我们使用了一个全局动态内存分配器，以实现原本标准库中的堆内存分配。
//!   而语言要求我们同时实现一个错误回调，这里我们直接 panic
#![feature(alloc_error_handler)]
//! # 一些 unstable 的功能需要在 crate 层级声明后才可以使用
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
//! - `#![feature(slice_fill)]`
//!   允许将 slice 填充值
#![feature(slice_fill)]

#[macro_use]
mod console;
mod interrupt;
mod memory;
mod panic;
mod sbi;

extern crate alloc;

// 汇编编写的程序入口，具体见该文件
global_asm!(include_str!("entry.asm"));

/// Rust 的入口函数
///
/// 在 `_start` 为我们进行了一系列准备之后，这是第一个被调用的 Rust 函数
#[no_mangle]
pub extern "C" fn rust_main() {
    interrupt::init();
    memory::init();

    let mut remap = memory::mapping::MemorySet::new_kernel().unwrap();
    remap.activate();

    println!("kernel remapped");

    use memory::{mapping::*, *};
    let entry = VirtualAddress(rust_main as usize);
    assert_eq!(Mapping::lookup(entry), Some(PhysicalAddress::from(entry)));
    assert_eq!(Mapping::lookup(VirtualAddress(0)), None);
    let segment = Segment {
        map_type: MapType::Framed,
        range: Range::from(VirtualAddress(0x1000_0011)..VirtualAddress(0x1000_3000)),
        flags: Flags::READABLE | Flags::WRITABLE,
    };
    let data = alloc::vec![0x5au8; 5000];
    remap.add_segment(segment, Some(&data)).unwrap();
    remap.activate();
    for offset in 0..segment.range.len() {
        let va = segment.range.start + offset;
        assert_eq!(*va.deref::<u8>(), if offset < data.len() { 0x5a } else { 0 });
        let pa = Mapping::lookup(va).unwrap();
        assert_eq!(pa.page_offset(), va.page_offset());
        assert_eq!(*pa.deref_kernel::<u8>(), *va.deref::<u8>());
    }
    remap.remove_segment(&segment).unwrap();
    remap.activate();
    for offset in 0..3 {
        assert_eq!(Mapping::lookup(VirtualAddress(0x1000_0000 + offset * PAGE_SIZE)), None);
    }
    println!("mapping tests passed");
    println!("LAB3 SUCCESS");
    sbi::shutdown();
}
