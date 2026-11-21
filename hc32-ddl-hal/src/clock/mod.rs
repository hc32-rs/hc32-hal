//! 系统时钟。
//!
//! 模块入口保留应用可见的配置与查询 API：[`Config`] 与 [`init`]、门控
//! [`enable_peripheral`]、频率快照 [`clocks`]、SysTick 时钟与 HCLK/PCLK
//! 波形输出。寄存器操作按 IP 后端拆分：当前为 SYSCTRL `ddl_v2_2`
//! （HC32L110 的 `CLOCK` 外设，[`sysctrl_v2_2`]）。接入第二种布局时沿
//! 用本入口的公开类型，仅替换寄存器访问（AGENTS.md「HAL 源码布局与
//! 命名」）。
//!
//! HC32L110 的时钟控制模块提供 4 个系统时钟源（`SYSCTRL0.CLK_SW4_SEL`，
//! 2 位，RM 3.5.1）：
//!
//! - `RCH`：内部高速 RC，4/8/16/22.12/24 MHz 出厂校准档位（RCH_CR.TRIM）；
//! - `XTH`：外部高速晶振 4~32 MHz，或 P01 引脚外部输入；
//! - `RCL`：内部低速 RC，32.768/38.4 kHz；
//! - `XTL`：外部低速 32.768 kHz 晶振，或 P14 引脚外部输入。
//!
//! 本芯片**没有 PLL**（DS/RM/DDL SDK 三方一致）。
//!
//! 初始化入口 [`init`] 按 RM 3.2.1 标准时钟切换流程执行：配置引脚与振荡
//! 参数、使能源并等待稳定、按新旧时钟源中较高频率设置 FLASH 等待周期
//! （FLASH_BYPASS 解锁后写 FLASH_CR.WAIT）、切换 `CLK_SW4_SEL`、按新源
//! 频率复配等待周期、设置分频、关闭不再使用的源。
//!
//! [`sysctrl_v2_2`]: self::sysctrl_v2_2

mod sysctrl_v2_2;

use core::sync::atomic::{AtomicU32, Ordering};

use crate::time::Hertz;
use sysctrl_v2_2 as backend;

// ---------------------------------------------------------------------------
// 时钟源与档位枚举
// ---------------------------------------------------------------------------

/// 系统时钟源（SYSCTRL0.CLK_SW4_SEL，RM 3.5.1 位 5:4）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Sysclk {
    /// 内部高速 RC 时钟（编码 0，复位默认）。
    Rch,
    /// 外部高速晶振（编码 1）。
    Xth,
    /// 内部低速 RC 时钟（编码 2）。
    Rcl,
    /// 外部低速晶振（编码 3）。
    Xtl,
}

impl Sysclk {
    /// 寄存器编码。
    pub const fn encode(self) -> u8 {
        match self {
            Sysclk::Rch => 0,
            Sysclk::Xth => 1,
            Sysclk::Rcl => 2,
            Sysclk::Xtl => 3,
        }
    }

    /// 从寄存器编码还原，非法值返回 `None`。
    pub const fn decode(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Sysclk::Rch),
            1 => Some(Sysclk::Xth),
            2 => Some(Sysclk::Rcl),
            3 => Some(Sysclk::Xtl),
            _ => None,
        }
    }
}

/// RCH 输出频率档位。
///
/// 每档对应 flash 出厂校准区（0x00100C00~0x00100C08，RM 3.5.4 TRIM 字段
/// 描述）的一组 TRIM 值；写入 RCH_CR.TRIM 即得到该频率。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RchFreq {
    /// 24 MHz（校准值 @0x00100C00）。
    Mhz24,
    /// 22.12 MHz（校准值 @0x00100C02）。
    Mhz22_12,
    /// 16 MHz（校准值 @0x00100C04）。
    Mhz16,
    /// 8 MHz（校准值 @0x00100C06）。
    Mhz8,
    /// 4 MHz（校准值 @0x00100C08），复位默认。
    Mhz4,
}

impl RchFreq {
    /// 出厂校准值地址（RM 3.5.4）。
    const fn trim_addr(self) -> *const u16 {
        match self {
            RchFreq::Mhz24 => 0x0010_0C00 as *const u16,
            RchFreq::Mhz22_12 => 0x0010_0C02 as *const u16,
            RchFreq::Mhz16 => 0x0010_0C04 as *const u16,
            RchFreq::Mhz8 => 0x0010_0C06 as *const u16,
            RchFreq::Mhz4 => 0x0010_0C08 as *const u16,
        }
    }

    /// 读取出厂校准 TRIM 值。
    ///
    /// 地址位于 flash 校准数据区（RM 存储映射 0x00100C00~0x00100C3B，
    /// Trim Data），厂商保证有数据。
    fn trim_value(self) -> u16 {
        // safety: 芯片固定的已映射只读区，无副作用。
        unsafe { self.trim_addr().read_volatile() }
    }

    /// 该档位的标称输出频率。
    pub const fn frequency(self) -> Hertz {
        match self {
            RchFreq::Mhz24 => Hertz::mhz(24),
            RchFreq::Mhz22_12 => Hertz(22_120_000),
            RchFreq::Mhz16 => Hertz::mhz(16),
            RchFreq::Mhz8 => Hertz::mhz(8),
            RchFreq::Mhz4 => Hertz::mhz(4),
        }
    }

    /// 从 RCH_CR.TRIM 原始值推断档位。
    ///
    /// TRIM[10:9] 区分 4/8/16/24 档；22.12M 与 24M 共享编码，需比对出厂
    /// 校准值才能区分（SDK `Clk_GetHClkFreq` 同做法）。
    pub fn from_trim(trim: u16) -> Self {
        match (trim >> 9) & 0x3 {
            0 => RchFreq::Mhz4,
            1 => RchFreq::Mhz8,
            2 => RchFreq::Mhz16,
            // 与 22.12M 出厂校准值一致则判为 22.12M，否则 24M。
            _ if trim == RchFreq::Mhz22_12.trim_value() => RchFreq::Mhz22_12,
            _ => RchFreq::Mhz24,
        }
    }

    /// RM 3.2.7 方案 1 的逐级换频档位：4/8/16 为独立级，22.12M 与 24M
    /// 同为顶档。
    const fn step_level(self) -> u8 {
        match self {
            RchFreq::Mhz4 => 0,
            RchFreq::Mhz8 => 1,
            RchFreq::Mhz16 => 2,
            RchFreq::Mhz22_12 | RchFreq::Mhz24 => 3,
        }
    }
}

/// RCL 输出频率档位（RCL_CR.TRIM，校准值 @0x00100C20/22，RM 3.5.6）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RclFreq {
    /// 38.4 kHz（校准值 @0x00100C20，TRIM 位 9 = 1）。
    K38_4,
    /// 32.768 kHz（校准值 @0x00100C22，TRIM 位 9 = 0）。
    K32_768,
}

impl RclFreq {
    /// 出厂校准值地址（RM 3.5.6）。
    const fn trim_addr(self) -> *const u16 {
        match self {
            RclFreq::K38_4 => 0x0010_0C20 as *const u16,
            RclFreq::K32_768 => 0x0010_0C22 as *const u16,
        }
    }

    /// 读取出厂校准 TRIM 值。
    fn trim_value(self) -> u16 {
        // safety: 芯片固定的已映射只读区，无副作用。
        unsafe { self.trim_addr().read_volatile() }
    }

    /// 标称输出频率。
    pub const fn frequency(self) -> Hertz {
        match self {
            RclFreq::K38_4 => Hertz(38_400),
            RclFreq::K32_768 => Hertz(32_768),
        }
    }
}

/// XTH 工作频率范围（XTH_CR.DRIVER[3:2]，RM 3.5.5）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum XthFreqRange {
    /// 4~8 MHz（编码 00）。
    Mhz4_8,
    /// 8~16 MHz（编码 01）。
    Mhz8_16,
    /// 16~24 MHz（编码 10）。
    Mhz16_24,
    /// 24~32 MHz（编码 11）。
    Mhz24_32,
}

impl XthFreqRange {
    const fn encode(self) -> u8 {
        match self {
            XthFreqRange::Mhz4_8 => 0,
            XthFreqRange::Mhz8_16 => 1,
            XthFreqRange::Mhz16_24 => 2,
            XthFreqRange::Mhz24_32 => 3,
        }
    }
}

/// XTH 晶振驱动能力（XTH_CR.DRIVER[1:0]，RM 3.5.5）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum XthDriver {
    /// 最弱驱动（编码 00）。
    Weakest,
    /// 弱驱动（编码 01）。
    Weak,
    /// 默认驱动（编码 10，推荐值）。
    Default,
    /// 最强驱动（编码 11）。
    Strongest,
}

impl XthDriver {
    const fn encode(self) -> u8 {
        match self {
            XthDriver::Weakest => 0,
            XthDriver::Weak => 1,
            XthDriver::Default => 2,
            XthDriver::Strongest => 3,
        }
    }
}

/// XTL 振荡幅度档位（XTL_CR.DRIVER[3:2]，RM 3.5.7）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum XtlAmplitude {
    /// 最小振幅（编码 00）。
    Min,
    /// 正常振幅（编码 01）。
    Normal,
    /// 较大振幅（编码 10，推荐值）。
    Large,
    /// 最大振幅（编码 11）。
    Max,
}

impl XtlAmplitude {
    const fn encode(self) -> u8 {
        match self {
            XtlAmplitude::Min => 0,
            XtlAmplitude::Normal => 1,
            XtlAmplitude::Large => 2,
            XtlAmplitude::Max => 3,
        }
    }
}

/// XTL 晶振驱动能力（XTL_CR.DRIVER[1:0]，RM 3.5.7）。
///
/// 注意编码与 XTH 不同：XTL 的默认档为 01。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum XtlDriver {
    /// 最弱驱动（编码 00）。
    Weakest,
    /// 默认驱动（编码 01，推荐值）。
    Default,
    /// 较强驱动（编码 10）。
    Stronger,
    /// 最强驱动（编码 11）。
    Strongest,
}

impl XtlDriver {
    const fn encode(self) -> u8 {
        match self {
            XtlDriver::Weakest => 0,
            XtlDriver::Default => 1,
            XtlDriver::Stronger => 2,
            XtlDriver::Strongest => 3,
        }
    }
}

/// 晶振稳定时间档位（XTH_CR/XTL_CR.STARTUP，RM 3.5.5/3.5.7）。
///
/// RM 3.5.5 强烈建议 XTH 使用 `Cycles16384`。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CrystalStartup {
    /// 256 周期（编码 00）。
    Cycles256,
    /// 1024 周期（编码 01）。
    Cycles1024,
    /// 4096 周期（编码 10）。
    Cycles4096,
    /// 16384 周期（编码 11），XTH 推荐值。
    Cycles16384,
}

impl CrystalStartup {
    const fn encode(self) -> u8 {
        match self {
            CrystalStartup::Cycles256 => 0,
            CrystalStartup::Cycles1024 => 1,
            CrystalStartup::Cycles4096 => 2,
            CrystalStartup::Cycles16384 => 3,
        }
    }
}

/// RCL 稳定时间档位（RCL_CR.STARTUP，RM 3.5.6 位 11:10）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RclStartup {
    /// 4 周期（编码 00）。
    Cycles4,
    /// 16 周期（编码 01）。
    Cycles16,
    /// 64 周期（编码 10）。
    Cycles64,
    /// 256 周期（编码 11）。
    Cycles256,
}

impl RclStartup {
    const fn encode(self) -> u8 {
        match self {
            RclStartup::Cycles4 => 0,
            RclStartup::Cycles16 => 1,
            RclStartup::Cycles64 => 2,
            RclStartup::Cycles256 => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// 分频枚举
// ---------------------------------------------------------------------------

/// HCLK 分频（SYSCTRL0.HCLK_PRS，RM 3.5.1 位 8:6）。
///
/// 判别值即寄存器编码（log2 分频比）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum HclkPrescaler {
    /// SystemClk/1（编码 0，复位默认）。
    #[default]
    Div1 = 0,
    /// SystemClk/2。
    Div2 = 1,
    /// SystemClk/4。
    Div4 = 2,
    /// SystemClk/8。
    Div8 = 3,
    /// SystemClk/16。
    Div16 = 4,
    /// SystemClk/32。
    Div32 = 5,
    /// SystemClk/64。
    Div64 = 6,
    /// SystemClk/128。
    Div128 = 7,
}

impl crate::time::Prescaler for HclkPrescaler {
    fn num(&self) -> u32 {
        1
    }

    fn denom(&self) -> u32 {
        1u32 << (*self as u32)
    }
}

/// PCLK 分频（SYSCTRL0.PCLK_PRS，RM 3.5.1 位 10:9）。
///
/// 判别值即寄存器编码（log2 分频比）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum PclkPrescaler {
    /// HCLK/1（编码 0，复位默认）。
    #[default]
    Div1 = 0,
    /// HCLK/2。
    Div2 = 1,
    /// HCLK/4。
    Div4 = 2,
    /// HCLK/8。
    Div8 = 3,
}

impl crate::time::Prescaler for PclkPrescaler {
    fn num(&self) -> u32 {
        1
    }

    fn denom(&self) -> u32 {
        1u32 << (*self as u32)
    }
}

// ---------------------------------------------------------------------------
// 晶振配置
// ---------------------------------------------------------------------------

/// XTH 配置。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct XthConfig {
    /// 晶振工作频率范围，选取 DRIVER[3:2] 编码。
    pub freq_range: XthFreqRange,
    /// 驱动能力（DRIVER[1:0]）。
    pub driver: XthDriver,
    /// 稳定时间档位（STARTUP）。
    pub startup: CrystalStartup,
    /// true = P01 引脚外部时钟输入（SYSCTRL1.EXTH_EN），false = 晶振。
    pub bypass: bool,
    /// 晶振频率或外部输入频率。
    pub freq: Hertz,
}

impl XthConfig {
    /// 晶振模式。
    ///
    /// 频率范围按 RM 3.5.5 分段选取，边界按 DS 7.3.7 推荐编码归属：
    /// 8M→0b0010（4~8M 段）、12M→0b0110（8~16M 段）、16M→0b1010
    /// （16~24M 段）、24M/32M→0b1110（24~32M 段）；驱动统一取默认档 10。
    pub fn crystal(freq: Hertz) -> Self {
        debug_assert!(
            (4_000_000..=32_000_000).contains(&freq.0),
            "XTH 晶振频率超出 4~32MHz"
        );
        // RM 3.5.5 分段：00:4~8M、01:8~16M、10:16~24M、11:24~32M；
        // 8M 边界按 DS 7.3.7 推荐编码 0b0010 归 4~8M 段（审阅修订），
        // 16M 归 16~24M、24M 归 24~32M 段（同按 DS 推荐编码）。
        let freq_range = match freq.0 {
            0..=8_000_000 => XthFreqRange::Mhz4_8,
            8_000_001..16_000_000 => XthFreqRange::Mhz8_16,
            16_000_000..=24_000_000 => XthFreqRange::Mhz16_24,
            _ => XthFreqRange::Mhz24_32,
        };
        Self {
            freq_range,
            driver: XthDriver::Default,
            startup: CrystalStartup::Cycles16384,
            bypass: false,
            freq,
        }
    }

    /// 外部输入模式：P01 输入 4~32 MHz（RM 3.1.4），P02 需配为模拟态。
    pub fn bypass(freq: Hertz) -> Self {
        debug_assert!(
            (4_000_000..=32_000_000).contains(&freq.0),
            "XTH 外部输入频率超出 4~32MHz"
        );
        Self {
            freq_range: XthFreqRange::Mhz4_8,
            driver: XthDriver::Default,
            startup: CrystalStartup::Cycles16384,
            bypass: true,
            freq,
        }
    }
}

/// XTL 配置。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct XtlConfig {
    /// 振荡幅度档位（DRIVER[3:2]）。
    pub amplitude: XtlAmplitude,
    /// 驱动能力（DRIVER[1:0]）。
    pub driver: XtlDriver,
    /// 稳定时间档位（STARTUP）。
    pub startup: CrystalStartup,
    /// true = P14 引脚外部时钟输入（SYSCTRL1.EXTL_EN），false = 晶振。
    pub bypass: bool,
}

impl XtlConfig {
    /// 32.768 kHz 晶振。
    ///
    /// DRIVER 编码取 DS 7.3.7 推荐值 0b1010（amp=10、driver=10）。RM
    /// 3.5.7 字段级推荐为 amp=10、driver=01，与 DS 整体推荐不一致，差异
    /// 见 `docs/sdk-doc-discrepancies.md`，默认取 DS 电气特性表推荐值。
    pub fn crystal() -> Self {
        Self {
            amplitude: XtlAmplitude::Large,
            driver: XtlDriver::Stronger,
            startup: CrystalStartup::Cycles4096,
            bypass: false,
        }
    }

    /// 外部输入模式：P14 输入 32.768 kHz（RM 3.1.3），P15 需配为模拟态。
    pub fn bypass() -> Self {
        Self {
            amplitude: XtlAmplitude::Large,
            driver: XtlDriver::Stronger,
            startup: CrystalStartup::Cycles4096,
            bypass: true,
        }
    }
}

impl Default for XtlConfig {
    fn default() -> Self {
        Self::crystal()
    }
}

// ---------------------------------------------------------------------------
// SysTick / RTC 补偿 / 门控
// ---------------------------------------------------------------------------

/// SysTick 外部参考时钟选择（SYSTICK_CR.CLK_SEL，RM 3.5.9 位 27:26）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SysTickClock {
    /// XTL（编码 00）。
    Xtl,
    /// RCL（编码 01）。
    Rcl,
    /// SystemClk/8（编码 10）。SDK 枚举将其称为 RCH，以 RM 描述为准。
    SysclkDiv8,
    /// XTH（编码 11）。
    Xth,
}

impl SysTickClock {
    const fn encode(self) -> u8 {
        match self {
            SysTickClock::Xtl => 0,
            SysTickClock::Rcl => 1,
            SysTickClock::SysclkDiv8 => 2,
            SysTickClock::Xth => 3,
        }
    }
}

/// RTC 高速补偿时钟选择（SYSCTRL1.RTC_FREQ_ADJUST，RM 3.5.2 位 11:9）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RtcAdjustFreq {
    /// 4 MHz（编码 000）。
    Mhz4,
    /// 6 MHz（编码 001）。
    Mhz6,
    /// 8 MHz（编码 010）。
    Mhz8,
    /// 12 MHz（编码 011）。
    Mhz12,
    /// 16 MHz（编码 100）。
    Mhz16,
    /// 20 MHz（编码 101）。
    Mhz20,
    /// 24 MHz（编码 110）。
    Mhz24,
    /// 32 MHz（编码 111）。
    Mhz32,
}

impl RtcAdjustFreq {
    const fn encode(self) -> u8 {
        match self {
            RtcAdjustFreq::Mhz4 => 0,
            RtcAdjustFreq::Mhz6 => 1,
            RtcAdjustFreq::Mhz8 => 2,
            RtcAdjustFreq::Mhz12 => 3,
            RtcAdjustFreq::Mhz16 => 4,
            RtcAdjustFreq::Mhz20 => 5,
            RtcAdjustFreq::Mhz24 => 6,
            RtcAdjustFreq::Mhz32 => 7,
        }
    }
}

/// 外设门控位（PERI_CLKEN，RM 3.5.8）。
///
/// 该寄存器不在 SYSCTRL0/1 解锁范围内，读-改-写直接操作。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PeripheralGate {
    /// UART0（位 0）。
    Uart0,
    /// UART1（位 1）。
    Uart1,
    /// LPUART（位 2）。
    Lpuart,
    /// I2C（位 4）。
    I2c,
    /// SPI（位 6）。
    Spi,
    /// Timer0/1/2（位 8）。
    BaseTim,
    /// LPTimer（位 9）。
    LpTim,
    /// Timer4/5/6（位 10）。
    AdvTim,
    /// PCA（位 14）。
    Pca,
    /// WDT（位 15）。
    Wdt,
    /// ADC（位 16）。
    Adc,
    /// VC/LVD（位 17）。
    Vc,
    /// RTC（位 20）。
    Rtc,
    /// CLKTRIM（位 21）。
    Trim,
    /// SysTick 参考时钟（位 24）。
    Tick,
    /// CRC（位 26）。
    Crc,
    /// GPIO（位 28）。
    Gpio,
    /// Flash 控制器（位 31）。
    Flash,
}

/// 开启 GPIO 外设时钟门控，幂等。
///
/// GPIO 初始化路径（[`crate::init`] 与各 GPIO/复用构造入口）统一经由
/// 当前型号的系统控制器打开门控。读取-修改-写回非原子：若中断处理器
/// 也会开关 GPIO 门控，需调用方自行用临界区保护。驱动构造路径只在启动
/// 阶段执行，通常无需额外保护。
pub(crate) fn enable_gpio() {
    enable_peripheral(PeripheralGate::Gpio, true);
}

/// 设置外设时钟门控。
///
/// 读取-修改-写回非原子；中断处理器同时操作门控时需调用方加临界区。
pub fn enable_peripheral(gate: PeripheralGate, enable: bool) {
    backend::modify_peri_clken(|w| match gate {
        PeripheralGate::Uart0 => w.set_uart0(enable),
        PeripheralGate::Uart1 => w.set_uart1(enable),
        PeripheralGate::Lpuart => w.set_lpuart(enable),
        PeripheralGate::I2c => w.set_i2c(enable),
        PeripheralGate::Spi => w.set_spi(enable),
        PeripheralGate::BaseTim => w.set_basetim(enable),
        PeripheralGate::LpTim => w.set_lptim(enable),
        PeripheralGate::AdvTim => w.set_advtim(enable),
        PeripheralGate::Pca => w.set_pca(enable),
        PeripheralGate::Wdt => w.set_wdt(enable),
        PeripheralGate::Adc => w.set_adc(enable),
        PeripheralGate::Vc => w.set_vc(enable),
        PeripheralGate::Rtc => w.set_rtc(enable),
        PeripheralGate::Trim => w.set_trim(enable),
        PeripheralGate::Tick => w.set_tick(enable),
        PeripheralGate::Crc => w.set_crc(enable),
        PeripheralGate::Gpio => w.set_gpio(enable),
        PeripheralGate::Flash => w.set_flash(enable),
    });
}

/// 查询外设时钟门控状态。
pub fn is_peripheral_enabled(gate: PeripheralGate) -> bool {
    let r = backend::read_peri_clken();
    match gate {
        PeripheralGate::Uart0 => r.uart0(),
        PeripheralGate::Uart1 => r.uart1(),
        PeripheralGate::Lpuart => r.lpuart(),
        PeripheralGate::I2c => r.i2c(),
        PeripheralGate::Spi => r.spi(),
        PeripheralGate::BaseTim => r.basetim(),
        PeripheralGate::LpTim => r.lptim(),
        PeripheralGate::AdvTim => r.advtim(),
        PeripheralGate::Pca => r.pca(),
        PeripheralGate::Wdt => r.wdt(),
        PeripheralGate::Adc => r.adc(),
        PeripheralGate::Vc => r.vc(),
        PeripheralGate::Rtc => r.rtc(),
        PeripheralGate::Trim => r.trim(),
        PeripheralGate::Tick => r.tick(),
        PeripheralGate::Crc => r.crc(),
        PeripheralGate::Gpio => r.gpio(),
        PeripheralGate::Flash => r.flash(),
    }
}

// ---------------------------------------------------------------------------
// 频率快照
// ---------------------------------------------------------------------------

/// 初始化后的时钟树频率快照。
///
/// 由 [`init`] 写入，其他驱动经 [`clocks`] 读取（例如 UART 的波特率
/// 计算取 `pclk`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clocks {
    /// 系统时钟（切换后的源频率）。
    pub sys: Hertz,
    /// AHB 总线时钟：`sys / hclk_pre`。
    pub hclk: Hertz,
    /// APB 总线时钟：`hclk / pclk_pre`。
    pub pclk: Hertz,
}

impl Default for Clocks {
    fn default() -> Self {
        // 复位态：RCH 4 MHz，HCLK = PCLK = 4 MHz。
        Self {
            sys: Hertz::mhz(4),
            hclk: Hertz::mhz(4),
            pclk: Hertz::mhz(4),
        }
    }
}

static SYS_HZ: AtomicU32 = AtomicU32::new(4_000_000);
static HCLK_HZ: AtomicU32 = AtomicU32::new(4_000_000);
static PCLK_HZ: AtomicU32 = AtomicU32::new(4_000_000);

/// 读取当前时钟频率快照。
///
/// 未经 [`init`] 配置时返回复位默认值（RCH 4 MHz、不分频）。
pub fn clocks() -> Clocks {
    Clocks {
        sys: Hertz(SYS_HZ.load(Ordering::Relaxed)),
        hclk: Hertz(HCLK_HZ.load(Ordering::Relaxed)),
        pclk: Hertz(PCLK_HZ.load(Ordering::Relaxed)),
    }
}

fn store_clocks(c: Clocks) {
    SYS_HZ.store(c.sys.0, Ordering::Relaxed);
    HCLK_HZ.store(c.hclk.0, Ordering::Relaxed);
    PCLK_HZ.store(c.pclk.0, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// 配置与初始化
// ---------------------------------------------------------------------------

/// 时钟配置。
#[derive(Debug, Copy, Clone)]
pub struct Config {
    /// RCH 档位。`None` = 保持当前档位不重配。
    pub rch: Option<RchFreq>,
    /// XTH 配置。`None` = 不启用 XTH。
    pub xth: Option<XthConfig>,
    /// RCL 档位。`None` = 不启用 RCL。
    pub rcl: Option<RclFreq>,
    /// XTL 配置。`None` = 不启用 XTL。
    pub xtl: Option<XtlConfig>,
    /// 系统时钟源。
    pub sys: Sysclk,
    /// HCLK 分频。
    pub hclk_pre: HclkPrescaler,
    /// PCLK 分频。
    pub pclk_pre: PclkPrescaler,
    /// 时钟故障检测（SYSCTRL1.CLOCK_FT_EN）。SDK clk.h 定义为时钟故障
    /// 检测使能；RM 3.5.2 位 4 标记 Reserved，差异见差异台账。
    pub clock_fault_detect: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rch: Some(RchFreq::Mhz4),
            xth: None,
            rcl: None,
            xtl: None,
            sys: Sysclk::Rch,
            hclk_pre: HclkPrescaler::Div1,
            pclk_pre: PclkPrescaler::Div1,
            clock_fault_detect: false,
        }
    }
}

// ---------------------------------------------------------------------------
// 内部工具：晶振引脚、换频步进
// ---------------------------------------------------------------------------

/// 读当前系统时钟源。
fn current_source() -> Sysclk {
    Sysclk::decode(backend::read_sysctrl0().clk_sw4_sel()).unwrap_or(Sysclk::Rch)
}

/// 当前 HCLK 频率，用于软件延时估算。
///
/// XTH 为当前源时晶振频率未知，按 32 MHz 保守估计（延时只会长不会短）。
fn current_hclk_hz() -> u32 {
    let sys = match current_source() {
        Sysclk::Rch => {
            RchFreq::from_trim(backend::read_rch_cr().trim())
                .frequency()
                .0
        }
        Sysclk::Rcl => {
            let trim = backend::read_rcl_cr().trim();
            if trim & (1 << 9) != 0 {
                RclFreq::K38_4.frequency().0
            } else {
                RclFreq::K32_768.frequency().0
            }
        }
        Sysclk::Xth => 32_000_000,
        Sysclk::Xtl => 32_768,
    };
    let prs = backend::read_sysctrl0().hclk_prs();
    sys >> prs
}

/// 软件延时不少于 10 ms（RM 3.2.3 Step7：XTH 稳定后需延时 10ms 以上）。
///
/// 以当前 HCLK 估算迭代次数；M0+ 上该循环单次迭代约 3 周期起，按 3
/// 周期取下限保证时长不少于 10 ms。SDK 无此延时，差异见差异台账。
fn delay_10ms() {
    let n = (current_hclk_hz() / 300).max(1_000);
    let mut i = 0u32;
    while i < n {
        i = i.wrapping_add(1);
        core::hint::black_box(i);
    }
}

/// 临时开启 GPIO 门控执行晶振引脚配置，结束后恢复原门控值（SDK
/// `Clk_Enable` 同做法：保存 PERI_CLKEN → 置 GPIO 位 → 配引脚 → 恢复）。
fn with_gpio_gate(f: impl FnOnce()) {
    let saved = backend::read_peri_clken();
    if !saved.gpio() {
        enable_gpio();
        f();
        backend::write_peri_clken(saved);
    } else {
        f();
    }
}

/// 直接写 RCH 出厂校准值（RCH_CR 不在解锁范围，RM 3.5.4）。
fn write_rch_trim(freq: RchFreq) {
    backend::modify_rch_cr(|w| w.set_trim(freq.trim_value()));
}

/// RM 3.2.7 方案 1：RCH 为当前系统时钟时逐级换频。
///
/// 先把 HCLK 分频提到 /128，按 4M ↔ 8M ↔ 16M ↔ 24M/22.12M 逐级写 TRIM，
/// 分频在 [`init`] 末尾统一写回配置值。
fn rch_step_to(target: RchFreq) {
    backend::write_sysctrl0(|w| w.set_hclk_prs(HclkPrescaler::Div128 as u8));
    let mut level = RchFreq::from_trim(backend::read_rch_cr().trim()).step_level();
    let target_level = target.step_level();
    while level != target_level {
        level = if target_level > level {
            level + 1
        } else {
            level - 1
        };
        // 中间档写对应校准值；顶档直接写目标值（22.12M 或 24M）。
        let step = match level {
            0 => RchFreq::Mhz4,
            1 => RchFreq::Mhz8,
            2 => RchFreq::Mhz16,
            _ => target,
        };
        write_rch_trim(step);
    }
    write_rch_trim(target);
}

// ---------------------------------------------------------------------------
// 初始化主流程（RM 3.2.1）
// ---------------------------------------------------------------------------

/// 按配置初始化时钟树。
///
/// 执行 RM 3.2.1 标准切换流程并写入频率快照（[`clocks`]）。可重复调
/// 用（运行中重新配置），旧时钟源为 XTH 时其频率未知，FLASH 等待周期
/// 按保守的 32 MHz 处理。
///
/// # Panics
///
/// `sys` 指定的源未在配置中启用（例如 `sys = Xth` 而 `xth = None`），
/// 或该源等待稳定超时。
pub fn init(config: &Config) {
    let old = current_source();

    // ---- RCH 频率（RM 3.2.6/3.2.7）----
    if let Some(target) = config.rch {
        let current = RchFreq::from_trim(backend::read_rch_cr().trim());
        if current != target {
            if old == Sysclk::Rch {
                // RCH 正在驱动系统时钟：按 RM 3.2.7 方案 1 逐级换频。
                rch_step_to(target);
            } else {
                // RCH 非当前源：直接改写（RM 3.2.6 Step1）。
                write_rch_trim(target);
            }
        }
    }

    // ---- XTH（RM 3.2.3）----
    if let Some(xth) = config.xth {
        with_gpio_gate(|| crate::gpio::backend_setup_xth_pins(xth.bypass));
        // XTH_CR 不在解锁范围（RM 3.2.3 Step2 先于 Step4 解锁）。
        backend::modify_xth_cr(|w| {
            w.set_driver((xth.freq_range.encode() << 2) | xth.driver.encode());
            w.set_startup(xth.startup.encode());
        });
        if xth.bypass {
            backend::write_sysctrl1(|w| w.set_exth_en(true));
        }
        // 使能并等稳定（Step4~Step7）。
        backend::write_sysctrl0(|w| w.set_xth_en(true));
        let deadline = wait_stable(WaitSource::Xth);
        if !deadline {
            panic!("clock: XTH 等待稳定超时");
        }
        // RM 3.2.3 Step7：稳定后软件延时 10ms 以上再切换。
        delay_10ms();
    }

    // ---- RCL（RM 3.2.5 Step1~Step4）----
    if let Some(rcl) = config.rcl {
        backend::modify_rcl_cr(|w| {
            w.set_trim(rcl.trim_value());
            w.set_startup(RclStartup::Cycles256.encode());
        });
        backend::write_sysctrl0(|w| w.set_rcl_en(true));
        if !wait_stable(WaitSource::Rcl) {
            panic!("clock: RCL 等待稳定超时");
        }
    }

    // ---- XTL（RM 3.2.2）----
    if let Some(xtl) = config.xtl {
        with_gpio_gate(|| crate::gpio::backend_setup_xtl_pins(xtl.bypass));
        backend::modify_xtl_cr(|w| {
            w.set_driver((xtl.amplitude.encode() << 2) | xtl.driver.encode());
            w.set_startup(xtl.startup.encode());
        });
        if xtl.bypass {
            backend::write_sysctrl1(|w| w.set_extl_en(true));
        }
        backend::write_sysctrl0(|w| w.set_xtl_en(true));
        if !wait_stable(WaitSource::Xtl) {
            panic!("clock: XTL 等待稳定超时");
        }
    }

    // ---- 目标源频率 ----
    // sys = Xtl 必须提供 XTL 配置（保证振荡器已按 RM 3.2.2 使能并稳定），
    // 与 sys = Xth/Rcl 的守卫一致。
    if config.sys == Sysclk::Xtl && config.xtl.is_none() {
        panic!("clock: sys = Xtl 需要在 config.xtl 提供配置");
    }
    let sys_freq = match config.sys {
        Sysclk::Rch => match config.rch {
            Some(f) => f.frequency(),
            None => RchFreq::from_trim(backend::read_rch_cr().trim()).frequency(),
        },
        Sysclk::Xth => {
            config
                .xth
                .as_ref()
                .expect("clock: sys = Xth 需要在 config.xth 提供配置")
                .freq
        }
        Sysclk::Rcl => config
            .rcl
            .expect("clock: sys = Rcl 需要在 config.rcl 提供档位")
            .frequency(),
        Sysclk::Xtl => Hertz(32_768),
    };

    // ---- RCH 重新使能/稳定化（RM 3.2.6 Step3/Step4）----
    // 再入场景：目标源为 RCH 且当前系统时钟不在 RCH 上。无条件"先清后置
    // RCH_EN"重启振荡启动序列再等待 Stable：本次 init 可能刚直写过 TRIM
    // （RM 3.1.1"特定的更改时序"），TRIM 更改后 STABLE 是否重新清零手册
    // 未说明，重启使等待必然有效；RCH 未驱动系统时钟，清置窗口无手册禁
    // 忌，启动仅 4us（RM 3.1.1）代价可忽略。
    if config.sys == Sysclk::Rch && old != Sysclk::Rch {
        backend::write_sysctrl0(|w| w.set_rch_en(false));
        backend::write_sysctrl0(|w| w.set_rch_en(true));
        // RM 3.2.6 Step4："查询等待 RCH_CR.Stable 标志变为 1"。
        while !backend::read_rch_cr().stable() {}
    }

    // ---- FLASH 等待周期，前置（RM 3.2.1 Step4，按新旧源较高频率）----
    // 旧源为 XTH 时晶振频率未知，按 32 MHz 保守处理。
    let old_may_exceed_24m = match old {
        Sysclk::Xth => true,
        Sysclk::Rch | Sysclk::Rcl | Sysclk::Xtl => false,
    };
    if old_may_exceed_24m || sys_freq.0 > 24_000_000 {
        backend::set_flash_wait(true);
    }

    // ---- 切换系统时钟源（RM 3.2.1 Step6）----
    if config.sys != old {
        backend::write_sysctrl0(|w| w.set_clk_sw4_sel(config.sys.encode()));
    }

    // ---- 分频（RM 3.5.1；RM 3.2.7 方案 1 的分频在此一并写回）----
    backend::write_sysctrl0(|w| {
        w.set_hclk_prs(config.hclk_pre as u8);
        w.set_pclk_prs(config.pclk_pre as u8);
    });

    // ---- FLASH 等待周期，按新源频率复配（RM 3.2.1 Step7）----
    backend::set_flash_wait(sys_freq.0 > 24_000_000);

    // ---- 时钟故障检测（SYSCTRL1 位 4，SDK 定义 CLOCK_FT_EN）----
    // RM 3.5.2 位 4 标记 Reserved（差异台账已登记）；仅在用户显式开启
    // 时写 1，默认不触碰保留位。
    if config.clock_fault_detect {
        backend::write_sysctrl1(|w| w.set_clock_ft_en(true));
    }

    // ---- 关闭不再使用的旧源（RM 3.2.1 Step8）----
    // 仅处理本函数知情的能力：复位后仅 RCH 使能；XTH/RCL/XTL 若为旧源
    // 且未在配置中出现，说明是本函数之外的使能，不代为关闭。
    if old == Sysclk::Rch && config.rch.is_none() && config.sys != Sysclk::Rch {
        backend::write_sysctrl0(|w| w.set_rch_en(false));
    }

    // ---- 频率快照 ----
    let hclk = sys_freq / config.hclk_pre;
    let pclk = hclk / config.pclk_pre;
    store_clocks(Clocks {
        sys: sys_freq,
        hclk,
        pclk,
    });
}

/// 振荡稳定等待（`*_CR.STABLE`，RM 3.5.4~3.5.7）。
///
/// 带简单超时上限（STABLE 长期为 0 时避免死循环），硬件正常时远在超时
/// 前返回。手册未规定轮询超时值，上限按最长稳定时间的量级放宽取值。
fn wait_stable(source: WaitSource) -> bool {
    // 上限迭代数：按最慢时钟 XTL 2000ms 启动（DS 7.3.7）量级放宽。
    let deadline = 20_000_000u32;
    for _ in 0..deadline {
        let stable = match source {
            WaitSource::Xth => backend::read_xth_cr().stable(),
            WaitSource::Rcl => backend::read_rcl_cr().stable(),
            WaitSource::Xtl => backend::read_xtl_cr().stable(),
        };
        if stable {
            return true;
        }
    }
    false
}

enum WaitSource {
    Xth,
    Rcl,
    Xtl,
}

// ---------------------------------------------------------------------------
// 运行期单项开关
// ---------------------------------------------------------------------------

/// 深度休眠唤醒后是否强制使用 RCH（SYSCTRL0.WAKEUP_BYRCH，RM 3.5.1 位 15）。
pub fn set_wakeup_by_rch(enable: bool) {
    backend::write_sysctrl0(|w| w.set_wakeup_byrch(enable));
}

/// XTL 高级使能（SYSCTRL1.XTL_ALWAYS_ON，RM 3.5.2 位 3）。
///
/// 置 1 后 SYSCTRL0.XTL_EN 只可置位不可清零。
pub fn set_xtl_always_on(enable: bool) {
    backend::write_sysctrl1(|w| w.set_xtl_alwayson(enable));
}

/// RTC 低功耗模式（SYSCTRL1.RTC_LPW，RM 3.5.2 位 5）。
///
/// 使能后 RTC 寄存器不可读写。
pub fn set_rtc_lpw(enable: bool) {
    backend::write_sysctrl1(|w| w.set_rtc_lpw(enable));
}

/// RESET 引脚改作 GPIO（SYSCTRL1.RES_UIO，RM 3.5.2 位 7）。
pub fn set_res_pin_gpio(enable: bool) {
    backend::write_sysctrl1(|w| w.set_res_uio(enable));
}

/// SWD 引脚改作 GPIO（SYSCTRL1.SWD_UIO，RM 3.5.2 位 8）。
pub fn set_swd_pin_gpio(enable: bool) {
    backend::write_sysctrl1(|w| w.set_swd_uio(enable));
}

/// RTC 高速补偿时钟选择（SYSCTRL1.RTC_FREQ_ADJUST，RM 3.5.2 位 11:9）。
pub fn set_rtc_adjust_clock(freq: RtcAdjustFreq) {
    backend::write_sysctrl1(|w| w.set_rtc_freq_adjust(freq.encode()));
}

// ---------------------------------------------------------------------------
// SysTick 时钟
// ---------------------------------------------------------------------------

/// SysTick 参考时钟选择 XTL/RCL/SystemClk/8/XTH（SYSTICK_CR.CLK_SEL，
/// NOREF = 0，RM 3.5.9）。
///
/// 同时开启 PERI_CLKEN.TICK 门控。外部参考时钟频率不允许高于 HCLK
/// （RM 3.5.9 NOREF 字段注）。
pub fn set_systick_clock(sel: SysTickClock) {
    enable_peripheral(PeripheralGate::Tick, true);
    backend::modify_systick_cr(|w| {
        w.set_noref(false);
        w.set_clk_sel(sel.encode());
    });
}

/// SysTick 改用 HCLK（SYSTICK_CR.NOREF = 1，RM 3.5.9 位 25）。
pub fn set_systick_hclk() {
    enable_peripheral(PeripheralGate::Tick, true);
    backend::modify_systick_cr(|w| w.set_noref(true));
}

/// 读取 STCALIB：参考时钟为 XTL 时的 10 ms 校准值（RM 3.5.9 位 23:0）。
pub fn systick_calib() -> u32 {
    backend::read_systick_cr().stcalib()
}

// ---------------------------------------------------------------------------
// HCLK/PCLK 波形输出（时钟分频例程的观测点）
// ---------------------------------------------------------------------------

/// 输出分频（GPIO CTRL1.HCLK_SEL/PCLK_SEL，RM 6.5.5.1 位 11:10/9:8）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum OutputDiv {
    /// 不分频（编码 00）。
    Div1,
    /// /2（编码 01）。
    Div2,
    /// /4（编码 10）。
    Div4,
    /// /8（编码 11）。
    Div8,
}

impl OutputDiv {
    const fn encode(self) -> u8 {
        match self {
            OutputDiv::Div1 => 0,
            OutputDiv::Div2 => 1,
            OutputDiv::Div4 => 2,
            OutputDiv::Div8 => 3,
        }
    }
}

/// P24 输出 HCLK 波形（P24_SEL=3，GPIO CTRL1.HCLK_EN，RM 6.5.5/SDK
/// `Gpio_SetFunc_HCLKOUT_P24`）。
///
/// 引脚与 CTRL1 的寄存器操作经 GPIO 后端执行，门控由后端打开。
pub fn enable_hclk_output_p24(div: OutputDiv) {
    crate::gpio::backend_enable_hclk_p24(div.encode());
}

/// 停止 P24 的 HCLK 输出（CTRL1.HCLK_EN = 0）。
pub fn disable_hclk_output_p24() {
    crate::gpio::backend_disable_hclk_p24();
}

/// P31 输出 PCLK 波形（P31_SEL=3，GPIO CTRL1.PCLK_EN，RM 6.5.5/SDK
/// `Gpio_SetFunc_PCLKOUT_P31`，SDK 同时清 HCLK_EN——SDK 宏即如此，RM
/// 未要求，无害保守）。
///
/// 非加密芯片上电后 P31 默认为 SWCLK 调试引脚（RM 25.3.2），本函数同时
/// 置 SYSCTRL1.SWD_USE_IO 释放该引脚。
pub fn enable_pclk_output_p31(div: OutputDiv) {
    enable_gpio();
    set_swd_pin_gpio(true);
    crate::gpio::backend_enable_pclk_p31(div.encode());
}

/// 停止 P31 的 PCLK 输出（CTRL1.PCLK_EN = 0）。
pub fn disable_pclk_output_p31() {
    crate::gpio::backend_disable_pclk_p31();
}
