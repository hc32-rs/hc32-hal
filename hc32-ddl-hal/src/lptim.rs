//! 低功耗定时器 LPTIMER（HC32L110，LPTIM `ddl_v1`）。
//!
//! 异步 16 位定时/计数器（RM 第 10 章）。系统时钟关闭后仍可用内部低速
//! RCL 或外部低速晶振 XTL 计时，并通过中断在低功耗模式下唤醒系统。
//!
//! - 时钟（CR.TCK_SEL）：PCLK / XTL / RCL，复位默认 PCLK。选择 XTL/RCL
//!   时读 CNT 经同步器，读数有数个时钟的滞后。
//! - 模式（CR.MD）：模式 1 为 16 位自由计数；模式 2 为 16 位自动重装载。
//!   启动定时器时 CNT 由 ARR 自动装载（RM 10.2）。
//! - 计数（CR.CT = 1）：EXT 输入的下降沿计数，来源由 GPIO_CTRL3/VC 互联
//!   选择（RM 10.3.2）。
//! - 门控（CR.GATE）：GATE 电平有效且 TR = 1 才计数；来源由 GPIO_CTRL4
//!   `tm3_gate_sel` 选择（RM 6.5.5.4）。
//!
//! ARR 写同步：写 ARR 前必须读 CR.WT_FLAG，仅当 WT_FLAG = 1（写同步完
//! 成）时写入才有效，写入后 WT_FLAG 变低等待硬件同步（RM 10.4.2）。使用
//! 中断时重载值应小于 0xFFFC，否则中断可能丢失（RM 10.2 注意事项）。

use crate::clock::{PeripheralGate, clocks, enable_peripheral};
use crate::pac;
use crate::peripheral::Peri;
use crate::time::Hertz;

/// LPTIMER 计数时钟（CR.TCK_SEL，RM 10.4.3 位 5:4）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum ClockSource {
    /// PCLK（编码 00，复位默认）。读 CNT 经同步。
    #[default]
    Pclk,
    /// 外部低速晶振 XTL 32.768 kHz（编码 10）。
    Xtl,
    /// 内部低速 RC RCL（编码 11）。
    Rcl,
}

impl ClockSource {
    const fn encode(self) -> u8 {
        match self {
            ClockSource::Pclk => 0,
            ClockSource::Xtl => 2,
            ClockSource::Rcl => 3,
        }
    }

    /// 运行期时钟频率。
    ///
    /// PCLK 取 [`crate::clock::clocks`] 快照；XTL/RCL 按标称 32.768 kHz
    /// （RCL 校准为 38.4 kHz 时由调用方自行换算）。
    pub fn frequency(self) -> Hertz {
        match self {
            ClockSource::Pclk => clocks().pclk,
            ClockSource::Xtl | ClockSource::Rcl => Hertz(32_768),
        }
    }
}

/// 工作模式（CR.MD，RM 10.4.3 位 1）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Mode {
    /// 模式 1：16 位自由计数（编码 0）。
    Free16,
    /// 模式 2：16 位自动重装载（编码 1）。
    Reload16,
}

impl Mode {
    const fn encode(self) -> bool {
        matches!(self, Mode::Reload16)
    }
}

/// GATE 门控输入来源（GPIO_CTRL4 `tm3_gate_sel`，RM 6.5.5.4）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum GateSource {
    /// LPTIM_GATE 引脚（编码 0，复位默认）。
    #[default]
    Pin,
    /// UART0 RXD（编码 1）。
    Uart0Rxd,
    /// UART1 RXD（编码 2）。
    Uart1Rxd,
    /// LPUART RXD（编码 3）。
    LpuartRxd,
}

impl GateSource {
    const fn encode(self) -> u8 {
        match self {
            GateSource::Pin => 0,
            GateSource::Uart0Rxd => 1,
            GateSource::Uart1Rxd => 2,
            GateSource::LpuartRxd => 3,
        }
    }
}

/// 门控极性（CR.GATE_P，RM 10.4.3 位 9）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum GatePolarity {
    /// 高电平有效（编码 0，复位默认）。
    #[default]
    ActiveHigh,
    /// 低电平有效（编码 1）。
    ActiveLow,
}

/// 门控配置。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct GateConfig {
    /// 门控来源。
    pub source: GateSource,
    /// 门控极性。
    pub polarity: GatePolarity,
}

// ---------------------------------------------------------------------------
// 频率换算（模式 2）
// ---------------------------------------------------------------------------

/// 按目标溢出周期选择重载值（模式 2）。
///
/// 溢出间隔为 `(0x1_0000 - ARR) / 时钟频率`。LPTIMER 无预分频，周期太短
/// 时返回 `None`。
///
/// 本函数**无条件**要求结果 ARR < 0xFFFC（即溢出节拍数 > 4 的极短周期被
/// 拒绝）：RM 10.2 规定重载值 ≥ 0xFFFC（ARR = 0x1_0000 − 节拍数，对应
/// 1–4 个节拍）时中断会丢失，为覆盖后续可能开启中断的用法，这里统一按
/// 该限制处理。纯轮询、确需极短周期的场合绕开本函数直接写 ARR。
pub fn period_to_reload(period: Hertz, clock: Hertz) -> Option<u16> {
    let ticks = clock.0 / period.0;
    if ticks == 0 {
        return None;
    }
    let arr = 0x1_0000 - ticks;
    if arr >= 0xFFFC {
        return None;
    }
    Some(arr as u16)
}

/// 模式 2 的溢出周期。
pub fn reload_to_period(arr: u16, clock: Hertz) -> Hertz {
    Hertz(clock.0 / (0x1_0000 - arr as u32))
}

// ---------------------------------------------------------------------------
// 驱动
// ---------------------------------------------------------------------------

/// 低功耗定时器驱动。
pub struct LpTimer<'d> {
    /// 持有实例单例维持独占；寄存器访问经 PAC 全局常量。
    #[allow(dead_code)]
    peri: Peri<'d, LpTim>,
    _lifetime: core::marker::PhantomData<&'d mut ()>,
}

impl<'d> LpTimer<'d> {
    /// 接管实例并配置为模式 2 定时器（含时钟选择），不启动。
    ///
    /// `clock` 选 PCLK 时使用时钟快照频率；选 XTL/RCL 时按 32.768 kHz
    /// 标称值换算 ARR。
    ///
    /// # Panics
    ///
    /// 周期无法以 16 位重载表示（或触发 RM 10.2 中断丢失限制）时 panic。
    pub fn new_periodic(peri: Peri<'d, LpTim>, period: Hertz, clock: ClockSource) -> Self {
        let hz = clock.frequency();
        let arr = period_to_reload(period, hz)
            .unwrap_or_else(|| panic!("lptim: 周期 {} Hz 超出 16 位重载范围", period.0));
        let mut this = Self::take(peri);
        this.stop();
        this.clear_interrupt();
        this.set_gate(None);
        this.write_cr(|w| {
            w.set_md(Mode::Reload16.encode());
            w.set_ct(false);
            w.set_tck_sel(clock.encode());
        });
        this.set_reload(arr);
        this
    }

    /// 接管实例并配置为模式 1 的 16 位自由计数定时器，不启动。
    pub fn new_free16(peri: Peri<'d, LpTim>, clock: ClockSource) -> Self {
        let mut this = Self::take(peri);
        this.stop();
        this.clear_interrupt();
        this.set_gate(None);
        this.write_cr(|w| {
            w.set_md(Mode::Free16.encode());
            w.set_ct(false);
            w.set_tck_sel(clock.encode());
        });
        this
    }

    /// 接管实例并配置为外部事件计数器（EXT 下降沿，模式 2 重载），不启动。
    ///
    /// EXT 输入由 TCK_SEL 选择的计数时钟采样，外部输入频率须低于采样时钟
    /// 的 1/2（RM 10.4.3 CT 字段）。引脚复用如 P03 = LPTIM_EXT（编码 4）。
    pub fn new_counter(peri: Peri<'d, LpTim>) -> Self {
        let mut this = Self::take(peri);
        this.stop();
        this.clear_interrupt();
        this.set_gate(None);
        this.write_cr(|w| {
            w.set_md(Mode::Reload16.encode());
            w.set_ct(true);
            w.set_tck_sel(ClockSource::Pclk.encode());
        });
        this
    }

    fn take(peri: Peri<'d, LpTim>) -> Self {
        enable_peripheral(PeripheralGate::LpTim, true);
        Self {
            peri,
            _lifetime: core::marker::PhantomData,
        }
    }

    fn regs(&self) -> pac::lptim_ddl_v1::Lptim {
        pac::LPTIMER
    }

    fn write_cr(&self, f: impl FnOnce(&mut pac::lptim_ddl_v1::regs::Cr)) {
        self.regs().cr().modify(f);
    }

    /// 启动定时器（CR.TR = 1）。启动时 CNT 由 ARR 自动装载（RM 10.2）。
    pub fn start(&mut self) {
        self.write_cr(|w| w.set_tr(true));
    }

    /// 停止定时器（CR.TR = 0）。
    pub fn stop(&mut self) {
        self.write_cr(|w| w.set_tr(false));
    }

    /// 当前是否在运行（CR.TR）。
    pub fn is_running(&self) -> bool {
        self.regs().cr().read().tr()
    }

    /// 写重载值（ARR）。
    ///
    /// 遵守 RM 10.4.2 写同步：先等待 CR.WT_FLAG = 1 再写入。等待带超时上
    /// 限，超时 panic（硬件异常时避免死循环）。
    pub fn set_reload(&mut self, arr: u16) {
        // WT_FLAG = 1 表示同步完成，此时写入有效（RM 10.4.2）。
        for _ in 0..1_000_000 {
            if self.regs().cr().read().wt_flag() {
                self.regs().arr().write(|w| w.set_arr(arr));
                return;
            }
        }
        panic!("lptim: 等待 WT_FLAG 同步超时");
    }

    /// 读当前计数值（CNT，只读）。
    pub fn count(&self) -> u16 {
        self.regs().cnt().read().cnt()
    }

    /// 配置门控：`None` 关闭；`Some(cfg)` 使能并写 GPIO_CTRL4.tm3_gate_sel。
    pub fn set_gate(&mut self, gate: Option<GateConfig>) {
        self.write_cr(|w| match gate {
            None => w.set_gate(false),
            Some(cfg) => {
                w.set_gate(true);
                w.set_gate_p(matches!(cfg.polarity, GatePolarity::ActiveLow));
            }
        });
        if let Some(cfg) = gate {
            pac::GPIO
                .ctrl4()
                .modify(|w| w.set_tm3_gate_sel(cfg.source.encode()));
        }
    }

    /// 使能溢出中断（CR.IE = 1）。
    pub fn enable_interrupt(&mut self) {
        self.write_cr(|w| w.set_ie(true));
    }

    /// 关闭溢出中断（CR.IE = 0）。
    pub fn disable_interrupt(&mut self) {
        self.write_cr(|w| w.set_ie(false));
    }

    /// 溢出标志（IFR.TF）是否置位。
    pub fn is_interrupt_pending(&self) -> bool {
        self.regs().ifr().read().tf()
    }

    /// 清除溢出标志：写 ICLR.TFC = 0（写 0 清除，RM 10.4.5）。
    pub fn clear_interrupt(&mut self) {
        self.regs().iclr().write(|w| w.set_tfc(false));
    }

    /// 使能 TOG/TOGN 翻转输出（CR.TOG_EN = 1），可驱动 Buzzer（RM 10.3.3）。
    ///
    /// 引脚复用如 P31 = LPTIM_TOG（编码 1）、P32 = LPTIM_TOGN（编码 1）。
    pub fn enable_toggle_output(&mut self) {
        self.write_cr(|w| w.set_tog_en(true));
    }

    /// 关闭 TOG/TOGN 输出。
    pub fn disable_toggle_output(&mut self) {
        self.write_cr(|w| w.set_tog_en(false));
    }
}

/// LPTIMER 实例单例。
#[derive(Copy, Clone)]
pub struct LpTim {
    _private: (),
}

impl crate::peripheral::PeripheralType for LpTim {}

impl LpTim {
    /// 直接取得该实例，绕过 [`crate::init`] 的独占检查。
    ///
    /// # Safety
    ///
    /// 调用方须保证同一实例不被两处同时持有。
    pub unsafe fn steal() -> crate::peripheral::Peri<'static, Self> {
        unsafe { crate::peripheral::Peri::new_unchecked(Self { _private: () }) }
    }
}
