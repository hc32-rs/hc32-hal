//! 高级定时器 TIM4/5/6（HC32L110，ADT `ddl_v1`）。
//!
//! 三个功能相同的 16 位高性能计数器（RM 第 12 章）。核心能力：
//!
//! - 波形模式（GCONR.MODE）：锯齿波 A、三角波 A（谷点缓存传送）、三角波
//!   B（峰谷两点缓存传送）；
//! - 计数时钟（GCONR.CKDIV）：PCLK 的 1/2/4/8/16/64/256/1024 分频；或由
//!   HCUPR/HCDOR 选择端口/AOS 事件作为硬件计数时钟；
//! - 比较输出（GCMAR/GCMBR + PCONR）：计数值匹配时 CHxA/CHxB 输出指定
//!   电平，支持独立或互补 PWM；
//! - 捕获输入（HCPAR/HCPBR + CAPCA/CAPCB）：捕获条件有效时计数值存入
//!   GCMAR/GCMBR，并可反向缓存到 GCMCR/GCMDR（BCONR.BENA/BENB）；
//! - 缓存（BCONR）：PERBR→PERAR、GCMCR→GCMAR 等在溢出/谷点自动传送，
//!   运行中改周期/占空比不产生毛刺；
//! - 软件同步（SSTAR/SSTPR/SCLRR，三通道共用寄存器，写 1 触发）；
//! - 刹车保护（AOSSR/PTBKS/PTBKP + PCONR.DISVALA/DISSELA）：端口/同高同
//!   低/低功耗/VC 四类无效条件强制输出到指定状态。
//!
//! 三通道共用寄存器（AOSSR/AOSCL/PTBKS/TTRIG/ITRIG/PTBKP 与 SSTAR/SSTPR/
//! SCLRR）：任一实例写入对所有通道生效（RM 12.3.27 等注明）。
//!
//! 中断（IFR/ICLR/ICONR）：比较匹配 A–D、上溢、下溢、死区错误等，标志
//! 写 ICLR 对应位 = 0 清除（写 1 无效，RM 12.3.25）。

use crate::clock::{PeripheralGate, clocks, enable_peripheral};
use crate::pac;
use crate::peripheral::Peri;
use crate::time::Hertz;

/// 计数时钟分频（GCONR.CKDIV，RM 12.3.6 位 6:4）。
///
/// 判别值即寄存器编码。
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
    /// PCLK / 64。
    Div64 = 5,
    /// PCLK / 256。
    Div256 = 6,
    /// PCLK / 1024。
    Div1024 = 7,
}

impl crate::time::Prescaler for Prescaler {
    fn num(&self) -> u32 {
        1
    }

    fn denom(&self) -> u32 {
        match *self as u32 {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 8,
            4 => 16,
            5 => 64,
            6 => 256,
            _ => 1024,
        }
    }
}

/// 波形模式（GCONR.MODE，RM 12.3.6 位 3:1）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum WaveMode {
    /// 锯齿波 A 模式（编码 000，复位默认）。
    #[default]
    SawtoothA,
    /// 三角波 A 模式（编码 100）：一个周期一次缓存传送（谷点）。
    TriangleA,
    /// 三角波 B 模式（编码 101）：峰谷两点各一次缓存传送。
    TriangleB,
}

impl WaveMode {
    const fn encode(self) -> u8 {
        match self {
            WaveMode::SawtoothA => 0,
            WaveMode::TriangleA => 4,
            WaveMode::TriangleB => 5,
        }
    }
}

/// 计数方向（GCONR.DIR，RM 12.3.6 位 8）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Direction {
    /// 递减计数（编码 0）。
    #[default]
    Down,
    /// 递加计数（编码 1）。
    Up,
}

/// 匹配时端口电平动作（PCONR.CMPCA/CMPCB/PERCA/PERCB，RM 12.3.8）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PortAction {
    /// 保持低电平（编码 00）。
    KeepLow,
    /// 置高电平（编码 01）。
    SetHigh,
    /// 保持先前状态（编码 10）。
    Keep,
    /// 反转电平（编码 11）。
    Toggle,
}

impl PortAction {
    const fn encode(self) -> u8 {
        match self {
            PortAction::KeepLow => 0,
            PortAction::SetHigh => 1,
            PortAction::Keep => 2,
            PortAction::Toggle => 3,
        }
    }
}

/// 刹车时端口输出状态（PCONR.DISVALA/DISVALB，RM 12.3.8）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum BrakeOutput {
    /// 正常输出（编码 00，复位默认）。
    #[default]
    Normal,
    /// 高阻态（编码 01）。
    HighZ,
    /// 低电平（编码 10）。
    Low,
    /// 高电平（编码 11）。
    High,
}

impl BrakeOutput {
    const fn encode(self) -> u8 {
        match self {
            BrakeOutput::Normal => 0,
            BrakeOutput::HighZ => 1,
            BrakeOutput::Low => 2,
            BrakeOutput::High => 3,
        }
    }
}

/// 通道选择（CHxA / CHxB）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Ch {
    /// A 通道（GCMAR/CHxA）。
    A,
    /// B 通道（GCMBR/CHxB）。
    B,
}

// ---------------------------------------------------------------------------
// 频率换算
// ---------------------------------------------------------------------------

/// 按目标 PWM 周期选预分频与周期值（PERAR）。
///
/// 锯齿波递加计数：计数值 0 → PERAR 后溢出，周期为 `(PERAR + 1) *
/// CKDIV / PCLK`。三角波按峰谷合计 2×(PERAR + 1) 节拍换算；手册正文未
/// 给出显式周期公式，该系数来自图 12-3 波形（**待上板验证**，差异台账
/// 已登记）。找不到满足 16 位范围的档位时返回 `None`。
pub fn period_to_perar(period: Hertz, mode: WaveMode) -> Option<(Prescaler, u16)> {
    // 三角波一个周期 = 2 * (PERAR + 1) 个计数节拍。
    let scale = match mode {
        WaveMode::SawtoothA => 1,
        WaveMode::TriangleA | WaveMode::TriangleB => 2,
    };
    let ticks = clocks().pclk.0 / period.0;
    for div in [
        Prescaler::Div1,
        Prescaler::Div2,
        Prescaler::Div4,
        Prescaler::Div8,
        Prescaler::Div16,
        Prescaler::Div64,
        Prescaler::Div256,
        Prescaler::Div1024,
    ] {
        let denom = crate::time::Prescaler::denom(&div);
        let counts = ticks / (denom * scale);
        if counts == 0 || counts > 0x1_0000 {
            continue;
        }
        // PERAR = counts - 1；counts ≥ 1 保证不下溢。
        return Some((div, (counts - 1) as u16));
    }
    None
}

/// 周期值换算回频率（供校验/显示）。
pub fn perar_to_period(pre: Prescaler, perar: u16, mode: WaveMode) -> Hertz {
    let scale = match mode {
        WaveMode::SawtoothA => 1,
        WaveMode::TriangleA | WaveMode::TriangleB => 2,
    };
    let denom = crate::time::Prescaler::denom(&pre);
    Hertz(clocks().pclk.0 / ((perar as u32 + 1) * denom * scale))
}

// ---------------------------------------------------------------------------
// 驱动
// ---------------------------------------------------------------------------

/// 高级定时器驱动（TIM4/5/6）。
pub struct Adt<'d, T: Instance> {
    /// 持有实例单例维持独占；寄存器访问经 PAC 全局常量。
    #[allow(dead_code)]
    peri: Peri<'d, T>,
}

impl<'d, T: Instance> Adt<'d, T> {
    /// 接管实例，配置波形模式与预分频（递加计数），不启动。
    ///
    /// 周期值（PERAR/PERBR）与比较值保持复位值 0xFFFF，按需用
    /// [`set_period`] / [`enable_output`] 配置。锯齿波模式下方向运行中可
    /// 改（RM 12.2.3.1）；三角波模式方向只能停止时设定（RM 12.2.3.2）。
    pub fn new(peri: Peri<'d, T>, mode: WaveMode, pre: Prescaler) -> Self {
        enable_peripheral(PeripheralGate::AdvTim, true);
        let mut this = Self { peri };
        this.stop();
        this.write_gconr(|w| {
            w.set_mode(mode.encode());
            w.set_ckdiv(pre as u8);
            // 递加计数（GCONR.DIR = 1）。
            w.set_dir(true);
        });
        this
    }

    /// 接管实例并按目标周期配置为 PWM 时基（锯齿波递加），不启动。
    ///
    /// 自动选择预分频；同时使能周期缓存（PERBR→PERAR，BCONR.BENP），
    /// 运行中改周期无毛刺。
    ///
    /// # Panics
    ///
    /// 周期无法以 16 位 PERAR 表示时 panic。
    pub fn new_pwm(peri: Peri<'d, T>, period: Hertz) -> Self {
        let (pre, perar) = period_to_perar(period, WaveMode::SawtoothA)
            .unwrap_or_else(|| panic!("adt: 周期 {} Hz 超出 16 位范围", period.0));
        let mut this = Self::new(peri, WaveMode::SawtoothA, pre);
        this.set_period(perar);
        this
    }
    /// PAC 寄存器块。
    fn regs(&self) -> pac::adt_ddl_v1::Adt {
        T::regs()
    }

    fn write_gconr(&self, f: impl FnOnce(&mut pac::adt_ddl_v1::regs::Gconr)) {
        self.regs().gconr().modify(f);
    }

    /// 启动计数（GCONR.START = 1）。
    pub fn start(&mut self) {
        self.write_gconr(|w| w.set_start(true));
    }

    /// 停止计数（GCONR.START = 0）。
    pub fn stop(&mut self) {
        self.write_gconr(|w| w.set_start(false));
    }

    /// 是否在计数（GCONR.START）。
    pub fn is_running(&self) -> bool {
        self.regs().gconr().read().start()
    }

    /// 读当前计数值（CNTER）。
    pub fn count(&self) -> u16 {
        self.regs().cnter().read().cnt()
    }

    /// 写计数初值（CNTER）。
    pub fn set_count(&mut self, value: u16) {
        self.regs().cnter().write(|w| w.set_cnt(value));
    }

    /// 周期值（PERAR）。
    pub fn period(&self) -> u16 {
        self.regs().perar().read().pera()
    }

    /// 配置周期值并使能缓存传送（BCONR.BENP，RM 12.3.9 位 8）。
    ///
    /// 写 PERBR 后由硬件在溢出/谷点装载到 PERAR。
    pub fn set_period(&mut self, perar: u16) {
        let r = self.regs();
        r.perbr().write(|w| w.set_perb(perar));
        r.bconr().modify(|w| w.set_benp(true));
        // 首个周期：直接写 PERAR，保证启动即按新周期计数。
        r.perar().write(|w| w.set_pera(perar));
    }

    /// 当前计数方向（STFLR.DIRF）。
    pub fn direction(&self) -> Direction {
        if self.regs().stflr().read().dirf() {
            Direction::Up
        } else {
            Direction::Down
        }
    }

    // ---- 比较输出 / 捕获输入通道 ----

    /// 读比较基准值（GCMAR/GCMBR）。
    pub fn compare(&self, ch: Ch) -> u16 {
        let r = self.regs();
        match ch {
            Ch::A => r.gcmar().read().gcma(),
            Ch::B => r.gcmbr().read().gcmb(),
        }
    }

    /// 使能比较输出：直写比较值（GCMAR/GCMBR）并同步缓存（GCMCR/GCMDR）
    /// 打开 OUTEN 与缓存传送（BCONR.BENA/BENB）。
    ///
    /// 直写保证停止状态下启动即按设定值比较；后续改值用 [`set_compare`]
    /// 只写缓存，由硬件在传送点装载，运行中无毛刺。
    /// 锯齿波 A 模式下典型的 PWM 输出为 `cmp = SetHigh`、`per = KeepLow`
    /// （本函数固定周期点动作与起停电平为低，匹配点按 `cmp_action`）。
    /// 端口引脚复用编码见 RM 表 6-2（如 P26 = TIM4_CHA 编码 2），由调用
    /// 方经 GPIO 配置。
    pub fn enable_output(&mut self, ch: Ch, compare: u16, cmp_action: PortAction) {
        let r = self.regs();
        match ch {
            Ch::A => {
                r.gcmar().write(|w| w.set_gcma(compare));
                r.gcmcr().write(|w| w.set_gcmc(compare));
                r.bconr().modify(|w| w.set_bena(true));
                r.pconr().modify(|w| {
                    w.set_outena(true);
                    w.set_capca(false);
                    w.set_cmpca(cmp_action.encode());
                    // 周期点保持低：与 cmp = SetHigh 组成标准 PWM。
                    w.set_perca(PortAction::KeepLow.encode());
                    // 计数开始/停止端口输出低（STASTPSA = 0 时 STAC/STPC
                    // 生效，RM 12.3.8）。
                    w.set_stastpsa(false);
                    w.set_staca(false);
                    w.set_stpca(false);
                });
            }
            Ch::B => {
                r.gcmbr().write(|w| w.set_gcmb(compare));
                r.gcmdr().write(|w| w.set_gcmd(compare));
                r.bconr().modify(|w| w.set_benb(true));
                r.pconr().modify(|w| {
                    w.set_outenb(true);
                    w.set_capcb(false);
                    w.set_cmpcb(cmp_action.encode());
                    w.set_percb(PortAction::KeepLow.encode());
                    w.set_stastpsb(false);
                    w.set_stacb(false);
                    w.set_stpcb(false);
                });
            }
        }
    }

    /// 更新比较值（写缓存寄存器 GCMCR/GCMDR，BCONR 使能时由硬件在传送点
    /// 装载到 GCMAR/GCMBR）。
    pub fn set_compare(&mut self, ch: Ch, compare: u16) {
        let r = self.regs();
        match ch {
            Ch::A => r.gcmcr().write(|w| w.set_gcmc(compare)),
            Ch::B => r.gcmdr().write(|w| w.set_gcmd(compare)),
        }
    }

    /// 读捕获值（捕获输入功能时 GCMAR/GCMBR 保存捕获计数值，RM 12.2.1.3）。
    ///
    /// 下一次捕获事件会覆盖该值；使能了 BENA/BENB 缓存时，上次捕获值同
    /// 时镜像在 GCMCR/GCMDR，可用 [`capture_value_cached`] 在新捕获后
    /// 读取旧值。
    pub fn capture_value(&self, ch: Ch) -> u16 {
        let r = self.regs();
        match ch {
            Ch::A => r.gcmar().read().gcma(),
            Ch::B => r.gcmbr().read().gcmb(),
        }
    }

    /// 读缓存侧捕获值（GCMCR/GCMDR，捕获动作时由 GCMAR/GCMBR 传入，
    /// RM 12.3.9 BENA/BENB 字段）。
    pub fn capture_value_cached(&self, ch: Ch) -> u16 {
        let r = self.regs();
        match ch {
            Ch::A => r.gcmcr().read().gcmc(),
            Ch::B => r.gcmdr().read().gcmd(),
        }
    }

    /// 使能捕获输入（PCONR.CAPCA/CAPCB = 1），配置硬件捕获条件
    /// （HCPAR/HCPBR 的通道边沿位）并使能捕获缓存反向传送（BCONR.BENA/
    /// BENB：捕获时 GCMAR/GCMBR→GCMCR/GCMDR）。
    ///
    /// `source` 选择捕获输入端口信号（CHxA/CHxB/TIMTRIA–D 引脚，RM
    /// 12.3.17）。通道信号来源（引脚或 UART RX）经 GPIO_CTRL3 的
    /// `tmx_a_sel/tmx_b_sel` 选择，用 [`set_channel_source`] 模块函数配置。
    pub fn enable_capture(
        &mut self,
        ch: Ch,
        source: CapturePort,
        edge: CaptureEdge,
        enable_interrupt: bool,
    ) {
        let r = self.regs();
        let (set_bit, clear_bit) = source.encode(edge);
        match ch {
            Ch::A => {
                r.bconr().modify(|w| w.set_bena(true));
                r.pconr().modify(|w| {
                    w.set_capca(true);
                    w.set_outena(false);
                });
                // HCPAR 选中边沿位置 1、配对边沿位清 0（切换边沿不留残留）。
                r.hcpar().modify(|w| set_hcpa_bits(w, set_bit, clear_bit));
                if enable_interrupt {
                    r.iconr().modify(|w| w.set_intena(true));
                }
            }
            Ch::B => {
                r.bconr().modify(|w| w.set_benb(true));
                r.pconr().modify(|w| {
                    w.set_capcb(true);
                    w.set_outenb(false);
                });
                r.hcpbr().modify(|w| set_hcpb_bits(w, set_bit, clear_bit));
                if enable_interrupt {
                    r.iconr().modify(|w| w.set_intenb(true));
                }
            }
        }
    }

    // ---- 中断 ----

    /// 使能比较匹配中断（ICONR.INTENA–D）。
    pub fn enable_compare_interrupt(&mut self, ch: Ch) {
        self.regs().iconr().modify(|w| match ch {
            Ch::A => w.set_intena(true),
            Ch::B => w.set_intenb(true),
        });
    }

    /// 使能周期匹配中断：锯齿波递加上溢（INTENOVF）/下溢（INTENUDF），
    /// 三角波峰/谷（RM 12.2.12.2）。
    pub fn enable_period_interrupt(&mut self, up: bool, down: bool) {
        self.regs().iconr().modify(|w| {
            w.set_intenovf(up);
            w.set_intenudf(down);
        });
    }

    /// 比较匹配标志（IFR.CMAF/CMBF）是否置位。
    pub fn is_compare_pending(&self, ch: Ch) -> bool {
        let r = self.regs();
        match ch {
            Ch::A => r.ifr().read().cmaf(),
            Ch::B => r.ifr().read().cmbf(),
        }
    }

    /// 上溢/峰点标志（IFR.OVFF）是否置位。
    pub fn is_overflow_pending(&self) -> bool {
        self.regs().ifr().read().ovff()
    }

    /// 下溢/谷点标志（IFR.UDFF）是否置位。
    pub fn is_underflow_pending(&self) -> bool {
        self.regs().ifr().read().udff()
    }

    /// 清除比较匹配标志：ICLR 各位写 0 清除、写 1 无效（RM 12.3.25），
    /// 目标位写 0、其余有效位写 1 保持不动。
    pub fn clear_compare(&mut self, ch: Ch) {
        self.regs().iclr().write(|w| {
            w.set_cmac(matches!(ch, Ch::B));
            w.set_cmbc(matches!(ch, Ch::A));
            w.set_cmcc(true);
            w.set_cmdc(true);
            w.set_ovfc(true);
            w.set_udfc(true);
            w.set_dtec(true);
            w.set_samlc(true);
            w.set_samhc(true);
        });
    }

    /// 清除周期标志：`up` 清上溢/峰点（OVFC），`down` 清下溢/谷点（UDFC）。
    /// 未选中的位写 1 保持不动（ICLR 写 0 清除、写 1 无效，RM 12.3.25）。
    pub fn clear_period(&mut self, up: bool, down: bool) {
        self.regs().iclr().write(|w| {
            w.set_ovfc(!up);
            w.set_udfc(!down);
            w.set_cmac(true);
            w.set_cmbc(true);
            w.set_cmcc(true);
            w.set_cmdc(true);
            w.set_dtec(true);
            w.set_samlc(true);
            w.set_samhc(true);
        });
    }

    // ---- 刹车 ----

    /// 配置刹车时端口输出状态（PCONR.DISVALA/DISVALB）与无效条件选择
    /// （DISSELA/DISSELB，RM 12.3.8）。
    pub fn set_brake_output(&mut self, ch: Ch, cond: BrakeCondition, output: BrakeOutput) {
        let r = self.regs();
        match ch {
            Ch::A => r.pconr().modify(|w| {
                w.set_dissela(cond as u8);
                w.set_disvala(output.encode());
            }),
            Ch::B => r.pconr().modify(|w| {
                w.set_disselb(cond as u8);
                w.set_disvalb(output.encode());
            }),
        }
    }
}

/// 刹车无效条件选择（PCONR.DISSELA/DISSELB，RM 12.3.8；条件来源见
/// RM 12.2.13.1~13.4）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum BrakeCondition {
    /// 无效条件 0：VC1/VC2 中断标志（RM 12.2.13.4）。
    #[default]
    Vc,
    /// 无效条件 1：输出同高同低刹车（RM 12.2.13.3）。
    SameLevel,
    /// 无效条件 2：低功耗模式自动刹车（RM 12.2.13.2）。
    LowPower,
    /// 无效条件 3：端口刹车（RM 12.2.13.1）。
    Port,
}

// ---------------------------------------------------------------------------
// 捕获条件编码（HCPAR/HCPBR，RM 12.3.17/12.3.18）
// ---------------------------------------------------------------------------

/// 捕获输入端口信号。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CapturePort {
    /// 本定时器 CHxA 引脚。
    ChxA,
    /// 本定时器 CHxB 引脚。
    ChxB,
    /// TIMTRIA 引脚（三通道共用）。
    TrigA,
    /// TIMTRIB 引脚。
    TrigB,
    /// TIMTRIC 引脚。
    TrigC,
    /// TIMTRID 引脚。
    TrigD,
}

/// 捕获边沿。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CaptureEdge {
    /// 上升沿。
    Rising,
    /// 下降沿。
    Falling,
}

impl CapturePort {
    /// HCPAR/HCPBR 位编码：4/6 = CHxA/CHxB 上升沿，5/7 = 下降沿；8/10/12/14
    /// = TRIA–D 上升沿，9/11/13/15 = 下降沿。返回 `(置位位, 清零位)`：
    /// 选中边沿位置 1，配对的另一边沿位清 0（重复配置时切换边沿不留
    /// 残留，RM 12.3.17/12.3.18 各位独立 R/W）。
    fn encode(self, edge: CaptureEdge) -> (u8, u8) {
        let (rising_bit, falling_bit) = match self {
            CapturePort::ChxA => (4, 5),
            CapturePort::ChxB => (6, 7),
            CapturePort::TrigA => (8, 9),
            CapturePort::TrigB => (10, 11),
            CapturePort::TrigC => (12, 13),
            CapturePort::TrigD => (14, 15),
        };
        match edge {
            CaptureEdge::Rising => (rising_bit, falling_bit),
            CaptureEdge::Falling => (falling_bit, rising_bit),
        }
    }
}

// ---------------------------------------------------------------------------
// 通道信号来源（GPIO_CTRL3，RM 6.5.5.3）
// ---------------------------------------------------------------------------

/// CHxA/CHxB 端口信号来源（GPIO_CTRL3 `tmx_a_sel/tmx_b_sel`）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum ChannelSource {
    /// 对应 TIMx_CHA/CHB 引脚（编码 0，复位默认）。
    #[default]
    Pin,
    /// UART0 RXD（编码 1）。
    Uart0Rxd,
    /// UART1 RXD（编码 2）。
    Uart1Rxd,
    /// LPUART RXD（编码 3）。
    LpuartRxd,
}

impl ChannelSource {
    const fn encode(self) -> u8 {
        match self {
            ChannelSource::Pin => 0,
            ChannelSource::Uart0Rxd => 1,
            ChannelSource::Uart1Rxd => 2,
            ChannelSource::LpuartRxd => 3,
        }
    }
}

/// 配置 ADT 通道端口信号来源（GPIO_CTRL3，RM 6.5.5.3）。
///
/// `timer` 为定时器编号（4/5/6 对应 ADT0/1/2），其他值 panic。
pub fn set_channel_source(timer: u8, ch: Ch, source: ChannelSource) {
    assert!(
        (4..=6).contains(&timer),
        "adt: set_channel_source 的 timer 须为 4/5/6"
    );
    pac::GPIO.ctrl3().modify(|w| {
        let code = source.encode();
        match (timer, ch) {
            (4, Ch::A) => w.set_tm4_a_sel(code),
            (4, Ch::B) => w.set_tm4_b_sel(code),
            (5, Ch::A) => w.set_tm5_a_sel(code),
            (5, Ch::B) => w.set_tm5_b_sel(code),
            (6, Ch::A) => w.set_tm6_a_sel(code),
            _ => w.set_tm6_b_sel(code),
        }
    });
}

// ---------------------------------------------------------------------------
// 软件同步（三通道共用寄存器，RM 12.2.5）
// ---------------------------------------------------------------------------

/// 软件同步启动 TIM4/5/6（SSTAR，写 1 触发，三通道共用）。
///
/// `tim4/tim5/tim6` 选择要同步启动的通道。
pub fn sync_start(tim4: bool, tim5: bool, tim6: bool) {
    pac::ADT0.sstar().write(|w| {
        w.set_ssta0(tim4);
        w.set_ssta1(tim5);
        w.set_ssta2(tim6);
    });
}

/// 软件同步停止 TIM4/5/6（SSTPR，三通道共用）。
pub fn sync_stop(tim4: bool, tim5: bool, tim6: bool) {
    pac::ADT0.sstpr().write(|w| {
        w.set_sstp0(tim4);
        w.set_sstp1(tim5);
        w.set_sstp2(tim6);
    });
}

/// 软件同步清零 TIM4/5/6 计数器（SCLRR，三通道共用）。
pub fn sync_clear(tim4: bool, tim5: bool, tim6: bool) {
    pac::ADT0.sclrr().write(|w| {
        w.set_sclr0(tim4);
        w.set_sclr1(tim5);
        w.set_sclr2(tim6);
    });
}

// ---------------------------------------------------------------------------
// 实例单例
// ---------------------------------------------------------------------------

/// HCPAR 按位号写两个位：`set_bit` 位置 1、`clear_bit` 位清 0（0–15）。
fn set_hcpa_bits(w: &mut pac::adt_ddl_v1::regs::Hcpar, set_bit: u8, clear_bit: u8) {
    match set_bit {
        0 => w.set_hcpa0(true),
        1 => w.set_hcpa1(true),
        2 => w.set_hcpa2(true),
        3 => w.set_hcpa3(true),
        4 => w.set_hcpa4(true),
        5 => w.set_hcpa5(true),
        6 => w.set_hcpa6(true),
        7 => w.set_hcpa7(true),
        8 => w.set_hcpa8(true),
        9 => w.set_hcpa9(true),
        10 => w.set_hcpa10(true),
        11 => w.set_hcpa11(true),
        12 => w.set_hcpa12(true),
        13 => w.set_hcpa13(true),
        14 => w.set_hcpa14(true),
        _ => w.set_hcpa15(true),
    }
    match clear_bit {
        0 => w.set_hcpa0(false),
        1 => w.set_hcpa1(false),
        2 => w.set_hcpa2(false),
        3 => w.set_hcpa3(false),
        4 => w.set_hcpa4(false),
        5 => w.set_hcpa5(false),
        6 => w.set_hcpa6(false),
        7 => w.set_hcpa7(false),
        8 => w.set_hcpa8(false),
        9 => w.set_hcpa9(false),
        10 => w.set_hcpa10(false),
        11 => w.set_hcpa11(false),
        12 => w.set_hcpa12(false),
        13 => w.set_hcpa13(false),
        14 => w.set_hcpa14(false),
        _ => w.set_hcpa15(false),
    }
}

/// HCPBR 按位号写两个位：`set_bit` 位置 1、`clear_bit` 位清 0（0–15）。
fn set_hcpb_bits(w: &mut pac::adt_ddl_v1::regs::Hcpbr, set_bit: u8, clear_bit: u8) {
    match set_bit {
        0 => w.set_hcpb0(true),
        1 => w.set_hcpb1(true),
        2 => w.set_hcpb2(true),
        3 => w.set_hcpb3(true),
        4 => w.set_hcpb4(true),
        5 => w.set_hcpb5(true),
        6 => w.set_hcpb6(true),
        7 => w.set_hcpb7(true),
        8 => w.set_hcpb8(true),
        9 => w.set_hcpb9(true),
        10 => w.set_hcpb10(true),
        11 => w.set_hcpb11(true),
        12 => w.set_hcpb12(true),
        13 => w.set_hcpb13(true),
        14 => w.set_hcpb14(true),
        _ => w.set_hcpb15(true),
    }
    match clear_bit {
        0 => w.set_hcpb0(false),
        1 => w.set_hcpb1(false),
        2 => w.set_hcpb2(false),
        3 => w.set_hcpb3(false),
        4 => w.set_hcpb4(false),
        5 => w.set_hcpb5(false),
        6 => w.set_hcpb6(false),
        7 => w.set_hcpb7(false),
        8 => w.set_hcpb8(false),
        9 => w.set_hcpb9(false),
        10 => w.set_hcpb10(false),
        11 => w.set_hcpb11(false),
        12 => w.set_hcpb12(false),
        13 => w.set_hcpb13(false),
        14 => w.set_hcpb14(false),
        _ => w.set_hcpb15(false),
    }
}

/// 高级定时器实例能力。
pub trait Instance: crate::peripheral::PeripheralType + 'static {
    /// 取 PAC 寄存器块。
    fn regs() -> pac::adt_ddl_v1::Adt;

    /// 定时器编号（4/5/6）。
    fn number() -> u8;
}

macro_rules! adt_instance {
    ($name:ident, $pac_reg:ident, $num:expr, $doc:expr) => {
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
            fn regs() -> pac::adt_ddl_v1::Adt {
                pac::$pac_reg
            }

            fn number() -> u8 {
                $num
            }
        }
    };
}

adt_instance!(Adt0, ADT0, 4, "TIM4 实例单例。");
adt_instance!(Adt1, ADT1, 5, "TIM5 实例单例。");
adt_instance!(Adt2, ADT2, 6, "TIM6 实例单例。");
