//! HC32 DDL 系列微控制器的硬件抽象层。
//!
//! API 风格与代码组织参照 Embassy STM32 HAL，寄存器语义以小华半导体的
//! DS/RM 手册与 DDL SDK 为准。使用入口：
//!
//! ```no_run
//! let p = hc32_ddl_hal::init(hc32_ddl_hal::Config::default());
//! let mut led = hc32_ddl_hal::gpio::Output::new(
//!     p.P14,
//!     hc32_ddl_hal::gpio::Level::High,
//!     hc32_ddl_hal::gpio::Drive::High,
//! );
//! ```
//!
//! 启用恰好一个芯片 feature（如 `hc32l110c6ua`）选择目标型号。
//!
//! [`Config::clock`] 配置时钟树；不配置时芯片运行在复位默认的内部高速
//! RCH（4 MHz，HCLK = PCLK = 4 MHz），与不触碰时钟树的行为一致。

#![no_std]
#![cfg_attr(not(feature = "hc32l110"), allow(unused))]

#[cfg(not(feature = "hc32l110"))]
compile_error!(
    "hc32-ddl-hal: 请启用恰好一个芯片 feature，例如 hc32-ddl-hal = { features = [\"hc32l110c6ua\"] }"
);

pub mod adt;
pub mod bt;
pub mod clk_trim;
pub mod clock;
pub mod crc;
pub mod gpio;
pub mod lptim;
pub mod pca;
mod peripheral;
pub mod peripherals;
#[cfg(feature = "hc32l110")]
pub mod pac {
    //! 当前芯片的寄存器访问层。
    pub use hc32_metapac::*;
}

pub mod uart;
pub mod wdt;

mod time;
pub use time::Hertz;

use peripherals::Peripherals;

/// 初始化配置。
#[derive(Default)]
pub struct Config {
    /// 时钟树配置。`None` = 保持复位默认（RCH 4 MHz，不分频）。
    pub clock: Option<clock::Config>,
}

/// 初始化 HAL，一次性发放全部外设单例。
///
/// 配置了 [`Config::clock`] 时先执行时钟树初始化（RM 3.2.1 标准切换流
/// 程），再发放外设。无论是否配置时钟，均按数据手册要求为 16 引脚封装
/// 未引出的 P03/P32/P33/P34 使能上拉（方向寄存器复位值已为输入）。
pub fn init(config: Config) -> Peripherals {
    if let Some(clock) = &config.clock {
        clock::init(clock);
    }
    init_pins();
    Peripherals::take()
}

/// 封装相关的引脚默认处理。
fn init_pins() {
    // 16 引脚封装未引出的 IO 需设为输入并使能上拉（DS 第 4 章表下注）。
    // 方向寄存器复位值为全输入，这里只补上拉。
    #[cfg(not(feature = "hc32l110-full-pins"))]
    {
        clock::enable_gpio();
        // P03 / P32 / P33 / P34
        pac::GPIO.p0pu().modify(|w| w.set_p03(true));
        pac::GPIO.p3pu().modify(|w| {
            w.set_p32(true);
            w.set_p33(true);
            w.set_p34(true);
        });
    }
}
