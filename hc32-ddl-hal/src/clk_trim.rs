//! 时钟校准与监测（HC32L110，CLKTRIM `ddl_v1`，基址 0x4000_1800）。
//!
//! CLKTRIM 用一个精准的参考时钟校准/监测另一个时钟（RM 19 章）：
//!
//! - **校准模式**（RM 19.3.1）：选好参考/被校准时钟，设 REFCNT 后置
//!   `TRIM_START`；参考计数器减到 0 时 `IFR.stop` 置位，读 `CALCNT` 即得
//!   两时钟的频率比。`IFR.calcnt_of` 置位表示被校准计数器中途溢出，
//!   需缩短校准时间重来。
//! - **监测模式**（RM 19.3.2）：只能以 XTH/XTL 为被监测时钟；设
//!   REFCNT（监测间隔）与 CALCON（溢上门限），`mon_en` + `TRIM_START`
//!   启动。间隔到时被监测计数器未溢出则 `IFR.xth_fault`/`xtl_fault`
//!   置位，表示时钟失效。
//!
//! 本模块只做轮询；`CR.IE` 与 NVIC IRQ31（`CLK_TRIM_IRQn`）的绑定留给
//! 中断框架统一接入。

use crate::clock::PeripheralGate;
use crate::clock::enable_peripheral;
use crate::pac;

/// 参考时钟选择（CLKTRIM_CR.REFCLK_SEL，RM 19.4.1 位 3:1）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ReferenceClock {
    /// 内部高速 RCH（编码 000）。
    Rch,
    /// 外部高速晶振 XTH（编码 001）。
    Xth,
    /// 内部低速 RCL（编码 010）。
    Rcl,
    /// 外部低速晶振 XTL（编码 011）。
    Xtl,
    /// 内部 10k 振荡器（编码 100），仅供 WDT 与 CLKTRIM 使用（RM 3.1）。
    Irc10k,
    /// GPIO CTRL1.EXT_CLK_SEL 选择的引脚时钟（编码 101）。
    ExtClkIn,
}

impl ReferenceClock {
    const fn encode(self) -> u8 {
        match self {
            ReferenceClock::Rch => 0,
            ReferenceClock::Xth => 1,
            ReferenceClock::Rcl => 2,
            ReferenceClock::Xtl => 3,
            ReferenceClock::Irc10k => 4,
            ReferenceClock::ExtClkIn => 5,
        }
    }
}

/// 被校准/监测时钟选择（CLKTRIM_CR.CALCLK_SEL，RM 19.4.1 位 5:4）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CalibratedClock {
    /// RCH（编码 00）。
    Rch,
    /// XTH（编码 01）。
    Xth,
    /// RCL（编码 10）。
    Rcl,
    /// XTL（编码 11）。
    Xtl,
}

impl CalibratedClock {
    const fn encode(self) -> u8 {
        match self {
            CalibratedClock::Rch => 0,
            CalibratedClock::Xth => 1,
            CalibratedClock::Rcl => 2,
            CalibratedClock::Xtl => 3,
        }
    }
}

/// CLKTRIM 事件（IFR/ICLR，RM 19.4.5/19.4.6）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TrimEvent {
    /// 参考计数器停止（IFR.stop）。清零 TRIM_START 结束本次运行。
    Stop,
    /// 校准计数器溢出（IFR.calcnt_of）。需缩短校准时间重来。
    CalcOverflow,
    /// XTL 失效（IFR.xtl_fault）。写 ICLR.xtl_fault_clr = 0 清除。
    XtlFault,
    /// XTH 失效（IFR.xth_fault）。写 ICLR.xth_fault_clr = 0 清除。
    XthFault,
}

/// 校准运行结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrateResult {
    /// 参考计数器剩余值（REFCNT，RM 19.4.3）。
    pub refcnt: u32,
    /// 被校准计数器计数值（CALCNT，RM 19.4.4）。
    pub calcnt: u32,
}

/// CLKTRIM 驱动。
///
/// 以 `pac::CLK_TRIM` 单例构造；门控（PERI_CLKEN.Trim）在构造时开启，
/// 不可关闭（无 driver-level 释放语义，与 Embassy rcc 门控同约定）。
pub struct Trim {
    _private: (),
}

impl Trim {
    /// 取得 CLKTRIM 驱动并开启其门控。
    ///
    /// # Safety
    ///
    /// 单例外设，调用方须保证全程序只有一份实例在用。
    pub unsafe fn steal() -> Self {
        enable_peripheral(PeripheralGate::Trim, true);
        Self { _private: () }
    }

    /// 中断使能（CLKTRIM_CR.IE，RM 19.4.1 位 7）。
    ///
    /// NVIC 侧的 IRQ31 绑定由中断框架统一处理，本函数只操作模块级 IE。
    pub fn set_interrupt_enabled(&mut self, enable: bool) {
        pac::CLK_TRIM.cr().modify(|w| w.set_ie(enable));
    }

    // -----------------------------------------------------------------------
    // 校准模式（RM 19.3.1）
    // -----------------------------------------------------------------------

    /// 启动一次校准，阻塞等待参考计数器停止。
    ///
    /// 校准时长 = `ref_cycles` 个参考时钟周期（REFCON.rcntval）。返回
    /// `None` 表示校准计数器中途溢出（IFR.calcnt_of）：校准时间过长，
    /// 需减小 `ref_cycles` 重试（RM 19.3.1 注意）。阻塞轮询 `IFR.stop`
    /// 置位后清零 TRIM_START 结束。轮询带防御性迭代上限（手册未规定超时
    /// 值，上限按 32 位计数器最长周期量级放宽），参考时钟停振时返回
    /// `None` 而非死循环。
    pub fn calibrate_blocking(
        &mut self,
        reference: ReferenceClock,
        calibrated: CalibratedClock,
        ref_cycles: u32,
    ) -> Option<CalibrateResult> {
        pac::CLK_TRIM.refcon().write(|w| w.set_rcntval(ref_cycles));
        pac::CLK_TRIM.cr().modify(|w| {
            w.set_mon_en(false);
            w.set_refclk_sel(reference.encode());
            w.set_calclk_sel(calibrated.encode());
        });
        // 启动并轮询。
        pac::CLK_TRIM.cr().modify(|w| w.set_trim_start(true));
        // 防御上限：正常情况下 ref_cycles 个参考周期内必然置位。
        let deadline = ref_cycles.saturating_mul(8).max(100_000);
        for _ in 0..deadline {
            let ifr = pac::CLK_TRIM.ifr().read();
            if ifr.stop() {
                // RM 19.3.1 Step9：清零 TRIM_START 结束校准（同时清除
                // stop/calcnt_of 标志，RM 19.4.5）。
                pac::CLK_TRIM.cr().modify(|w| w.set_trim_start(false));
                return Some(CalibrateResult {
                    refcnt: pac::CLK_TRIM.refcnt().read().refcnt(),
                    calcnt: pac::CLK_TRIM.calcnt().read().calcnt(),
                });
            }
            if ifr.calcnt_of() {
                // 校准计数器先溢出：结束并报错（RM 19.3.1 注意）。
                pac::CLK_TRIM.cr().modify(|w| w.set_trim_start(false));
                return None;
            }
        }
        // 参考时钟未计数到零（可能停振或配置异常）：结束并报错。
        pac::CLK_TRIM.cr().modify(|w| w.set_trim_start(false));
        None
    }

    /// 校准模式的频率比：`f_calibrated / f_reference = calcnt / ref_cycles`。
    pub fn frequency_ratio(result: &CalibrateResult, ref_cycles: u32) -> (u32, u32) {
        (result.calcnt, ref_cycles)
    }

    // -----------------------------------------------------------------------
    // 监测模式（RM 19.3.2）
    // -----------------------------------------------------------------------

    /// 启动晶振失效监测，不阻塞。
    ///
    /// 被监测时钟只能为 XTH 或 XTL（RM 19.3.2，硬性约束，违反时 panic）。
    /// 监测间隔为 `ref_cycles` 个参考时钟周期；`cal_overflow` 为被监测计
    /// 数器的溢出门限（CALCON.ccntval）。间隔到而计数器未溢出，
    /// `IFR.xth_fault`/`xtl_fault` 置位表示失效。
    ///
    /// # Panics
    ///
    /// `calibrated` 不为 XTH/XTL（RM 19.3.2 只允许监测外部晶振）。
    pub fn start_monitor(
        &mut self,
        reference: ReferenceClock,
        calibrated: CalibratedClock,
        ref_cycles: u32,
        cal_overflow: u32,
    ) {
        assert!(
            matches!(calibrated, CalibratedClock::Xth | CalibratedClock::Xtl),
            "clk_trim: 监测模式被监测时钟只能为 XTH/XTL（RM 19.3.2）"
        );
        pac::CLK_TRIM.refcon().write(|w| w.set_rcntval(ref_cycles));
        pac::CLK_TRIM
            .calcon()
            .write(|w| w.set_ccntval(cal_overflow));
        pac::CLK_TRIM.cr().modify(|w| {
            w.set_mon_en(true);
            w.set_refclk_sel(reference.encode());
            w.set_calclk_sel(calibrated.encode());
            w.set_trim_start(true);
        });
    }

    /// 读取未决事件（IFR，RM 19.4.5）。
    pub fn events(&self) -> TrimEvents {
        let ifr = pac::CLK_TRIM.ifr().read();
        TrimEvents {
            stop: ifr.stop(),
            calcnt_of: ifr.calcnt_of(),
            xtl_fault: ifr.xtl_fault(),
            xth_fault: ifr.xth_fault(),
        }
    }

    /// 清除晶振失效标志（ICLR 写 0 清除，RM 19.4.6）。
    ///
    /// stop/calcnt_of 由清零 TRIM_START 自动清除（RM 19.4.5），不在本
    /// 函数范围。
    pub fn clear_faults(&mut self, event: TrimEvent) {
        match event {
            TrimEvent::XtlFault => pac::CLK_TRIM.iclr().modify(|w| w.set_xtl_fault_clr(false)),
            TrimEvent::XthFault => pac::CLK_TRIM.iclr().modify(|w| w.set_xth_fault_clr(false)),
            TrimEvent::Stop | TrimEvent::CalcOverflow => {
                // stop/calcnt_of 经 TRIM_START 清零，无需操作。
            }
        }
    }

    /// 停止校准/监测（TRIM_START = 0，同时清除 stop/calcnt_of）。
    pub fn stop(&mut self) {
        pac::CLK_TRIM.cr().modify(|w| w.set_trim_start(false));
    }
}

/// IFR 快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrimEvents {
    /// 参考计数器停止。
    pub stop: bool,
    /// 校准计数器溢出。
    pub calcnt_of: bool,
    /// XTL 失效。
    pub xtl_fault: bool,
    /// XTH 失效。
    pub xth_fault: bool,
}
