//! 中断模块
//!
//!

mod context;
mod handler;
pub(crate) mod timer;

pub use context::Context;

/// 初始化中断相关的子模块
///
/// - [`handler::init`]
/// - [`timer::init`]
pub fn init() {
    handler::init();
    timer::init();
    println!("mod interrupt initialized");
}
