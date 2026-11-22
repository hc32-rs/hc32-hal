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
//! 启用恰好一个芯片 feature（如 `hc32l110c6ua`、`hc32l136k8ta`、
//! `hc32f420katb`）选择目标型号。构建期由 build.rs 从该料号的 PAC 元数据
//! 读取外设实例的 `registers.kind` 与 `registers.version`，派生后端 cfg
//! （如 `gpio_ddl_v2`、`sysctrl_ddl_v2_2`）与实例 cfg（`peri_uart0` 等），
//! 据此开放驱动模块与 [`Peripherals`] 字段。应用只选完整料号，不感知
//! IP 后端。
//!
//! 时钟能力按型号开放：HC32L110 的 [`Config::clock`] 可配置时钟树
//! （RCH 档位/晶振/分频，不配置时保持复位默认 RCH 4 MHz）；HC32L13x
//! 与 HC32F420 当前只接入默认路径（`init` 内装载 RCH/RCL 出厂 TRIM，
//! 系统时钟保持复位默认 4 MHz——L13x 为 RCH 4 MHz、F420 为 RCHCLKD
//! 4 MHz（RCH 24 MHz 经 RCHCR.DIV 分频））。

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
// - L110：sysctrl 驱动写晶振引脚（GPIO）与 FLASH 等待周期（FLASH），
//   三个实例 cfg 都在；
// - L13x：FLASH 布局为 ddl_v3_2（未适配驱动），sysctrl 驱动不触碰
//   FLASH 寄存器，后端依赖表相应只要求 GPIO。
#[cfg(all(sysctrl_ddl_v2_2, peri_clock, peri_flash, peri_gpio))]
pub mod clock;
#[cfg(all(sysctrl_ddl_v3_1, peri_sysctrl, peri_gpio))]
pub mod clock;
// F420：sysctrl_v4 默认路径只装载 TRIM 与 GPIO 门控，不触碰 GPIO/FLASH
// 寄存器，无组合实例要求。
#[cfg(all(sysctrl_ddl_v4, peri_sysctrl))]
pub mod clock;
#[cfg(all(gpio_ddl_v2, sysctrl_ddl_v2_2, peri_gpio))]
pub mod gpio;
#[cfg(all(gpio_ddl_v3_3, sysctrl_ddl_v3_1, peri_gpio))]
pub mod gpio;
// F420：gpio_ddl_v3_1 的发放已按实例核对 GPIOA~GPIOD 集合（build.rs
// 的 GPIO_DRIVER_BACKENDS）；门控经 sysctrl_ddl_v4。
#[cfg(all(
    gpio_ddl_v3_1,
    sysctrl_ddl_v4,
    peri_sysctrl,
    peri_gpioa,
    peri_gpiob,
    peri_gpioc,
    peri_gpiod
))]
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
#[cfg(all(feature = "hc32l13x", feature = "_chip-selected"))]
pub mod peripherals;
#[cfg(all(feature = "hc32f420", feature = "_chip-selected"))]
pub mod peripherals;
mod time;
pub use time::Hertz;

#[cfg(all(feature = "hc32l110", feature = "_chip-selected"))]
use peripherals::Peripherals;
#[cfg(all(feature = "hc32l13x", feature = "_chip-selected"))]
use peripherals::Peripherals;
#[cfg(all(feature = "hc32f420", feature = "_chip-selected"))]
use peripherals::Peripherals;

/// 初始化配置。
///
/// 仅开放 GPIO 等驱动、时钟模块未适配的型号没有 [`Config::clock`] 字段。
/// HC32L13x 当前只接入默认时钟路径（复位 RCH 4 MHz + 出厂 TRIM 装载，
/// 见 [`clock::init_default`]），没有可配置字段。
#[derive(Default)]
#[cfg(all(sysctrl_ddl_v2_2, feature = "_chip-selected"))]
pub struct Config {
    /// 时钟树配置。`None` = 保持复位默认（RCH 4 MHz，不分频）。
    pub clock: Option<clock::Config>,
}

/// HC32L13x 的初始化配置：仅默认时钟路径，无字段。
///
/// `Config::default()` 在 L110 上携带时钟树配置字段（`Option<clock
/// ::Config>`，`None` = 复位默认）；L13x 的可配置时钟树尚未接入，应用
/// 用空配置初始化即可，后续接入后在本类型上增加同名字段保持入口稳定。
#[derive(Default)]
#[cfg(all(sysctrl_ddl_v3_1, feature = "_chip-selected"))]
pub struct Config {}

/// HC32F420 的初始化配置：仅默认时钟路径，无字段。
///
/// 同 L13x：可配置时钟树（RM 3.2 的 4 源切换、PLL）尚未接入，应用用
/// 空配置初始化，接入后在本类型上增加同名字段保持入口稳定。
#[derive(Default)]
#[cfg(all(sysctrl_ddl_v4, feature = "_chip-selected"))]
pub struct Config {}

/// 初始化 HAL，一次性发放全部外设单例。
///
/// 先在临界区内领取外设单例（第二次调用在改动任何硬件前 panic），成功
/// 后执行：HC32L110 配置了 [`Config::clock`] 时进行时钟树初始化
/// （RM 3.2.1 标准切换流程）；无论是否配置时钟，均按数据手册要求为
/// 16 引脚封装未引出的 P03/P32/P33/P34 使能上拉（方向寄存器复位值已
/// 为输入）。
#[cfg(all(feature = "hc32l110", feature = "_chip-selected"))]
pub fn init(config: Config) -> Peripherals {
    let peripherals = Peripherals::take();
    if let Some(clock) = &config.clock {
        clock::init(clock);
    }
    init_pins();
    peripherals
}

/// 初始化 HAL（HC32L13x 默认路径），一次性发放全部外设单例。
///
/// 先在临界区内领取外设单例（第二次调用在改动任何硬件前 panic），随后
/// 依次执行（对齐厂商 `SystemInit`，Rust 启动路径不运行厂商代码）：
///
/// 1. 装载 RCH/RCL 出厂校准 TRIM（[`clock::init_default`]）；
/// 2. 按当前料号封装处理未引出脚（数字输入 + 上拉，SDK `_HidePinInit`
///    同源；DS 3.1 表下注与 RM 6.3）。
///
/// 系统时钟保持复位默认：RCH 4 MHz，HCLK = PCLK = 4 MHz。
#[cfg(all(feature = "hc32l13x", feature = "_chip-selected"))]
pub fn init(_config: Config) -> Peripherals {
    let peripherals = Peripherals::take();
    clock::init_default();
    init_pins();
    peripherals
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

/// 封装相关的引脚默认处理（HC32L13x，SDK `_HidePinInit` 同源）。
///
/// 除 64 引脚封装（K8TA，全部 IO 引出）外，各封装均有未引出 IO（见
/// `peripherals` 模块文档的封装差异表），按 `hc32l13x-<引脚数>pin`
/// 封装 feature 选择掩码；处理期间临时打开 GPIO 门控、结束后恢复原值
/// （SDK `_HidePinInit` 保存/恢复同做法）。
#[cfg(all(feature = "hc32l13x", feature = "_chip-selected"))]
fn init_pins() {
    // SDK `_HidePinInit`：保存 PERI_CLKEN → 置 GPIO 位 → 配未引出脚 →
    // 整写恢复。GPIO 门控复位为关（RM 3.5.9 复位值 0x8080_0000）。
    let saved = clock::sysctrl_v3_1::read_peri_clken();
    clock::enable_gpio();
    gpio::backend_pull_up_unbonded();
    clock::sysctrl_v3_1::write_peri_clken(saved);
}

/// 初始化 HAL（HC32F420 默认路径），一次性发放全部外设单例。
///
/// 先在临界区内领取外设单例（第二次调用在改动任何硬件前 panic），随后
/// 装载 RCH/RCL 出厂校准 TRIM（[`clock::init_default`]，对齐厂商
/// `SystemInit`；Rust 启动路径不运行厂商代码）。
///
/// 系统时钟保持复位默认：RCH 24 MHz 本振 6 分频（RCHCR 复位值 0x2，
/// DIV 编码 0010 = /6，RM 3.1.2/3.5.6）得 RCHCLKD 4 MHz，
/// HCLK = PCLK0 = PCLK1 = 4 MHz
/// （SYSCTRL_CR0 复位值 0x0000_0000）。LQFP64 封装引出全部 52 个 IO，
/// 无未引出脚处理：复位后各端口默认为模拟输入（RM 6.3.3），使用前由
/// 各 GPIO 驱动按需配置；门控在构造引脚时按端口打开。
#[cfg(all(feature = "hc32f420", feature = "_chip-selected"))]
pub fn init(_config: Config) -> Peripherals {
    let peripherals = Peripherals::take();
    clock::init_default();
    peripherals
}
