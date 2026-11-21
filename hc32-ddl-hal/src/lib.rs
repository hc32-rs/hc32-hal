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
//! 启用恰好一个芯片 feature（如 `hc32l110c6ua`）选择目标型号。构建期
//! 由 build.rs 从该料号的 PAC 元数据读取外设实例的 `registers.kind`
//! 与 `registers.version`，派生后端 cfg（如 `gpio_ddl_v2`、
//! `sysctrl_ddl_v2_2`）与实例 cfg（`peri_uart0` 等），据此开放驱动
//! 模块与 [`Peripherals`] 字段。应用只选完整料号，不感知 IP 后端。
//!
//! [`Config::clock`] 配置时钟树；不配置时芯片运行在复位默认的内部高速
//! RCH（4 MHz，HCLK = PCLK = 4 MHz），与不触碰时钟树的行为一致。

#![no_std]
// 零料号：所有驱动模块都被 cfg 关闭，死代码警告无意义。
#![cfg_attr(not(feature = "_chip-selected"), allow(unused))]

// 零料号：所有驱动模块都被 cfg 关闭，此诊断是唯一输出。
#[cfg(not(feature = "_chip-selected"))]
compile_error!(
    "hc32-ddl-hal: 请启用恰好一个芯片 feature，例如 hc32-ddl-hal = { features = [\"hc32l110c6ua\"] }"
);

// 当前芯片的寄存器访问层：料号 feature 激活 metapac 后按需重导出。
#[cfg(feature = "_chip-selected")]
pub use hc32_metapac as pac;

// 驱动模块按后端 cfg 开放：`<kind>_<version>` 与 `peri_<实例>` 均由
// build.rs 从当前料号的 PAC 元数据派生，GPIO 与 SYSCTRL 独立选择后端
// （AGENTS.md「从具体型号选择驱动」）。组合依赖在 build.rs 的
// DRIVER_BACKENDS 核对后才会发出对应后端 cfg。
#[cfg(all(sysctrl_ddl_v2_2, peri_clock, peri_flash, peri_gpio))]
pub mod clock;
#[cfg(all(gpio_ddl_v2, sysctrl_ddl_v2_2, peri_gpio))]
pub mod gpio;

// 以下驱动还含 HC32L110 手写的实例单例与引脚复用表（PAC 元数据暂缺
// 引脚/复用数据），随家族 feature 一并门控；数据迁入元数据后仅凭料号
// feature 即可开放，见 AGENTS.md 的迁移说明。
#[cfg(all(
    feature = "hc32l110",
    feature = "_chip-selected",
    adt_ddl_v1,
    peri_adt0,
    peri_adt1,
    peri_adt2
))]
pub mod adt;
#[cfg(all(
    feature = "hc32l110",
    feature = "_chip-selected",
    bt_ddl_v1,
    peri_bt0,
    peri_bt1,
    peri_bt2
))]
pub mod bt;
#[cfg(all(
    feature = "hc32l110",
    feature = "_chip-selected",
    lptim_ddl_v1,
    peri_lptimer
))]
pub mod lptim;
#[cfg(all(feature = "hc32l110", feature = "_chip-selected", pca_ddl_v1, peri_pca))]
pub mod pca;
// UART：波特率定时器占用 BT（RM 15.1/16.1）、引脚复用经 GPIO、门控
// 与频率经 SYSCTRL。
#[cfg(all(
    feature = "hc32l110",
    feature = "_chip-selected",
    uart_ddl_v1,
    lpuart_ddl_v1,
    bt_ddl_v1,
    gpio_ddl_v2,
    sysctrl_ddl_v2_2,
    peri_uart0,
    peri_uart1,
    peri_lpuart,
))]
pub mod uart;

// 纯寄存器驱动：无手写实例数据，仅按后端与实例 cfg 开放。
#[cfg(all(clk_trim_ddl_v1, peri_clk_trim))]
pub mod clk_trim;
#[cfg(all(crc_ddl_v1, peri_crc))]
pub mod crc;
#[cfg(all(wdt_ddl_v1, peri_wdt))]
pub mod wdt;

mod peripheral;
// 引脚单例与 Peripherals 集合当前按系列手写（PAC 元数据暂缺引脚
// 表），随家族 feature 门控；迁入元数据后按封装派生。
#[cfg(all(feature = "hc32l110", feature = "_chip-selected"))]
pub mod peripherals;
mod time;
pub use time::Hertz;

#[cfg(all(feature = "hc32l110", feature = "_chip-selected"))]
use peripherals::Peripherals;

/// 初始化配置。
///
/// 仅开放 GPIO 等驱动、时钟模块未适配的型号没有 [`Config::clock`] 字段。
#[derive(Default)]
#[cfg(all(feature = "hc32l110", feature = "_chip-selected"))]
pub struct Config {
    /// 时钟树配置。`None` = 保持复位默认（RCH 4 MHz，不分频）。
    pub clock: Option<clock::Config>,
}

/// 初始化 HAL，一次性发放全部外设单例。
///
/// 配置了 [`Config::clock`] 时先执行时钟树初始化（RM 3.2.1 标准切换流
/// 程），再发放外设。无论是否配置时钟，均按数据手册要求为 16 引脚封装
/// 未引出的 P03/P32/P33/P34 使能上拉（方向寄存器复位值已为输入）。
#[cfg(all(feature = "hc32l110", feature = "_chip-selected"))]
pub fn init(config: Config) -> Peripherals {
    if let Some(clock) = &config.clock {
        clock::init(clock);
    }
    init_pins();
    Peripherals::take()
}

/// 封装相关的引脚默认处理（HC32L110 手写引脚表）。
///
/// 未引出脚的上拉只适用于 16 引脚封装（TSSOP16/CSP16，`hc32l110-16pin`
/// 由 B 系列料号带出），按 DS 第 4 章表下注正向门控；20 引脚封装不做
/// 任何额外引脚处理。GPIO 门控经由当前型号的系统控制器打开。
#[cfg(all(feature = "hc32l110", feature = "_chip-selected"))]
fn init_pins() {
    #[cfg(feature = "hc32l110-16pin")]
    {
        // 16 引脚封装相对 TSSOP20 未引出的 IO 需设为输入并使能上拉
        // （DS 第 4 章表下注）。方向寄存器复位值为全输入，只补上拉。
        clock::enable_gpio();
        gpio::backend_pull_up_unbonded();
    }
}
