//! 看门狗定时器（HC32L110，WDT `ddl_v1`）。
//!
//! WDT 由专用 10 kHz 片内 RC 振荡器驱动（RM 14.1），Sleep/DeepSleep 模式下
//! 继续计数（RM 表 2-2/2-3）。20 位递增计数器，溢出时间经 `WDT_CON.WOV`
//! 选择 1.6 ms~52.4 s 共 16 档（RM 14.3.2）。
//!
//! 启动与清零共用同一个序列：向 `WDT_RST` 依次写入 0x1E、0xE1——未启动时
//! 启动定时器，已启动时清零计数器与中断标志（RM 14.3.1）。因此本驱动没有
//! “停止”方法：一旦启动，WDT 只能通过复位结束（RM 14.2 起始说明：
//! `WDT_CON` 仅在未启动时可写）。
//!
//! API 形态参照 Embassy STM32 的 `wdg::IndependentWatchdog`：构造即完成全
//! 部寄存器配置，[`Wdt::start`] 启动（Embassy 的 `unleash`），[`Wdt::feed`]
//! 喂狗（Embassy 的 `pet`）。溢出动作二选一：复位（[`Config::reset`]，芯片
//! 复位、`RESET_FLAG.WDT` 置位）或中断（[`Config::interrupt`]，周期性产生
//! NVIC #22 中断（RM 表 5-2），需在处理中喂狗清标志，RM 14.2.1 Step5）。
//!
//! ```
//! # use hc32_ddl_hal::wdt;
//! // 1.038s 溢出复位，周期喂狗避免复位。
//! let mut wdt = wdt::Wdt::new(wdt::Config::reset(wdt::Period::Ms102));
//! wdt.start();
//! wdt.feed();
//! ```
//!
//! # RM 与 SDK 的溢出档位差异
//!
//! RM 14.3.2 的 WOV=8 档标称 500 ms，与其余档位不符（1.6 ms × 2ⁿ 规律推算
//! 应约 410 ms）；HC32F003 SDK `wdt.h` 同样标 500 ms。本驱动以 RM 文本为
//! 准提供 [`Period::Ms500`]，实际时长待板测确认（差异台账待验证条目）。

use crate::pac;

/// WDT 溢出时间档位（`WDT_CON.WOV`，RM 14.3.2）。
///
/// 判别值即寄存器编码。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Period {
    /// 1.6 ms（编码 0）。
    Ms1_6,
    /// 3.2 ms（编码 1）。
    Ms3_2,
    /// 6.4 ms（编码 2）。
    Ms6_4,
    /// 13 ms（编码 3）。
    Ms13,
    /// 26 ms（编码 4）。
    Ms26,
    /// 51 ms（编码 5）。
    Ms51,
    /// 102 ms（编码 6）。
    Ms102,
    /// 205 ms（编码 7）。
    Ms205,
    /// 500 ms（编码 8）。
    ///
    /// RM 14.3.2 标称值，与其余档位的 1.6 ms × 2ⁿ 规律不符（推算约
    /// 410 ms），实际时长待板测确认（见模块文档“RM 与 SDK 的溢出档位
    /// 差异”）。
    Ms500,
    /// 820 ms（编码 9）。
    Ms820,
    /// 1.64 s（编码 10）。
    S1_64,
    /// 3.28 s（编码 11）。
    S3_28,
    /// 6.55 s（编码 12）。
    S6_55,
    /// 13.1 s（编码 13）。
    S13_1,
    /// 26.2 s（编码 14）。
    S26_2,
    /// 52.4 s（编码 15）。
    S52_4,
}

impl Period {
    /// 溢出时间的标称值，单位毫秒（RM 14.3.2 / DS 表 7-13）。
    ///
    /// 小数档位（1.6/3.2/6.4 ms）统一向上取整——喂狗窗口
    /// （[`Period::feed_window_millis`]）偏长不影响安全，偏短可能漏喂。
    pub const fn as_millis(self) -> u32 {
        match self {
            Period::Ms1_6 => 2,
            Period::Ms3_2 => 4,
            Period::Ms6_4 => 7,
            Period::Ms13 => 13,
            Period::Ms26 => 26,
            Period::Ms51 => 51,
            Period::Ms102 => 102,
            Period::Ms205 => 205,
            Period::Ms500 => 500,
            Period::Ms820 => 820,
            Period::S1_64 => 1_640,
            Period::S3_28 => 3_280,
            Period::S6_55 => 6_550,
            Period::S13_1 => 13_100,
            Period::S26_2 => 26_200,
            Period::S52_4 => 52_400,
        }
    }

    /// 溢出前的安全喂狗窗口，单位毫秒。
    ///
    /// RM 14.2.2 注：WDT 时钟为低精度 RC 振荡器，强烈建议在计数值到达
    /// 溢出值一半之前喂狗。返回 `as_millis / 2`。
    pub const fn feed_window_millis(self) -> u32 {
        self.as_millis() / 2
    }
}

/// WDT 溢出后的动作。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Action {
    /// 产生芯片复位（`WINT_EN = 0`，RM 14.2.2），`RESET_FLAG.WDT` 置位。
    Reset,
    /// 产生中断（`WINT_EN = 1`，NVIC #22（RM 表 5-2），RM 14.2.1），需在
    /// 处理程序中调用 [`Wdt::feed`] 清除标志。
    Interrupt,
}

/// WDT 配置。
#[derive(Debug, Copy, Clone)]
pub struct Config {
    /// 溢出时间档位。
    pub period: Period,
    /// 溢出后的动作。
    pub action: Action,
}

impl Config {
    /// 溢出产生芯片复位（RM 14.2.2 配置流程）。
    pub const fn reset(period: Period) -> Self {
        Self {
            period,
            action: Action::Reset,
        }
    }

    /// 溢出产生中断（RM 14.2.1 配置流程）。
    ///
    /// 中断模式需在 NVIC 侧使能 WDT 向量（#22，`rt` feature 下的
    /// [`crate::pac::interrupt`]），本驱动只负责模块级配置。
    pub const fn interrupt(period: Period) -> Self {
        Self {
            period,
            action: Action::Interrupt,
        }
    }
}

/// 向 WDT_RST 依次写入启动/清零序列 0x1E、0xE1（RM 14.3.1）。
fn write_rst_sequence() {
    pac::WDT.rst().write(|w| w.set_wdtrst(0x1E));
    pac::WDT.rst().write(|w| w.set_wdtrst(0xE1));
}

/// WDT 驱动。
///
/// 单例外设，无需持有引脚或 DMA 资源；构造时开启 PERI_CLKEN.WDT 门控并
/// 写 `WDT_CON`（RM 14.2：`WDT_CON` 仅在未启动时可修改）。
///
/// WDT 一旦启动无法停止或重配（RM 14.3.1/14.3.2 未定义停止序列或停止
/// 位），驱动不提供相应方法；中断模式下也无法静默清标志——RM 14.2.1
/// Step5 要求在中断处理中执行清零序列。
pub struct Wdt {
    _private: (),
}

impl Wdt {
    /// 按 [`Config`] 配置并构造 WDT（不启动）。
    ///
    /// 写 `WDT_CON` 前要求 WDT 未运行（RM 14.3.2 注：仅未运行时可写），
    /// 运行中写入的效果手册未规定——调用方可经 [`Wdt::is_running`] 预判。
    pub fn new(config: Config) -> Self {
        crate::clock::enable_peripheral(crate::clock::PeripheralGate::Wdt, true);
        pac::WDT.con().modify(|w| {
            w.set_wov(config.period as u8);
            w.set_wint_en(matches!(config.action, Action::Interrupt));
        });
        Self { _private: () }
    }

    /// 启动 WDT（RM 14.2.1 Step4 / 14.2.2 Step3）。
    pub fn start(&mut self) {
        write_rst_sequence();
    }

    /// 喂狗：清零计数器与中断标志（RM 14.2.2 Step4 / 14.3.1）。
    ///
    /// 建议在溢出值一半之前调用（RM 14.2.2 注，窗口见
    /// [`Period::feed_window_millis`]）。
    pub fn feed(&mut self) {
        write_rst_sequence();
    }

    /// WDT 是否正在运行（`WDT_CON.WDTR`，RM 14.3.2 位 4）。
    pub fn is_running() -> bool {
        pac::WDT.con().read().wdtr()
    }

    /// WDT 中断标志（`WDT_CON.WDINT`，RM 14.3.2 位 7）。
    ///
    /// 置位后需执行清零序列（[`Wdt::feed`]）清除，标志不清除时中断持续
    /// 触发。
    pub fn interrupt_pending() -> bool {
        pac::WDT.con().read().wdint()
    }

    /// 计数器当前计数值（`WDT_CON.WCNTL`，RM 14.3.2 位 15:8）。
    ///
    /// 计数器为 20 位，本字段只读出低 8 位。
    pub fn counter() -> u8 {
        pac::WDT.con().read().wcntl()
    }
}

/// 上次复位是否由 WDT 溢出引起（`RESET_FLAG.WDT`，RM 4.2.1 位 3）。
///
/// 该标志需软件清除且上电状态不定（RM 4.2.1 位描述）；`clear = true` 时
/// 顺带写 0 清除本次标志。
pub fn reset_caused_by_wdt(clear: bool) -> bool {
    let caused = pac::RESET.reset_flag().read().wdt();
    if clear {
        pac::RESET.reset_flag().modify(|w| w.set_wdt(false));
    }
    return caused;
}
