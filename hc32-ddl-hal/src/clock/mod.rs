//! 系统时钟。
//!
//! 模块入口保留应用可见的配置与查询 API：[`Config`] 与 [`init`]、门控
//! [`enable_peripheral`]、频率快照 [`clocks`]、SysTick 时钟与 HCLK/PCLK
//! 波形输出。寄存器操作按 IP 后端拆分：HC32L110 的 `CLOCK` 外设为
//! SYSCTRL `ddl_v2_2`（[`sysctrl_v2_2`]），HC32L13x 的 `SYSCTRL` 外设为
//! `ddl_v3_1`（[`sysctrl_v3_1`]）。接入第二种布局时沿用本入口的公开
//! 类型，仅替换寄存器访问（AGENTS.md「HAL 源码布局与命名」）。
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
//! HC32L13x（`sysctrl_ddl_v3_1`）支持 RCH/XTH/RCL/XTL/PLL 五源切换、
//! HCLK/PCLK 分频和外设门控。PLL 输入 4~24 MHz、输出 8~48 MHz，倍频
//! 2~12；启动前使能 BGR 并等待 20us，FLASH 等待周期随新旧源中较高
//! 频率配置。PLL 只能与 RCH/XTH 直接切换，其他组合由 [`init`] 自动
//! 经参考源或 RCH 中转。顶层 `Config::default()` 仍走复位默认 RCH
//! 4 MHz 与出厂 TRIM 装载路径。
//!
//! HC32F420（`sysctrl_ddl_v4`）支持 RCHCLKD、XTH、RCL、PLL 四个系统
//! 时钟源，以及 HCLK、PCLK0、PCLK1 分频。[`init`] 按 RM 3.2 使用临时
//! HCLK 分频完成切换，并同步设置 FLASH_WAIT；[`clocks`] 与 [`pclk1`]
//! 返回配置后的频率。复位默认仍为 RCHCLKD 4 MHz。
//!
//! [`sysctrl_v2_2`]: self::sysctrl_v2_2
//! [`sysctrl_v3_1`]: self::sysctrl_v3_1
//! [`sysctrl_v4`]: self::sysctrl_v4

#[cfg(sysctrl_ddl_v2_2)]
pub(crate) mod sysctrl_v2_2;
#[cfg(sysctrl_ddl_v2_2)]
use sysctrl_v2_2 as backend;
#[cfg(sysctrl_ddl_v2_2)]
mod l110;
#[cfg(sysctrl_ddl_v2_2)]
use l110::DEFAULT_RCH_HZ as BACKEND_SYS_HZ;

#[cfg(sysctrl_ddl_v3_1)]
pub(crate) mod sysctrl_v3_1;
#[cfg(sysctrl_ddl_v3_1)]
use sysctrl_v3_1 as backend;
#[cfg(sysctrl_ddl_v3_1)]
use sysctrl_v3_1::DEFAULT_RCH_HZ as BACKEND_SYS_HZ;
#[cfg(sysctrl_ddl_v3_1)]
mod l13x;

#[cfg(sysctrl_ddl_v4)]
pub(crate) mod sysctrl_v4;
#[cfg(sysctrl_ddl_v4)]
use sysctrl_v4 as backend;
#[cfg(sysctrl_ddl_v4)]
use sysctrl_v4::DEFAULT_SYSCLK_HZ as BACKEND_SYS_HZ;
#[cfg(sysctrl_ddl_v4)]
mod f420;
#[cfg(sysctrl_ddl_v4)]
pub use f420::*;

use core::sync::atomic::{AtomicU32, Ordering};

use crate::time::Hertz;

// ---------------------------------------------------------------------------
// 时钟源与档位枚举
// ---------------------------------------------------------------------------

/// 系统时钟源（SYSCTRL0.CLKSW，RM 3.5.1）。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
    /// 锁相环输出（编码 4，仅 HC32L13x）。
    #[cfg(sysctrl_ddl_v3_1)]
    Pll,
}

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
impl Sysclk {
    /// 寄存器编码。
    pub const fn encode(self) -> u8 {
        match self {
            Sysclk::Rch => 0,
            Sysclk::Xth => 1,
            Sysclk::Rcl => 2,
            Sysclk::Xtl => 3,
            #[cfg(sysctrl_ddl_v3_1)]
            Sysclk::Pll => 4,
        }
    }

    /// 从寄存器编码还原，非法值返回 `None`。
    pub const fn decode(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Sysclk::Rch),
            1 => Some(Sysclk::Xth),
            2 => Some(Sysclk::Rcl),
            3 => Some(Sysclk::Xtl),
            #[cfg(sysctrl_ddl_v3_1)]
            4 => Some(Sysclk::Pll),
            _ => None,
        }
    }
}

/// RCH 输出频率档位。
///
/// 每档对应 flash 出厂校准区（0x00100C00~0x00100C08，RM 3.5.4 TRIM 字段
/// 描述）的一组 TRIM 值；写入 RCH_CR.TRIM 即得到该频率。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

    /// 从 RCH_CR.TRIM 原始值推断档位；非出厂校准值按 SDK 规则归入
    /// 最接近的默认档，仅供保守的换频与延时估算。
    #[cfg(sysctrl_ddl_v2_2)]
    pub fn from_trim(trim: u16) -> Self {
        match (trim >> 9) & 0x3 {
            0 => RchFreq::Mhz4,
            1 => RchFreq::Mhz8,
            2 => RchFreq::Mhz16,
            _ if trim == RchFreq::Mhz22_12.trim_value() => RchFreq::Mhz22_12,
            _ => RchFreq::Mhz24,
        }
    }

    /// L13x SDK 按完整出厂校准值识别档位，未知值归入复位默认 4 MHz。
    #[cfg(sysctrl_ddl_v3_1)]
    pub fn from_trim(trim: u16) -> Self {
        match trim {
            _ if trim == RchFreq::Mhz24.trim_value() => RchFreq::Mhz24,
            _ if trim == RchFreq::Mhz22_12.trim_value() => RchFreq::Mhz22_12,
            _ if trim == RchFreq::Mhz16.trim_value() => RchFreq::Mhz16,
            _ if trim == RchFreq::Mhz8.trim_value() => RchFreq::Mhz8,
            _ => RchFreq::Mhz4,
        }
    }

    /// RCH 换频的逐级档位：4/8/16 为独立级，22.12M 与 24M 同为顶档。
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RclFreq {
    /// 38.4 kHz（校准值 @0x00100C20）。
    K38_4,
    /// 32.768 kHz（校准值 @0x00100C22）。
    K32_768,
}

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

/// PLL 参考时钟源（PLL_CR.REFSEL，仅 HC32L13x）。
#[cfg(sysctrl_ddl_v3_1)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PllSource {
    /// 内部高速 RC。
    Rch,
    /// XTH 晶振或 PD00 外部输入，由 [`XthConfig::bypass`] 决定。
    Xth,
}

/// PLL 配置（仅 HC32L13x）。
///
/// 输出频率为 `输入频率 × multiplier`。初始化会检查输入 4~24 MHz、
/// 倍频系数 2~12、输出 8~48 MHz，并自动设置 FRSEL/FOSC 与最长稳定时间。
#[cfg(sysctrl_ddl_v3_1)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PllConfig {
    /// 参考时钟源。
    pub source: PllSource,
    /// 倍频系数，允许 2~12。
    pub multiplier: u8,
}

#[cfg(sysctrl_ddl_v3_1)]
impl PllConfig {
    /// 创建 PLL 配置；参数合法性在 [`init`] 时检查。
    pub const fn new(source: PllSource, multiplier: u8) -> Self {
        Self { source, multiplier }
    }
}

/// XTH 工作频率范围（XTH_CR.XTH_FSEL，RM 3.5.5）。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

/// XTL 振荡幅度档位（XTL_CR.AMP_SEL，RM 3.5.7）。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct XthConfig {
    /// 晶振工作频率范围（XTH_FSEL）。
    pub freq_range: XthFreqRange,
    /// 驱动能力（DRIVER[1:0]）。
    pub driver: XthDriver,
    /// 稳定时间档位（STARTUP）。
    pub startup: CrystalStartup,
    /// true = 外部时钟输入（L110 P01 / L13x PD00），false = 晶振。
    pub bypass: bool,
    /// 晶振频率或外部输入频率。
    pub freq: Hertz,
}

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

    /// 外部输入模式：L110 P01 / L13x PD00 输入 4~32 MHz（RM 3.1.4），
    /// 另一晶振脚需配为模拟态。
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct XtlConfig {
    /// 振荡幅度档位（AMP_SEL）。
    pub amplitude: XtlAmplitude,
    /// 驱动能力（DRIVER[1:0]）。
    pub driver: XtlDriver,
    /// 稳定时间档位（STARTUP）。
    pub startup: CrystalStartup,
    /// true = 外部时钟输入（L110 P14 / L13x PC14），false = 晶振。
    pub bypass: bool,
}

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
impl XtlConfig {
    /// 32.768 kHz 晶振。
    ///
    /// DRIVER 编码取 DS 7.3.7 推荐值 0b1010（amp=10、driver=10）。RM
    /// 3.5.7 字段级推荐为 amp=10、driver=01，与 DS 整体推荐不一致，默认
    /// 取 DS 电气特性表推荐值。L13x 的稳定时间另按 DS 注 4 取 16384。
    pub fn crystal() -> Self {
        Self {
            amplitude: XtlAmplitude::Large,
            driver: XtlDriver::Stronger,
            startup: default_xtl_startup(),
            bypass: false,
        }
    }

    /// 外部输入模式：L110 P14 / L13x PC14 输入 32.768 kHz，另一晶振脚
    /// 需配为模拟态。
    pub fn bypass() -> Self {
        Self {
            amplitude: XtlAmplitude::Large,
            driver: XtlDriver::Stronger,
            startup: default_xtl_startup(),
            bypass: true,
        }
    }
}

#[cfg(sysctrl_ddl_v2_2)]
const fn default_xtl_startup() -> CrystalStartup {
    CrystalStartup::Cycles4096
}

#[cfg(sysctrl_ddl_v3_1)]
const fn default_xtl_startup() -> CrystalStartup {
    CrystalStartup::Cycles16384
}

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
impl Default for XtlConfig {
    fn default() -> Self {
        Self::crystal()
    }
}

// ---------------------------------------------------------------------------
// SysTick / RTC 补偿 / 门控
// ---------------------------------------------------------------------------

/// SysTick 外部参考时钟选择（SYSTICK_CR.CLK_SEL，RM 3.5.9 位 27:26）。
#[cfg(sysctrl_ddl_v2_2)]
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

#[cfg(sysctrl_ddl_v2_2)]
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
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

/// 外设门控位（PERI_CLKEN，L110 RM 3.5.8 / L13x RM 3.5.9）。
///
/// 该寄存器不在 SYSCTRL0/1 解锁范围内，读-改-写直接操作。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PeripheralGate {
    Uart0,
    Uart1,
    #[cfg(sysctrl_ddl_v2_2)]
    Lpuart,
    #[cfg(sysctrl_ddl_v3_1)]
    Lpuart0,
    #[cfg(sysctrl_ddl_v3_1)]
    Lpuart1,
    #[cfg(sysctrl_ddl_v2_2)]
    I2c,
    #[cfg(sysctrl_ddl_v3_1)]
    I2c0,
    #[cfg(sysctrl_ddl_v3_1)]
    I2c1,
    #[cfg(sysctrl_ddl_v2_2)]
    Spi,
    #[cfg(sysctrl_ddl_v3_1)]
    Spi0,
    #[cfg(sysctrl_ddl_v3_1)]
    Spi1,
    /// Timer0/1/2（位 8）。
    BaseTim,
    LpTim,
    /// Timer4/5/6（位 10）。
    AdvTim,
    #[cfg(sysctrl_ddl_v3_1)]
    Tim3,
    #[cfg(sysctrl_ddl_v3_1)]
    Opa,
    Pca,
    Wdt,
    /// ADC；L13x 上同时门控 BGR（位 16）。
    Adc,
    /// VC/LVD（位 17）。
    Vc,
    #[cfg(sysctrl_ddl_v3_1)]
    Rng,
    #[cfg(sysctrl_ddl_v3_1)]
    Pcnt,
    Rtc,
    Trim,
    #[cfg(sysctrl_ddl_v3_1)]
    Lcd,
    /// SysTick 参考时钟（位 24）。
    Tick,
    #[cfg(sysctrl_ddl_v3_1)]
    Swd,
    Crc,
    #[cfg(sysctrl_ddl_v3_1)]
    Aes,
    Gpio,
    #[cfg(sysctrl_ddl_v3_1)]
    Dma,
    #[cfg(sysctrl_ddl_v3_1)]
    Div,
    Flash,
}

/// 开启 GPIO 外设时钟门控，幂等（单外设多端口布局）。
///
/// 读取-修改-写回非原子：若中断处理器也会开关 GPIO 门控，需调用方自行
/// 用临界区保护。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
pub(crate) fn enable_gpio() {
    enable_peripheral(PeripheralGate::Gpio, true);
}

/// 开启 `port` 端口 GPIO 实例的时钟门控，幂等（按端口独立门控布局）。
#[cfg(sysctrl_ddl_v4)]
pub(crate) fn enable_gpio(port: u8) {
    backend::enable_gpio(port);
}

/// 设置外设时钟门控。
///
/// 读取-修改-写回非原子；中断处理器同时操作门控时需调用方加临界区。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
pub fn enable_peripheral(gate: PeripheralGate, enable: bool) {
    backend::modify_peri_clken(|w| match gate {
        PeripheralGate::Uart0 => w.set_uart0(enable),
        PeripheralGate::Uart1 => w.set_uart1(enable),
        #[cfg(sysctrl_ddl_v2_2)]
        PeripheralGate::Lpuart => w.set_lpuart(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Lpuart0 => w.set_lpuart0(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Lpuart1 => w.set_lpuart1(enable),
        #[cfg(sysctrl_ddl_v2_2)]
        PeripheralGate::I2c => w.set_i2c(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::I2c0 => w.set_i2c0(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::I2c1 => w.set_i2c1(enable),
        #[cfg(sysctrl_ddl_v2_2)]
        PeripheralGate::Spi => w.set_spi(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Spi0 => w.set_spi0(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Spi1 => w.set_spi1(enable),
        PeripheralGate::BaseTim => w.set_basetim(enable),
        PeripheralGate::LpTim => w.set_lptim(enable),
        PeripheralGate::AdvTim => w.set_advtim(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Tim3 => w.set_tim3(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Opa => w.set_opa(enable),
        PeripheralGate::Pca => w.set_pca(enable),
        PeripheralGate::Wdt => w.set_wdt(enable),
        PeripheralGate::Adc => w.set_adc(enable),
        PeripheralGate::Vc => w.set_vc(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Rng => w.set_rng(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Pcnt => w.set_pcnt(enable),
        PeripheralGate::Rtc => w.set_rtc(enable),
        PeripheralGate::Trim => w.set_trim(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Lcd => w.set_lcd(enable),
        PeripheralGate::Tick => w.set_tick(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Swd => w.set_swd(enable),
        PeripheralGate::Crc => w.set_crc(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Aes => w.set_aes(enable),
        PeripheralGate::Gpio => w.set_gpio(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Dma => w.set_dma(enable),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Div => w.set_div(enable),
        PeripheralGate::Flash => w.set_flash(enable),
    });
}

/// 查询外设时钟门控状态。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
pub fn is_peripheral_enabled(gate: PeripheralGate) -> bool {
    let r = backend::read_peri_clken();
    match gate {
        PeripheralGate::Uart0 => r.uart0(),
        PeripheralGate::Uart1 => r.uart1(),
        #[cfg(sysctrl_ddl_v2_2)]
        PeripheralGate::Lpuart => r.lpuart(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Lpuart0 => r.lpuart0(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Lpuart1 => r.lpuart1(),
        #[cfg(sysctrl_ddl_v2_2)]
        PeripheralGate::I2c => r.i2c(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::I2c0 => r.i2c0(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::I2c1 => r.i2c1(),
        #[cfg(sysctrl_ddl_v2_2)]
        PeripheralGate::Spi => r.spi(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Spi0 => r.spi0(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Spi1 => r.spi1(),
        PeripheralGate::BaseTim => r.basetim(),
        PeripheralGate::LpTim => r.lptim(),
        PeripheralGate::AdvTim => r.advtim(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Tim3 => r.tim3(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Opa => r.opa(),
        PeripheralGate::Pca => r.pca(),
        PeripheralGate::Wdt => r.wdt(),
        PeripheralGate::Adc => r.adc(),
        PeripheralGate::Vc => r.vc(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Rng => r.rng(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Pcnt => r.pcnt(),
        PeripheralGate::Rtc => r.rtc(),
        PeripheralGate::Trim => r.trim(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Lcd => r.lcd(),
        PeripheralGate::Tick => r.tick(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Swd => r.swd(),
        PeripheralGate::Crc => r.crc(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Aes => r.aes(),
        PeripheralGate::Gpio => r.gpio(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Dma => r.dma(),
        #[cfg(sysctrl_ddl_v3_1)]
        PeripheralGate::Div => r.div(),
        PeripheralGate::Flash => r.flash(),
    }
}

// ---------------------------------------------------------------------------
// 频率快照
// ---------------------------------------------------------------------------

/// 初始化后的时钟树频率快照。
///
/// 由 [`init`] 写入，其他驱动经 [`clocks`] 读取（例如 UART 的波特率
/// 计算取 `pclk`）。HC32F420 有两根 APB 总线：`pclk` 记 PCLK0（APB0），
/// PCLK1（APB1）经 [`pclk1`] 单独查询。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clocks {
    /// 系统时钟（切换后的源频率）。
    pub sys: Hertz,
    /// AHB 总线时钟：`sys / hclk_pre`。
    pub hclk: Hertz,
    /// APB 总线时钟：`hclk / pclk_pre`（F420 为 PCLK0）。
    pub pclk: Hertz,
}

impl Default for Clocks {
    fn default() -> Self {
        // 复位态：系统时钟 4 MHz，HCLK = PCLK = 4 MHz。HC32L110/L13x
        // 为 RCH 4 MHz（各自 RM 3.5.1 SYSCTRL0 复位值 0x0000_0001：
        // 默认源 RCH、不分频）；HC32F420 为 RCHCLKD 4 MHz（RCH 24 MHz
        // 本振 6 分频：RCHCR 复位值 0x2、DIV 编码 0010 = /6，RM
        // 3.1.2/3.5.6，CR0 复位值 0x0000_0000 选 RCHCLKD 且各分频为 1）。
        Self {
            sys: Hertz::mhz(4),
            hclk: Hertz::mhz(4),
            pclk: Hertz::mhz(4),
        }
    }
}

static SYS_HZ: AtomicU32 = AtomicU32::new(BACKEND_SYS_HZ);
static HCLK_HZ: AtomicU32 = AtomicU32::new(BACKEND_SYS_HZ);
static PCLK_HZ: AtomicU32 = AtomicU32::new(BACKEND_SYS_HZ);
// F420 的第二根 APB 总线时钟，由时钟初始化与 store 路径一并更新。
#[cfg(sysctrl_ddl_v4)]
static PCLK1_HZ: AtomicU32 = AtomicU32::new(BACKEND_SYS_HZ);

/// 读取当前时钟频率快照。
///
/// HC32L110 未经 [`init`] 配置、HC32L13x/F420 未经 [`init_default`]
/// 装载时返回复位默认值（4 MHz、不分频）。F420 的 `pclk` 为 PCLK0
/// （APB0）；PCLK1（APB1）另有 [`pclk1`] 查询。
pub fn clocks() -> Clocks {
    Clocks {
        sys: Hertz(SYS_HZ.load(Ordering::Relaxed)),
        hclk: Hertz(HCLK_HZ.load(Ordering::Relaxed)),
        pclk: Hertz(PCLK_HZ.load(Ordering::Relaxed)),
    }
}

/// 读取 PCLK1（APB1）频率快照（仅 HC32F420：该布局有两根 APB 总线，
/// 复位默认 PCLK1 = PCLK0 = HCLK，RM 3.5.2 PCLK1PRS 复位值 0）。
#[cfg(sysctrl_ddl_v4)]
pub fn pclk1() -> Hertz {
    Hertz(PCLK1_HZ.load(Ordering::Relaxed))
}

// 各可配置时钟树在切换完成后写入；默认路径保持复位默认快照。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1, sysctrl_ddl_v4))]
fn store_clocks(c: Clocks) {
    SYS_HZ.store(c.sys.0, Ordering::Relaxed);
    HCLK_HZ.store(c.hclk.0, Ordering::Relaxed);
    PCLK_HZ.store(c.pclk.0, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// 配置与初始化
// ---------------------------------------------------------------------------

/// HC32L13x/HC32F420 默认时钟初始化（对齐厂商 `SystemInit` 默认路径）。
///
/// Rust 启动路径不执行厂商启动代码，本函数补齐其中的默认动作：
///
/// - HC32L13x：装载 RCH 出厂校准 TRIM（flash 0x00100C08，4 MHz 档；
///   RM 3.1.1/3.5.4）与 RCL 出厂校准 TRIM（flash 0x00100C22，
///   32.768 kHz 档，不开源）；
/// - HC32F420：装载 RCL 出厂校准 TRIM（flash 0x0100_11E8，32.768 kHz
///   档）与 RCH 出厂校准 TRIM（flash 0x0100_11E4，24 MHz 档）。
///
/// 装载后系统时钟保持复位默认：L13x 为 RCH 4 MHz（SYSCTRL0 复位值
/// 0x0000_0001，RM 3.5.1），F420 为 RCHCLKD 4 MHz（RCH 24 MHz 经
/// RCHCR.DIV = 2 分频，RM 3.1.2/3.5.6），HCLK = PCLK = 4 MHz，频率
/// 快照与 [`clocks`] 默认值一致。需要其他时钟树时改用 [`init`]。
/// F420 若已使用 22.12 MHz RCH 档，重复调用本函数会将 TRIM 恢复为
/// 24 MHz 出厂档；此时应重新调用 [`init`] 并更新频率快照。
///
/// [`clock::Config`]: Config
#[cfg(any(sysctrl_ddl_v3_1, sysctrl_ddl_v4))]
pub fn init_default() {
    backend::init_default();
}

/// 时钟配置。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
    /// PLL 配置（仅 L13x）。`None` = 不启用 PLL。
    #[cfg(sysctrl_ddl_v3_1)]
    pub pll: Option<PllConfig>,
    /// 系统时钟源。
    pub sys: Sysclk,
    /// HCLK 分频。
    pub hclk_pre: HclkPrescaler,
    /// PCLK 分频。
    pub pclk_pre: PclkPrescaler,
    /// 时钟故障检测（仅 L110；L13x 的对应位为保留位）。
    #[cfg(sysctrl_ddl_v2_2)]
    pub clock_fault_detect: bool,
}

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
impl Default for Config {
    fn default() -> Self {
        Self {
            rch: Some(RchFreq::Mhz4),
            xth: None,
            rcl: None,
            xtl: None,
            #[cfg(sysctrl_ddl_v3_1)]
            pll: None,
            sys: Sysclk::Rch,
            hclk_pre: HclkPrescaler::Div1,
            pclk_pre: PclkPrescaler::Div1,
            #[cfg(sysctrl_ddl_v2_2)]
            clock_fault_detect: false,
        }
    }
}

/// 按配置初始化 HC32L13x 时钟树，可重复调用。
///
/// PLL 只允许与 RCH/XTH 直接切换；其他组合由驱动自动经其参考源或
/// RCH 中转。切换前后按新旧源较高频率设置 FLASH 等待周期。
#[cfg(sysctrl_ddl_v3_1)]
pub fn init(config: &Config) {
    l13x::init(config);
}

/// 按配置初始化 HC32L110 时钟树。
///
/// 执行 RM 3.2.1 标准切换流程并写入频率快照（[`clocks`]）。可重复调
/// 用（运行中重新配置），旧时钟源为 XTH 时其频率未知，FLASH 等待周期
/// 按保守的 32 MHz 处理。
///
/// # Panics
///
/// `sys` 指定的源未在配置中启用（例如 `sys = Xth` 而 `xth = None`），
/// 或该源等待稳定超时。
#[cfg(sysctrl_ddl_v2_2)]
pub fn init(config: &Config) {
    l110::init(config);
}

// ---------------------------------------------------------------------------
// 运行期单项开关
// ---------------------------------------------------------------------------

/// 深度休眠唤醒后是否强制使用 RCH（SYSCTRL0.WAKEUP_BYRCH，RM 3.5.1 位 15）。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
pub fn set_wakeup_by_rch(enable: bool) {
    backend::write_sysctrl0(|w| w.set_wakeup_byrch(enable));
}

/// XTL 高级使能（SYSCTRL1.XTL_ALWAYS_ON，RM 3.5.2 位 3）。
///
/// 置 1 后 SYSCTRL0.XTL_EN 只可置位不可清零。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
pub fn set_xtl_always_on(enable: bool) {
    #[cfg(sysctrl_ddl_v2_2)]
    backend::write_sysctrl1(|w| w.set_xtl_alwayson(enable));
    #[cfg(sysctrl_ddl_v3_1)]
    backend::write_sysctrl1(|w| w.set_xtl_always_on(enable));
}

/// RTC 低功耗模式（SYSCTRL1.RTC_LPW，RM 3.5.2 位 5）。
///
/// 使能后 RTC 寄存器不可读写。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
pub fn set_rtc_lpw(enable: bool) {
    backend::write_sysctrl1(|w| w.set_rtc_lpw(enable));
}

/// RESET 引脚改作 GPIO（SYSCTRL1.RES_UIO，仅 L110）。
#[cfg(sysctrl_ddl_v2_2)]
pub fn set_res_pin_gpio(enable: bool) {
    backend::write_sysctrl1(|w| w.set_res_uio(enable));
}

/// Cortex-M0+ LockUp 时复位 MCU（SYSCTRL1.LOCKUP_EN，仅 L13x）。
#[cfg(sysctrl_ddl_v3_1)]
pub fn set_lockup_reset(enable: bool) {
    backend::write_sysctrl1(|w| w.set_lockup_en(enable));
}

/// SWD 引脚改作 GPIO（SYSCTRL1 位 8）。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
pub fn set_swd_pin_gpio(enable: bool) {
    #[cfg(sysctrl_ddl_v2_2)]
    backend::write_sysctrl1(|w| w.set_swd_uio(enable));
    #[cfg(sysctrl_ddl_v3_1)]
    backend::write_sysctrl1(|w| w.set_swd_use_io(enable));
}

/// RTC 高速补偿时钟选择（SYSCTRL1.RTC_FREQ_ADJUST，RM 3.5.2 位 11:9）。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
#[cfg(sysctrl_ddl_v2_2)]
pub fn set_systick_clock(sel: SysTickClock) {
    enable_peripheral(PeripheralGate::Tick, true);
    backend::modify_systick_cr(|w| {
        w.set_noref(false);
        w.set_clk_sel(sel.encode());
    });
}

/// SysTick 改用 HCLK（SYSTICK_CR.NOREF = 1，RM 3.5.9 位 25）。
#[cfg(sysctrl_ddl_v2_2)]
pub fn set_systick_hclk() {
    enable_peripheral(PeripheralGate::Tick, true);
    backend::modify_systick_cr(|w| w.set_noref(true));
}

/// 读取 STCALIB：参考时钟为 XTL 时的 10 ms 校准值（RM 3.5.9 位 23:0）。
#[cfg(sysctrl_ddl_v2_2)]
pub fn systick_calib() -> u32 {
    backend::read_systick_cr().stcalib()
}

// ---------------------------------------------------------------------------
// HCLK/PCLK 波形输出（时钟分频例程的观测点）
// ---------------------------------------------------------------------------

/// 输出分频（GPIO CTRL1.HCLK_SEL/PCLK_SEL）。
#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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

#[cfg(any(sysctrl_ddl_v2_2, sysctrl_ddl_v3_1))]
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
#[cfg(sysctrl_ddl_v2_2)]
pub fn enable_hclk_output_p24(div: OutputDiv) {
    crate::gpio::backend_enable_hclk_p24(div.encode());
}

/// 停止 P24 的 HCLK 输出（CTRL1.HCLK_EN = 0）。
#[cfg(sysctrl_ddl_v2_2)]
pub fn disable_hclk_output_p24() {
    crate::gpio::backend_disable_hclk_p24();
}

/// P31 输出 PCLK 波形（P31_SEL=3，GPIO CTRL1.PCLK_EN，RM 6.5.5/SDK
/// `Gpio_SetFunc_PCLKOUT_P31`，SDK 同时清 HCLK_EN——SDK 宏即如此，RM
/// 未要求，无害保守）。
///
/// 非加密芯片上电后 P31 默认为 SWCLK 调试引脚（RM 25.3.2），本函数同时
/// 置 SYSCTRL1.SWD_USE_IO 释放该引脚。
#[cfg(sysctrl_ddl_v2_2)]
pub fn enable_pclk_output_p31(div: OutputDiv) {
    enable_gpio();
    set_swd_pin_gpio(true);
    crate::gpio::backend_enable_pclk_p31(div.encode());
}

/// 停止 P31 的 PCLK 输出（CTRL1.PCLK_EN = 0）。
#[cfg(sysctrl_ddl_v2_2)]
pub fn disable_pclk_output_p31() {
    crate::gpio::backend_disable_pclk_p31();
}

/// PA01 输出 HCLK 波形（PA01_SEL=6，GPIO CTRL1.HCLK_EN，仅 L13x）。
#[cfg(sysctrl_ddl_v3_1)]
pub fn enable_hclk_output_pa01(div: OutputDiv) {
    crate::gpio::backend_enable_hclk_pa01(div.encode());
}

/// 停止 PA01 的 HCLK 输出（CTRL1.HCLK_EN = 0，仅 L13x）。
#[cfg(sysctrl_ddl_v3_1)]
pub fn disable_hclk_output_pa01() {
    crate::gpio::backend_disable_hclk_pa01();
}

/// PA02 输出 PCLK 波形（PA02_SEL=6，GPIO CTRL1.PCLK_EN，仅 L13x）。
#[cfg(sysctrl_ddl_v3_1)]
pub fn enable_pclk_output_pa02(div: OutputDiv) {
    crate::gpio::backend_enable_pclk_pa02(div.encode());
}

/// 停止 PA02 的 PCLK 输出（CTRL1.PCLK_EN = 0，仅 L13x）。
#[cfg(sysctrl_ddl_v3_1)]
pub fn disable_pclk_output_pa02() {
    crate::gpio::backend_disable_pclk_pa02();
}
