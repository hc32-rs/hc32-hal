//! 基本定时器 TIM0/1/2（HC32L110，BT `ddl_v1`）。
//!
//! 三个功能完全相同、彼此独立的同步定时/计数器（RM 第 9 章）。每个定时器
//! 有两种工作模式（CR.MD）：
//!
//! - 模式 1：32 位自由计数（CNT32），计到 0xFFFF_FFFF 后产生中断并从 0
//!   继续计数；
//! - 模式 2：16 位自动重装载（CNT + ARR），计到 0xFFFF 后产生中断并重载
//!   ARR 继续计数。
//!
//! 定时功能使用 PCLK，预分频 CR.PRS 为 1/2/4/8/16/32/64/256。置 CR.CT = 1
//! 切换为计数器功能，由 EXT 引脚输入的下降沿计数（输入经 PCLK 采样，频率
//! 须低于采样时钟的 1/2，RM 9.4.4 CT 字段）。
//!
//! 门控（CR.GATE）：仅当 GATE 输入电平有效且 TR = 1 时才计数；极性由
//! CR.GATE_P 选择，复位默认高电平有效。GATE 来源由 GPIO_CTRL4 的
//! `tmx_gate_sel` 字段选择（引脚或 UART RX，RM 6.5.5.4）。
//!
//! TOG 翻转输出（CR.TOG_EN = 1）：TOG/TOGN 输出相位相反的方波，可驱动
//! Buzzer；TOG0/TOG1 还分别连接 UART0/UART1 的波特率发生器（RM 9.3.2）。
//! 引脚复用编码见 RM 表 6-2，由 [`crate::gpio`] 的复用配置或手写 Pxy_SEL
//! 完成。中断标志 TF 在 IFR 中硬件置位，写 ICLR.TFC = 0 清除（写 1 无效，
//! RM 9.4.6）。

use crate::clock::{PeripheralGate, clocks, enable_peripheral};
use crate::pac;
use crate::peripheral::Peri;
use crate::time::Hertz;

/// TIM0/1/2 预分频（CR.PRS，RM 9.4.4 位 6:4）。
///
/// 判别值即寄存器编码；注意编码 7 是 /256 而非 2 的幂连续（RM 9.2 分频
/// 表：1/2/4/8/16/32/64/256）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Prescaler {
    /// PCLK / 1（编码 0，复位默认）。
    #[default]
    Div1 = 0,
    /// PCLK / 2。
    Div2 = 1,
    /// PCLK / 4。
    Div4 = 2,
    /// PCLK / 8。
    Div8 = 3,
    /// PCLK / 16。
    Div16 = 4,
    /// PCLK / 32。
    Div32 = 5,
    /// PCLK / 64。
    Div64 = 6,
    /// PCLK / 256。
    Div256 = 7,
}

impl crate::time::Prescaler for Prescaler {
    fn num(&self) -> u32 {
        1
    }

    fn denom(&self) -> u32 {
        match *self as u32 {
            7 => 256,
            n => 1u32 << n,
        }
    }
}

/// 工作模式（CR.MD，RM 9.4.4 位 1）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Mode {
    /// 模式 1：32 位自由计数（编码 0）。
    Free32,
    /// 模式 2：16 位自动重装载（编码 1）。
    Reload16,
}

impl Mode {
    const fn encode(self) -> bool {
        matches!(self, Mode::Reload16)
    }
}

/// GATE 门控输入来源（GPIO_CTRL4，RM 6.5.5.4）。
///
/// 每个定时器对应一个 2 位字段：TIM0 → `tm0_gate_sel`、TIM1 →
/// `tm1_gate_sel`、TIM2 → `tm2_gate_sel`（LPTIM → `tm3_gate_sel`）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum GateSource {
    /// 对应的 TIMx_GATE 引脚（编码 0，复位默认）。
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

/// 门控极性（CR.GATE_P，RM 9.4.4 位 9）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum GatePolarity {
    /// 高电平为门控有效电平（编码 0，复位默认）。
    #[default]
    ActiveHigh,
    /// 低电平为门控有效电平（编码 1）。
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

/// 按目标溢出周期选择预分频与重载值（模式 2）。
///
/// 周期公式（RM 9.2/9.2.2）：计数器从 ARR 数到 0xFFFF 后重载，溢出间隔
/// 为 `(0x1_0000 - ARR) * PRS / PCLK`。遍历 8 档预分频取第一个计数值不
/// 超过 16 位范围的档位；周期短到任何档位都无法表示时返回 `None`。
pub fn period_to_reload(period: Hertz) -> Option<(Prescaler, u16)> {
    // PCLK 周期数为期望溢出间隔折算的计数节拍总数。
    let ticks = clocks().pclk.0 / period.0;
    for div in 0..8u32 {
        // 编码 7 为 /256（RM 9.2 分频表），非 2 的幂连续。
        let denom = if div == 7 { 256 } else { 1u32 << div };
        let counts = ticks / denom;
        if counts == 0 || counts > 0x1_0000 {
            continue;
        }
        let pre = match div {
            0 => Prescaler::Div1,
            1 => Prescaler::Div2,
            2 => Prescaler::Div4,
            3 => Prescaler::Div8,
            4 => Prescaler::Div16,
            5 => Prescaler::Div32,
            6 => Prescaler::Div64,
            _ => Prescaler::Div256,
        };
        // ARR = 0x1_0000 - counts；counts ∈ (0, 0x1_0000] 保证 u16 可存。
        let arr = 0x1_0000 - counts.max(1);
        return Some((pre, arr as u16));
    }
    None
}

/// 模式 2 的溢出周期（供校验/显示）。
pub fn reload_to_period(pre: Prescaler, arr: u16) -> Hertz {
    let counts = 0x1_0000 - arr as u32;
    Hertz(clocks().pclk.0 / counts / crate::time::Prescaler::denom(&pre))
}

// ---------------------------------------------------------------------------
// 驱动
// ---------------------------------------------------------------------------

/// 基本定时器驱动（TIM0/1/2）。
pub struct Bt<'d, T: Instance> {
    /// 持有实例单例维持独占；寄存器访问经 PAC 全局常量。
    #[allow(dead_code)]
    peri: Peri<'d, T>,
}

impl<'d, T: Instance> Bt<'d, T> {
    /// 接管实例并按目标溢出周期配置为模式 2 定时器（含门控关闭、标志清
    /// 零），不启动。
    ///
    /// # Panics
    ///
    /// 周期超出可配置范围（[`period_to_reload`] 返回 `None`）时 panic。
    pub fn new_periodic(peri: Peri<'d, T>, period: Hertz) -> Self {
        let (pre, arr) = period_to_reload(period)
            .unwrap_or_else(|| panic!("bt: 周期 {} Hz 超出 16 位重载范围", period.0));
        Self::new_mode2(peri, pre, arr)
    }

    /// 接管实例并配置为模式 2 定时器，手动指定预分频与重载值，不启动。
    ///
    /// CNT 初值写为 ARR（首个周期即完整周期）。
    pub fn new_mode2(peri: Peri<'d, T>, pre: Prescaler, arr: u16) -> Self {
        let mut this = Self::take(peri);
        let r = this.regs();
        r.arr().write(|w| w.set_arr(arr));
        r.cnt().write(|w| w.set_cnt(arr));
        this.write_cr(|w| {
            w.set_md(Mode::Reload16.encode());
            w.set_ct(false);
            w.set_prs(pre as u8);
        });
        this.finish_setup();
        this
    }

    /// 接管实例并配置为模式 1 的 32 位自由计数定时器，不启动。
    ///
    /// CNT32 初值为 0。
    pub fn new_free32(peri: Peri<'d, T>, pre: Prescaler) -> Self {
        let mut this = Self::take(peri);
        this.write_cr(|w| {
            w.set_md(Mode::Free32.encode());
            w.set_ct(false);
            w.set_prs(pre as u8);
        });
        this.finish_setup();
        this
    }

    /// 接管实例并配置为外部事件计数器（EXT 引脚下降沿，模式 2 重载），
    /// 不启动。
    ///
    /// 计数模式下 EXT 输入由 PCLK 采样，外部输入频率须低于采样时钟的 1/2
    /// （RM 9.4.4 CT 字段）。
    pub fn new_counter(peri: Peri<'d, T>) -> Self {
        let mut this = Self::take(peri);
        this.write_cr(|w| {
            w.set_md(Mode::Reload16.encode());
            w.set_ct(true);
            w.set_prs(Prescaler::Div1 as u8);
        });
        this.finish_setup();
        this
    }

    fn take(peri: Peri<'d, T>) -> Self {
        enable_peripheral(PeripheralGate::BaseTim, true);
        Self { peri }
    }

    /// 构造收尾：停表、清标志、关门控、关翻转输出。
    fn finish_setup(&mut self) {
        self.stop();
        self.clear_interrupt();
        self.set_gate(None);
        self.write_cr(|w| w.set_tog_en(false));
    }

    /// PAC 寄存器块。
    fn regs(&self) -> pac::bt_ddl_v1::Bt {
        T::regs()
    }

    fn write_cr(&self, f: impl FnOnce(&mut pac::bt_ddl_v1::regs::Cr)) {
        self.regs().cr().modify(f);
    }

    /// 启动定时器（CR.TR = 1）。
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

    /// 配置门控：`None` 关闭门控（TR = 1 即计数）；`Some(cfg)` 使能门控
    /// 并选择 GPIO_CTRL4 中本实例的来源字段。
    pub fn set_gate(&mut self, gate: Option<GateConfig>) {
        self.write_cr(|w| match gate {
            None => w.set_gate(false),
            Some(cfg) => {
                w.set_gate(true);
                w.set_gate_p(matches!(cfg.polarity, GatePolarity::ActiveLow));
            }
        });
        if let Some(cfg) = gate {
            T::set_gate_source(cfg.source.encode());
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

    /// 清除溢出标志：写 ICLR.TFC = 0（写 0 清除，写 1 无效，RM 9.4.6）。
    pub fn clear_interrupt(&mut self) {
        self.regs().iclr().write(|w| w.set_tfc(false));
    }

    /// 使能 TOG/TOGN 翻转输出（CR.TOG_EN = 1）。
    ///
    /// TOG 在每次模式 2 重载时翻转（RM 9.2.2 Buzzer 表）。引脚复用编码见
    /// RM 表 6-2（如 P01 = TIM0_TOG 编码 4、P15 = TIM2_TOG 编码 2）。
    pub fn enable_toggle_output(&mut self) {
        self.write_cr(|w| w.set_tog_en(true));
    }

    /// 关闭 TOG/TOGN 输出：TOG_EN 置 0 后 TOG/TOGN 同时输出 0（RM 9.2.2）。
    pub fn disable_toggle_output(&mut self) {
        self.write_cr(|w| w.set_tog_en(false));
    }

    /// 读模式 2 计数值（CNT，16 位）。
    pub fn count16(&self) -> u16 {
        self.regs().cnt().read().cnt()
    }

    /// 读模式 1 计数值（CNT32，32 位）。
    pub fn count32(&self) -> u32 {
        self.regs().cnt32().read().cnt32()
    }

    /// 写模式 1 初始计数值（CNT32）。
    pub fn set_count32(&mut self, value: u32) {
        self.regs().cnt32().write(|w| w.set_cnt32(value));
    }

    /// 更新模式 2 重载值（ARR）。
    pub fn set_reload(&mut self, arr: u16) {
        self.regs().arr().write(|w| w.set_arr(arr));
    }

    /// 当前预分频档位下的计数时钟频率（PCLK / PRS，编码 7 为 /256）。
    pub fn counter_clock(&self) -> Hertz {
        let prs = self.regs().cr().read().prs();
        let denom = if prs == 7 { 256 } else { 1u32 << prs };
        Hertz(clocks().pclk.0 / denom)
    }
}

/// 基本定时器实例能力。
#[allow(private_bounds)]
pub trait Instance: crate::peripheral::PeripheralType + 'static {
    /// 取 PAC 寄存器块。
    fn regs() -> pac::bt_ddl_v1::Bt;

    /// 写 GPIO_CTRL4 中本实例的 gate_sel 字段。
    fn set_gate_source(code: u8);
}

// ---------------------------------------------------------------------------
// 实例单例
// ---------------------------------------------------------------------------

macro_rules! bt_instance {
    ($name:ident, $pac_reg:ident, $gate_field:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Copy, Clone)]
        pub struct $name {
            _private: (),
        }

        impl crate::peripheral::PeripheralType for $name {}

        impl $name {
            /// 直接取得该实例，绕过 [`crate::init`] 的独占检查。
            ///
            /// # Safety
            ///
            /// 调用方须保证同一实例不被两处同时持有。
            pub unsafe fn steal() -> crate::peripheral::Peri<'static, Self> {
                unsafe { crate::peripheral::Peri::new_unchecked(Self { _private: () }) }
            }
        }

        impl Instance for $name {
            fn regs() -> pac::bt_ddl_v1::Bt {
                pac::$pac_reg
            }

            fn set_gate_source(code: u8) {
                pac::GPIO.ctrl4().modify(|w| w.$gate_field(code));
            }
        }
    };
}

bt_instance!(Bt0, BT0, set_tm0_gate_sel, "TIM0 实例单例。");
bt_instance!(Bt1, BT1, set_tm1_gate_sel, "TIM1 实例单例。");
bt_instance!(Bt2, BT2, set_tm2_gate_sel, "TIM2 实例单例。");
