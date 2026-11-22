//! 引脚单例与外设集合：按系列转发。
//!
//! 两系列的 `Peripherals` 结构同源（单一声明宏派生），各自的手写引脚
//! 表与封装差异放在系列文件：HC32L110 见 [`peripherals_l110`]（DS 第
//! 4 章引脚表），HC32L13x 见 [`peripherals_l13x`]（DS Rev2.20 表 3-2）。
//! 引脚表迁入 PAC 元数据后由元数据按封装派生（AGENTS.md「从具体型号
//! 选择驱动」）。

#[cfg(feature = "hc32l110")]
mod peripherals_l110;
#[cfg(feature = "hc32l110")]
pub use peripherals_l110::Peripherals;
// 引脚单例类型随系列文件一并重导出（uart 等驱动按引脚名引用）。
#[cfg(feature = "hc32l110")]
pub use peripherals_l110::*;

#[cfg(feature = "hc32l13x")]
mod peripherals_l13x;
#[cfg(feature = "hc32l13x")]
pub use peripherals_l13x::Peripherals;
// 引脚单例类型随系列文件一并重导出（uart 等驱动按引脚名引用）。
#[cfg(feature = "hc32l13x")]
pub use peripherals_l13x::*;

#[cfg(feature = "hc32f420")]
mod peripherals_f420;
#[cfg(feature = "hc32f420")]
pub use peripherals_f420::Peripherals;
// 引脚单例类型随系列文件一并重导出（后续通信驱动按引脚名引用）。
#[cfg(feature = "hc32f420")]
pub use peripherals_f420::*;

use core::sync::atomic::{AtomicBool, Ordering};

/// 全部外设单例的独占占用标志，两系列共用。
///
/// Cortex-M0+ 没有原子读-改-写指令，分离的 load/store 若与中断处理
/// 交错，两侧都可能通过检查；检查与占用因此放在 PRIMASK 临界区内。
static TAKEN: AtomicBool = AtomicBool::new(false);

/// 两系列共用的单例领取入口：检查并占用独占标志，第二次调用 panic。
///
/// 检查与占用在同一临界区内完成，主程序与中断处理交错时也只会有一方
/// 领取成功；标志已占用时在临界区外 panic，不改变中断状态。
pub(crate) fn claim_peripherals() {
    let claimed = cortex_m::interrupt::free(|_| {
        if TAKEN.load(Ordering::Relaxed) {
            false
        } else {
            TAKEN.store(true, Ordering::Relaxed);
            true
        }
    });
    if !claimed {
        panic!("hc32-ddl-hal: peripherals already taken");
    }
}
