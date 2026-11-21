//! 可编程计数阵列 PCA（HC32L110，PCA `ddl_v1`）。
//!
//! 一个 16 位向上计数时基（CNT）加 5 个独立的捕获/比较模块（RM 第 11
//! 章）。时基时钟由 CMOD.CPS 选择：PCLK 的 32/16/8/4/2 分频、TIM0/TIM1
//! 溢出、或 ECI 引脚外部时钟（PCLK 4 分频采样，最高 PCLK/8）。
//!
//! 模块工作方式由 CCAPMx 选择：
//!
//! - 捕获：CAPP/CAPN 选择上升沿/下降沿/任意沿触发，捕获值存入 CCAPx，
//!   置 CCON.CCFx；
//! - 软件定时器：ECOM + MAT，计数值与 CCAPx 匹配时置 CCFx；
//! - 高速输出：ECOM + MAT + TOG，匹配时引脚电平翻转（CCAPO 寄存器控制
//!   初值）；
//! - 8 位 PWM：ECOM + PWM，CNTL 与 CCAPxL 比较，CNTL 溢出时 CCAPxH 自动
//!   装载到 CCAPxL（改占空比写 CCAPxH，硬件在周期边界装载，避免毛刺）。
//!
//! 写入顺序（RM 11.2.3.1 注意）：更新 CCAPx 时先写低字节后写高字节——写
//! CCAPxL 会自动清 ECOM，写 CCAPxH 会自动置 ECOM。16 位寄存器 CCAPx 的
//! 单次写入硬件上等价于先低后高。CNT 仅在计数停止（CCON.CR = 0）时可写
//! （RM 11.4.3）。
//!
//! 中断：CF（时基溢出，CMOD.CFIE 使能）与 CCF0–4（模块事件，CCAPMx.CCIE
//! 使能）共用 PCA 中断号；标志写 ICLR 对应位 = 0 清除（写 1 无效，RM
//! 11.4.4）。
//!
//! 模块 4 另有 WDT 模式（CMOD.WDTE），本驱动不涉及（独立 WDT 模块与 PCA
//! WDT 均不在定时器驱动范围内）。

use crate::clock::{PeripheralGate, clocks, enable_peripheral};
use crate::pac;
use crate::peripheral::Peri;
use crate::time::Hertz;

/// PCA 时基时钟（CMOD.CPS，RM 11.4.2 位 3:1）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TimebaseClock {
    /// PCLK / 32（编码 000）。
    PclkDiv32,
    /// PCLK / 16（编码 001）。
    PclkDiv16,
    /// PCLK / 8（编码 010）。
    PclkDiv8,
    /// PCLK / 4（编码 011）。
    PclkDiv4,
    /// PCLK / 2（编码 100）。
    PclkDiv2,
    /// TIM0 溢出（编码 101）。
    Tim0Overflow,
    /// TIM1 溢出（编码 110）。
    Tim1Overflow,
    /// ECI 引脚外部时钟（编码 111）。PCLK 4 分频采样，输入频率不得高于
    /// PCLK/8（RM 11.2.1）。
    Eci,
}

impl TimebaseClock {
    const fn encode(self) -> u8 {
        match self {
            TimebaseClock::PclkDiv32 => 0,
            TimebaseClock::PclkDiv16 => 1,
            TimebaseClock::PclkDiv8 => 2,
            TimebaseClock::PclkDiv4 => 3,
            TimebaseClock::PclkDiv2 => 4,
            TimebaseClock::Tim0Overflow => 5,
            TimebaseClock::Tim1Overflow => 6,
            TimebaseClock::Eci => 7,
        }
    }

    /// PCLK 分频档位的时基频率；外部时钟源返回 `None`。
    pub fn frequency(self) -> Option<Hertz> {
        let div = match self {
            TimebaseClock::PclkDiv32 => 32,
            TimebaseClock::PclkDiv16 => 16,
            TimebaseClock::PclkDiv8 => 8,
            TimebaseClock::PclkDiv4 => 4,
            TimebaseClock::PclkDiv2 => 2,
            _ => return None,
        };
        Some(Hertz(clocks().pclk.0 / div))
    }
}

/// 捕获触发边沿（CCAPMx.CAPP/CAPN，RM 11.4.5）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CaptureEdge {
    /// 上升沿（CAPP = 1，CAPN = 0）。
    Rising,
    /// 下降沿（CAPP = 0，CAPN = 1）。
    Falling,
    /// 任意沿（CAPP = CAPN = 1）。
    Both,
}

/// PCA 通道号（0–4）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Channel {
    /// 模块 0。
    C0,
    /// 模块 1。
    C1,
    /// 模块 2。
    C2,
    /// 模块 3。
    C3,
    /// 模块 4。
    C4,
}

impl Channel {
    /// 通道编号（0–4）。
    pub const fn number(self) -> u8 {
        match self {
            Channel::C0 => 0,
            Channel::C1 => 1,
            Channel::C2 => 2,
            Channel::C3 => 3,
            Channel::C4 => 4,
        }
    }
}

// ---------------------------------------------------------------------------
// 时基
// ---------------------------------------------------------------------------

/// PCA 驱动。
///
/// 时基与模块共用一个外设；通道对象由 [`Pca::channel`] 再借用取得。
pub struct Pca<'d> {
    /// 持有实例单例维持独占；寄存器访问经 PAC 全局常量。
    #[allow(dead_code)]
    peri: Peri<'d, PcaPeri>,
}

impl<'d> Pca<'d> {
    /// 接管 PCA 并配置时基时钟，计数器停止、初值 0。
    ///
    /// CNT 只能在停止时写入（RM 11.4.3），构造时写入 0。
    pub fn new(peri: Peri<'d, PcaPeri>, clock: TimebaseClock) -> Self {
        enable_peripheral(PeripheralGate::Pca, true);
        let mut this = Self { peri };
        this.stop();
        this.clear_overflow();
        // CMOD：只动 CPS 与 CFIE，保留 CIDL/WDTE 复位值 0。
        this.regs().cmod().modify(|w| w.set_cps(clock.encode()));
        this.regs().cnt().write(|w| w.set_cnt(0));
        this
    }

    /// PAC 寄存器块。
    fn regs(&self) -> pac::pca_ddl_v1::Pca {
        pac::PCA
    }

    /// 启动时基计数（CCON.CR = 1）。
    pub fn start(&mut self) {
        self.regs().ccon().modify(|w| w.set_cr(true));
    }

    /// 停止时基计数（CCON.CR = 0）。
    pub fn stop(&mut self) {
        self.regs().ccon().modify(|w| w.set_cr(false));
    }

    /// 时基是否在运行（CCON.CR）。
    pub fn is_running(&self) -> bool {
        self.regs().ccon().read().cr()
    }

    /// 读时基计数值（CNT）。
    pub fn count(&self) -> u16 {
        self.regs().cnt().read().cnt()
    }

    /// 写时基初值（仅停止时可写，RM 11.4.3；运行中调用先停表）。
    pub fn set_count(&mut self, value: u16) {
        if self.is_running() {
            self.stop();
        }
        self.regs().cnt().write(|w| w.set_cnt(value));
    }

    /// 使能时基溢出中断（CMOD.CFIE = 1）。
    pub fn enable_overflow_interrupt(&mut self) {
        self.regs().cmod().modify(|w| w.set_cfie(true));
    }

    /// 关闭时基溢出中断（CMOD.CFIE = 0）。
    pub fn disable_overflow_interrupt(&mut self) {
        self.regs().cmod().modify(|w| w.set_cfie(false));
    }

    /// 时基溢出标志（CCON.CF）是否置位。
    pub fn is_overflow_pending(&self) -> bool {
        self.regs().ccon().read().cf()
    }

    /// 清除时基溢出标志：ICLR 各位写 0 清除、写 1 无效（RM 11.4.4），
    /// 因此 CF 位写 0、其余通道位写 1 保持不动。
    pub fn clear_overflow(&mut self) {
        self.regs().iclr().write(|w| {
            w.set_cf(false);
            w.set_ccf0(true);
            w.set_ccf1(true);
            w.set_ccf2(true);
            w.set_ccf3(true);
            w.set_ccf4(true);
        });
    }

    /// 空闲模式下时基是否继续运行（CMOD.CIDL，RM 11.4.2 位 7）。
    ///
    /// CIDL = 1 时空闲模式停止计数；复位默认 0（继续运行）。
    pub fn set_idle_halt(&mut self, halt: bool) {
        self.regs().cmod().modify(|w| w.set_cidl(halt));
    }

    /// 再借用一个通道对象。同一通道只能被借用一次（由调用方保证）。
    pub fn channel(&mut self, ch: Channel) -> PcaChannel<'_> {
        PcaChannel {
            pca: self,
            ch,
            _lifetime: core::marker::PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// 通道
// ---------------------------------------------------------------------------

/// PCA 捕获/比较通道（模块 0–4）。
///
/// 由 [`Pca::channel`] 再借用取得，借用期间独占该通道的模式寄存器与数
/// 据寄存器。
pub struct PcaChannel<'a> {
    pca: &'a Pca<'a>,
    ch: Channel,
    _lifetime: core::marker::PhantomData<&'a mut ()>,
}

impl PcaChannel<'_> {
    /// 通道号。
    pub fn number(&self) -> u8 {
        self.ch.number()
    }

    fn regs(&self) -> pac::pca_ddl_v1::Pca {
        self.pca.regs()
    }

    /// 计算一组 CCAPMx 设置的原始位值（在 Ccapm0 布局上组装）。
    ///
    /// 五个 CCAPMx 字段集完全同构（RM 11.4.5），位布局一致；组装一次后
    /// 按通道构造目标寄存器写入。
    fn ccapm_bits(f: impl FnOnce(&mut pac::pca_ddl_v1::regs::Ccapm0)) -> u32 {
        let mut v = pac::pca_ddl_v1::regs::Ccapm0::default();
        f(&mut v);
        v.0
    }

    /// 写本通道的 CCAPMx（整字写入；寄存器仅位 6:0 有定义，其余位保留
    /// 复位值 0，直接写组装值即可）。
    fn write_mode(&mut self, f: impl FnOnce(&mut pac::pca_ddl_v1::regs::Ccapm0)) {
        let bits = Self::ccapm_bits(f);
        let r = self.regs();
        match self.ch {
            Channel::C0 => r.ccapm0().write_value(pac::pca_ddl_v1::regs::Ccapm0(bits)),
            Channel::C1 => r.ccapm1().write_value(pac::pca_ddl_v1::regs::Ccapm1(bits)),
            Channel::C2 => r.ccapm2().write_value(pac::pca_ddl_v1::regs::Ccapm2(bits)),
            Channel::C3 => r.ccapm3().write_value(pac::pca_ddl_v1::regs::Ccapm3(bits)),
            Channel::C4 => r.ccapm4().write_value(pac::pca_ddl_v1::regs::Ccapm4(bits)),
        }
    }

    /// 读-改-写本通道的 CCAPMx 中断使能位（CCIE），不动其他模式位。
    fn set_ccie(&self, enable: bool) {
        let r = self.regs();
        match self.ch {
            Channel::C0 => r.ccapm0().modify(|w| w.set_ccie(enable)),
            Channel::C1 => r.ccapm1().modify(|w| w.set_ccie(enable)),
            Channel::C2 => r.ccapm2().modify(|w| w.set_ccie(enable)),
            Channel::C3 => r.ccapm3().modify(|w| w.set_ccie(enable)),
            Channel::C4 => r.ccapm4().modify(|w| w.set_ccie(enable)),
        }
    }

    /// 读本通道 16 位比较/捕获寄存器（CCAPx）。
    pub fn read_capture(&self) -> u16 {
        let r = self.regs();
        match self.ch {
            Channel::C0 => r.ccap0().read().ccap0(),
            Channel::C1 => r.ccap1().read().ccap1(),
            Channel::C2 => r.ccap2().read().ccap2(),
            Channel::C3 => r.ccap3().read().ccap3(),
            Channel::C4 => r.ccap4().read().ccap4(),
        }
    }

    /// 写本通道 16 位比较值（CCAPx 单次写等价先低后高，RM 11.4.8）。
    fn write_ccap(&self, value: u16) {
        let r = self.regs();
        match self.ch {
            Channel::C0 => r.ccap0().write(|w| w.set_ccap0(value)),
            Channel::C1 => r.ccap1().write(|w| w.set_ccap1(value)),
            Channel::C2 => r.ccap2().write(|w| w.set_ccap2(value)),
            Channel::C3 => r.ccap3().write(|w| w.set_ccap3(value)),
            Channel::C4 => r.ccap4().write(|w| w.set_ccap4(value)),
        }
    }

    /// 写本通道 CCAPxH（PWM 下一周期占空比）。
    fn write_ccap_high(&self, value: u8) {
        let r = self.regs();
        match self.ch {
            Channel::C0 => r.ccap0h().write(|w| w.set_ccap0(value)),
            Channel::C1 => r.ccap1h().write(|w| w.set_ccap1(value)),
            Channel::C2 => r.ccap2h().write(|w| w.set_ccap2(value)),
            Channel::C3 => r.ccap3h().write(|w| w.set_ccap3(value)),
            Channel::C4 => r.ccap4h().write(|w| w.set_ccap4(value)),
        }
    }

    /// 读本通道 CCAPxL。
    fn read_ccap_low(&self) -> u8 {
        let r = self.regs();
        match self.ch {
            Channel::C0 => r.ccap0l().read().ccap0() as u8,
            Channel::C1 => r.ccap1l().read().ccap1() as u8,
            Channel::C2 => r.ccap2l().read().ccap2() as u8,
            Channel::C3 => r.ccap3l().read().ccap3() as u8,
            Channel::C4 => r.ccap4l().read().ccap4() as u8,
        }
    }

    // ---- 模式配置 ----

    /// 关闭本通道的全部功能（CCAPMx 清零），保留 CCON 标志。
    pub fn disable(&mut self) {
        self.write_mode(|w| {
            w.set_ccie(false);
            w.set_pwm(false);
            w.set_tog(false);
            w.set_mat(false);
            w.set_capn(false);
            w.set_capp(false);
            w.set_ecom(false);
        });
    }

    /// 配置为边沿捕获（RM 11.2.2）。
    ///
    /// `enable_interrupt` 同时置 CCAPMx.CCIE。捕获输入来源默认为对应
    /// PCA_CHx 引脚；UART RX 信号来源经 GPIO_CTRL2 `pca_capx_sel` 选择
    /// （RM 6.5.5.2），由 [`set_capture_source`] 模块函数提供。
    pub fn enable_capture(&mut self, edge: CaptureEdge, enable_interrupt: bool) {
        self.write_mode(|w| {
            w.set_ecom(false);
            w.set_capp(matches!(edge, CaptureEdge::Rising | CaptureEdge::Both));
            w.set_capn(matches!(edge, CaptureEdge::Falling | CaptureEdge::Both));
            w.set_mat(false);
            w.set_tog(false);
            w.set_pwm(false);
            w.set_ccie(enable_interrupt);
        });
    }

    /// 配置为 16 位软件定时器（ECOM + MAT，RM 11.2.3.1）。
    ///
    /// 计数值与 `compare` 匹配时置 CCFx；`enable_interrupt` 同时置 CCIE。
    pub fn enable_software_timer(&mut self, compare: u16, enable_interrupt: bool) {
        self.write_ccap(compare);
        self.write_mode(|w| {
            w.set_ecom(true);
            w.set_capp(false);
            w.set_capn(false);
            w.set_mat(true);
            w.set_tog(false);
            w.set_pwm(false);
            w.set_ccie(enable_interrupt);
        });
    }

    /// 更新软件定时器的比较值（RM 11.2.3.1：中断中重写 CCAPx）。
    pub fn set_compare(&mut self, compare: u16) {
        self.write_ccap(compare);
    }

    /// 配置为高速输出（ECOM + MAT + TOG，RM 11.2.3.2）。
    ///
    /// 匹配时引脚电平翻转；`initial_high` 写 CCAPOx 选择翻转起始电平。
    /// `enable_interrupt` 同时置 CCIE。
    pub fn enable_high_speed_output(
        &mut self,
        compare: u16,
        initial_high: bool,
        enable_interrupt: bool,
    ) {
        self.write_ccap(compare);
        // CCAPO 控制匹配翻转的起始电平（RM 11.4.9）。
        let r = self.regs();
        match self.ch {
            Channel::C0 => r.ccapo().modify(|w| w.set_ccapo0(initial_high)),
            Channel::C1 => r.ccapo().modify(|w| w.set_ccapo1(initial_high)),
            Channel::C2 => r.ccapo().modify(|w| w.set_ccapo2(initial_high)),
            Channel::C3 => r.ccapo().modify(|w| w.set_ccapo3(initial_high)),
            Channel::C4 => r.ccapo().modify(|w| w.set_ccapo4(initial_high)),
        }
        self.write_mode(|w| {
            w.set_ecom(true);
            w.set_capp(false);
            w.set_capn(false);
            w.set_mat(true);
            w.set_tog(true);
            w.set_pwm(false);
            w.set_ccie(enable_interrupt);
        });
    }

    /// 更新高速输出的匹配值。
    pub fn set_high_speed_compare(&mut self, compare: u16) {
        self.write_ccap(compare);
    }

    /// 配置为 8 位 PWM（ECOM + PWM，RM 11.2.3.4）。
    ///
    /// **注意 `duty` 数值与高电平占比反向**：CNTL < `duty` 时输出低、
    /// CNTL ≥ `duty` 时输出高（RM 11.4.7），即 `duty` = 0 输出恒高、
    /// `duty` = 255 时高电平仅约 1/256（RM 11.2.3.4：0~255 对应高电平
    /// 比例 100%~0.4%）。PWM 输出频率 = 时基频率 / 256（CNTL 8 位溢出
    /// 为一个周期，RM 11.2.3.4）。
    ///
    /// `duty` 写入 CCAPxL（当前周期生效）；随后调用 [`set_duty`] 改变
    /// 占空比写的是 CCAPxH，由硬件在 CNTL 溢出时装载，避免毛刺。
    pub fn enable_pwm(&mut self, duty: u8) {
        // 先写 L（硬件自动清 ECOM）再写 H（硬件自动置 ECOM），RM 11.4.6/11.4.7；
        // 最后打开 PWM 功能位。
        let r = self.regs();
        match self.ch {
            Channel::C0 => {
                r.ccap0l().write(|w| w.set_ccap0(duty));
                r.ccap0h().write(|w| w.set_ccap0(duty));
            }
            Channel::C1 => {
                r.ccap1l().write(|w| w.set_ccap1(duty));
                r.ccap1h().write(|w| w.set_ccap1(duty));
            }
            Channel::C2 => {
                r.ccap2l().write(|w| w.set_ccap2(duty));
                r.ccap2h().write(|w| w.set_ccap2(duty));
            }
            Channel::C3 => {
                r.ccap3l().write(|w| w.set_ccap3(duty));
                r.ccap3h().write(|w| w.set_ccap3(duty));
            }
            Channel::C4 => {
                r.ccap4l().write(|w| w.set_ccap4(duty));
                r.ccap4h().write(|w| w.set_ccap4(duty));
            }
        }
        self.write_mode(|w| {
            w.set_ecom(true);
            w.set_capp(false);
            w.set_capn(false);
            w.set_mat(false);
            w.set_tog(false);
            w.set_pwm(true);
            w.set_ccie(false);
        });
    }

    /// 更新 PWM 占空比（写 CCAPxH，下个 CNTL 溢出周期生效，RM 11.2.3.4）。
    ///
    /// 数值与高电平占比反向，见 [`enable_pwm`] 的说明。
    pub fn set_duty(&mut self, duty: u8) {
        self.write_ccap_high(duty);
    }

    /// 当前 PWM 占空比（CCAPxL）。
    pub fn duty(&self) -> u8 {
        self.read_ccap_low()
    }

    // ---- 标志 ----

    /// 本通道比较/捕获标志（CCON.CCFx）是否置位。
    pub fn is_pending(&self) -> bool {
        let r = self.regs();
        match self.ch {
            Channel::C0 => r.ccon().read().ccf0(),
            Channel::C1 => r.ccon().read().ccf1(),
            Channel::C2 => r.ccon().read().ccf2(),
            Channel::C3 => r.ccon().read().ccf3(),
            Channel::C4 => r.ccon().read().ccf4(),
        }
    }

    /// 清除本通道标志：ICLR 各位写 0 清除、写 1 无效（RM 11.4.4），目标
    /// 位写 0、其余位写 1 保持不动。
    pub fn clear(&mut self) {
        let r = self.regs();
        match self.ch {
            Channel::C0 => r.iclr().write(|w| {
                w.set_ccf0(false);
                w.set_ccf1(true);
                w.set_ccf2(true);
                w.set_ccf3(true);
                w.set_ccf4(true);
                w.set_cf(true);
            }),
            Channel::C1 => r.iclr().write(|w| {
                w.set_ccf0(true);
                w.set_ccf1(false);
                w.set_ccf2(true);
                w.set_ccf3(true);
                w.set_ccf4(true);
                w.set_cf(true);
            }),
            Channel::C2 => r.iclr().write(|w| {
                w.set_ccf0(true);
                w.set_ccf1(true);
                w.set_ccf2(false);
                w.set_ccf3(true);
                w.set_ccf4(true);
                w.set_cf(true);
            }),
            Channel::C3 => r.iclr().write(|w| {
                w.set_ccf0(true);
                w.set_ccf1(true);
                w.set_ccf2(true);
                w.set_ccf3(false);
                w.set_ccf4(true);
                w.set_cf(true);
            }),
            Channel::C4 => r.iclr().write(|w| {
                w.set_ccf0(true);
                w.set_ccf1(true);
                w.set_ccf2(true);
                w.set_ccf3(true);
                w.set_ccf4(false);
                w.set_cf(true);
            }),
        }
    }

    /// 使能/关闭本通道中断（CCAPMx.CCIE）。
    ///
    /// 读-改-写只动 CCIE 位，保留其他模式位（ECOM/MAT/TOG/PWM/CAPP/
    /// CAPN）不变。
    pub fn set_interrupt(&mut self, enable: bool) {
        self.set_ccie(enable);
    }
}

/// 配置 PCA 捕获通道的输入来源（GPIO_CTRL2 `pca_capx_sel`，RM 6.5.5.2）。
///
/// 编码：0 = PCA_CHx 引脚、1 = UART0_RXD、2 = UART1_RXD、3 = LPUART_RXD。
/// 通道 0/1 还可选 VC1/VC2 输出（在 VC 模块配置，不在本函数范围）。
pub fn set_capture_source(ch: Channel, uart_rxd: Option<UartRx>) {
    let code = match uart_rxd {
        None => 0,
        Some(UartRx::Uart0) => 1,
        Some(UartRx::Uart1) => 2,
        Some(UartRx::Lpuart) => 3,
    };
    pac::GPIO.ctrl2().modify(|w| match ch {
        Channel::C0 => w.set_pca_cap0_sel(code),
        Channel::C1 => w.set_pca_cap1_sel(code),
        Channel::C2 => w.set_pca_cap2_sel(code),
        Channel::C3 => w.set_pca_cap3_sel(code),
        Channel::C4 => w.set_pca_cap4_sel(code),
    });
}

/// 可选作 PCA 捕获/GATE 输入的 UART RX 信号。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UartRx {
    /// UART0 RXD。
    Uart0,
    /// UART1 RXD。
    Uart1,
    /// LPUART RXD。
    Lpuart,
}

/// PCA 实例单例。
#[derive(Copy, Clone)]
pub struct PcaPeri {
    _private: (),
}

impl crate::peripheral::PeripheralType for PcaPeri {}

impl PcaPeri {
    /// 直接取得该实例，绕过 [`crate::init`] 的独占检查。
    ///
    /// # Safety
    ///
    /// 调用方须保证同一实例不被两处同时持有。
    pub unsafe fn steal() -> crate::peripheral::Peri<'static, Self> {
        unsafe { crate::peripheral::Peri::new_unchecked(Self { _private: () }) }
    }
}
