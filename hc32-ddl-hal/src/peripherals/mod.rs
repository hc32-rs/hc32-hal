//! 引脚单例与外设集合：按系列转发。
//!
//! 两系列的 `Peripherals` 结构同源（单一声明宏派生），各自的手写引脚
//! 表与封装差异放在系列文件：HC32L110 见 [`peripherals_l110`]（DS 第
//! 4 章引脚表），HC32L130 见 [`peripherals_l130`]（DS Rev2.20 表 3-2）。
//! 引脚表迁入 PAC 元数据后由元数据按封装派生（AGENTS.md「从具体型号
//! 选择驱动」）。

#[cfg(feature = "hc32l110")]
mod peripherals_l110;
#[cfg(feature = "hc32l110")]
pub use peripherals_l110::Peripherals;
// 引脚单例类型随系列文件一并重导出（uart 等驱动按引脚名引用）。
#[cfg(feature = "hc32l110")]
pub use peripherals_l110::*;

#[cfg(feature = "hc32l130")]
mod peripherals_l130;
#[cfg(feature = "hc32l130")]
pub use peripherals_l130::Peripherals;
#[cfg(feature = "hc32l130")]
pub use peripherals_l130::*;
