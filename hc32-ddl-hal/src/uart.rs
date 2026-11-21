//! UART0/1 与 LPUART（HC32L110，UART/LPUART `ddl_v1`）。
//!
//! UART0/1 是 8051 风格的可编程串口（RM 第 15 章），寄存器组
//! SBUF/SCON/SADDR/SADEN/ISR/ICR。波特率**不是**由 UART 自身的分频器
//! 产生：Mode1/3 由基本定时器的 TOG 翻转输出供给——UART0 用 TIM0、
//! UART1 用 TIM1（RM 15.1、9.3.2），LPUART（LPMODE=0）用 TIM2 且必须
//! 使能 TIM2 的 TOG 输出（RM 16.1、16.4.3）。本驱动在构造时直接配置
//! 对应定时器为 16 位自动重载模式（CNT 与 ARR 都写装载值，RM 16.4.3
//! 注），因此串口占用 TIM0/1/2；定时器更高级的用法见 [`crate::bt`]，
//! 与本驱动共用同一实例会互相覆盖。
//!
//! 工作模式（SCON.SM01，RM 15.4.1）：
//!
//! - [`Mode::Mode1`]：10 位帧（1 起始 + 8 数据 + 1 停止），波特率可编程；
//! - [`Mode::Mode2`]：11 位帧（+ TB8/RB8 第 9 位），波特率固定
//!   `(DBAUD+1)·f/64`，不占用定时器；
//! - [`Mode::Mode3`]：11 位帧，波特率可编程，第 9 位由 [`Parity`] 软件奇
//!   偶校验或多机通信使用。
//!
//! 本芯片**没有**硬件数据位/停止位/校验配置，帧格式由模式决定，停止位
//! 固定 1 位。Mode2/3 的第 9 位（TB8/RB8）可作软件奇偶校验或多机通信：
//! RM 未规定奇偶约定（手册未说明），本驱动按通用 UART 惯例——偶校验使
//! 含 TB8 的 1 总数为偶、奇校验为奇（见 [`parity_bit`]）。
//!
//! 收发缓存（RM 15.8）：发送侧 UART 无缓存（发送中写 SBUF 会破坏当前
//! 帧），LPUART 有 1 帧发送缓存；接收侧仅 1 帧缓存，新帧到达会覆盖。
//! 标志 RI/TI 硬件置位、软件清零。ICR 为**写 0 清零、写 1 无效**（RM
//! 15.9.6/16.9.6），单字段写会连带清除其他标志，驱动经 [`clear_flags_u`]
//! 显式写全三个字段。
//!
//! 阻塞 API 按查询方式实现：发送"清 TI → 写 SBUF → 忙等 TI"，接收
//! "忙等 RI → 快照 FE/RB8 → 读 SBUF → 清 RI"。中断绑定骨架见
//! [`bind_uart_interrupt!`]（不自动使能 RIEN/TIEN，运行开关在
//! `enable_rx_interrupt`/`enable_tx_interrupt`）。
//!
//! LPUART 差异（RM 第 16 章）：SCLK 由 [`LpUartConfig::sclk`] 选择
//! （PCLK/XTL/RCL），`low_power = true`（SCON.LPMODE=1）时波特率为固定
//! 档 `f_SCLK / (PreScale·4)`、只支持 Mode1/3、不再占用定时器；普通
//! 模式的波特率定时器只能以 PCLK 计数，SCLK ≠ PCLK 的组合被拒绝（RM
//! 未说明其正确单位）。RCL 频率档位按 RCL_CR.TRIM 第 9 位区分
//! 38.4k/32.768k——该映射 RM 未说明（3.5.6 只给校准值地址），属未验证
//! 推断，见差异台账。

use crate::clock::{PeripheralGate, clocks, enable_peripheral};
use crate::gpio::{self, Pull};
use crate::pac;
use crate::peripheral::Peri;

// ---------------------------------------------------------------------------
// 配置类型
// ---------------------------------------------------------------------------

/// 工作模式（SCON.SM01，RM 15.4.1）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Mode {
    /// Mode0：8 位同步半双工（未实现）。
    Mode0,
    /// Mode1：10 位异步帧，波特率由定时器供给（编码 1）。
    #[default]
    Mode1,
    /// Mode2：11 位异步帧，波特率固定 `(DBAUD+1)·f/64`（编码 2）。
    Mode2,
    /// Mode3：11 位异步帧，波特率由定时器供给，第 9 位可作软件校验/多机位（编码 3）。
    Mode3,
}

impl Mode {
    /// SCON.SM01 寄存器编码。
    const fn encode(self) -> u8 {
        match self {
            Mode::Mode0 => 0,
            Mode::Mode1 => 1,
            Mode::Mode2 => 2,
            Mode::Mode3 => 3,
        }
    }

    /// 波特率是否由定时器供给（Mode1/3）。
    const fn uses_timer(self) -> bool {
        matches!(self, Mode::Mode1 | Mode::Mode3)
    }

    /// 是否带第 9 位（Mode2/3）。
    const fn has_9th_bit(self) -> bool {
        matches!(self, Mode::Mode2 | Mode::Mode3)
    }
}

/// 第 9 位（TB8/RB8）的软件奇偶校验（仅 Mode2/3 有意义）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Parity {
    /// 第 9 位不作校验（多机通信地址位或用户自定义）。
    #[default]
    None,
    /// 偶校验：TB8 使发送帧（含 TB8）中 1 的总数为偶数。
    Even,
    /// 奇校验：TB8 使发送帧（含 TB8）中 1 的总数为奇数。
    Odd,
}

/// 多机通信配置（SCON.SM2 + SADDR/SADEN，RM 15.6/15.7，仅 Mode2/3）。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MultiProcessor {
    /// 本机从机地址（SADDR）。
    pub address: u8,
    /// 地址掩码（SADEN），1 的位参与比较。
    pub mask: u8,
}

/// UART0/1 配置。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Config {
    /// 目标波特率。Mode1/3 由定时器分频逼近（四舍五入），Mode2 校验
    /// `(DBAUD+1)·PCLK/64` 与目标一致（允许 ±3% 误差）。
    ///
    /// 默认 115200。按 RM 15.4.2.1 波特率表，PCLK = 4 MHz 时 115200 误差
    /// 8.51%（不可用），建议 RCH 22.12 MHz（误差 0.01%）或 11.0592 MHz
    /// 档；4 MHz 适合 2400~57600 等低波特率。
    pub baudrate: u32,
    /// 工作模式。
    pub mode: Mode,
    /// 第 9 位软件校验（仅 Mode2/3 有意义）。
    pub parity: Parity,
    /// 双倍波特率（SCON.DBAUD）：Mode1/3 分子加倍、Mode2 固定波特率加倍。
    pub double_baud: bool,
    /// 多机通信地址识别；`Some` 时置 SM2 并写 SADDR/SADEN。
    pub multi_processor: Option<MultiProcessor>,
    /// RX 引脚内部上/下拉。
    pub rx_pull: Pull,
    /// TX 引脚内部上/下拉。
    pub tx_pull: Pull,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            baudrate: 115_200,
            mode: Mode::Mode1,
            parity: Parity::None,
            double_baud: false,
            multi_processor: None,
            rx_pull: Pull::None,
            tx_pull: Pull::None,
        }
    }
}

/// 配置错误。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConfigError {
    /// 目标波特率在当前 PCLK/模式下无法配置。
    InvalidBaudrate,
    /// 参数组合硬件不支持，如低功耗模式配了 Mode0/2。
    InvalidMode,
}

/// LPUART 传输时钟（SCON.SCLKSEL，RM 16.9.2 位 12:11）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Sclk {
    /// PCLK（编码 0，复位默认）。
    #[default]
    Pclk,
    /// PCLK（编码 1）。与 [`Sclk::Pclk`] 同为 PCLK，保留编码以对齐手册。
    PclkAlt,
    /// 外部低速 32.768 kHz 晶振（编码 2）。低功耗模式建议时钟源。
    Xtl,
    /// 内部低速 RC，38.4 kHz 或 32.768 kHz 档（编码 3），由 RCL_CR.TRIM 决定。
    Rcl,
}

impl Sclk {
    /// SCON.SCLKSEL 寄存器编码。
    const fn encode(self) -> u8 {
        match self {
            Sclk::Pclk => 0,
            Sclk::PclkAlt => 1,
            Sclk::Xtl => 2,
            Sclk::Rcl => 3,
        }
    }

    /// 当前 SCLK 频率。
    ///
    /// RCL 档位读 RCL_CR.TRIM 第 9 位区分 38.4k/32.768k——TRIM 值与标称
    /// 频率的映射 RM 3.5.6 未说明（手册只给两组校准值地址），该判别属
    /// 未验证推断，见差异台账；复位值 0x33F 会被判为 38.4k。
    fn frequency(self) -> u32 {
        match self {
            Sclk::Pclk | Sclk::PclkAlt => clocks().pclk.0,
            Sclk::Xtl => 32_768,
            Sclk::Rcl => {
                if pac::CLOCK.rcl_cr().read().trim() & (1 << 9) != 0 {
                    38_400
                } else {
                    32_768
                }
            }
        }
    }
}

/// LPUART 低功耗模式 SCLK 预分频（SCON.PRS，仅 LPMODE=1 有效，RM 16.9.2
/// 位 15:13）。
///
/// 判别值即寄存器编码；预分频值 = 2^(7−编码)，波特率 = f_SCLK / (预分频·4)。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LpPrescaler {
    /// ÷128（编码 0）。
    Div128 = 0,
    /// ÷64（编码 1）。
    Div64 = 1,
    /// ÷32（编码 2）。
    Div32 = 2,
    /// ÷16（编码 3）。
    Div16 = 3,
    /// ÷8（编码 4）。
    Div8 = 4,
    /// ÷4（编码 5）。
    Div4 = 5,
    /// ÷2（编码 6）。
    Div2 = 6,
    /// ÷1（编码 7，复位默认）。
    Div1 = 7,
}

impl LpPrescaler {
    /// 预分频值。
    const fn factor(self) -> u32 {
        1u32 << (7 - self as u32)
    }
}

/// LPUART 配置。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LpUartConfig {
    /// 传输时钟 SCLK 选择。
    pub sclk: Sclk,
    /// 低功耗模式（SCON.LPMODE=1）：波特率为固定档 `f_SCLK/(预分频·4)`，
    /// 只支持 Mode1/3，不占用定时器；DeepSleep 下仍可收发（RM 16.1）。
    pub low_power: bool,
    /// 低功耗模式的 SCLK 预分频（仅 `low_power = true` 时使用）。
    pub prescaler: LpPrescaler,
    /// 其余字段同 [`Config`]。
    pub base: Config,
}

impl Default for LpUartConfig {
    fn default() -> Self {
        Self {
            sclk: Sclk::Pclk,
            low_power: false,
            prescaler: LpPrescaler::Div1,
            base: Config::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// 收发错误
// ---------------------------------------------------------------------------

/// 收发错误。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Error {
    /// 接收帧错误（ISR.FE）：Stop 位无效，RM 15.5/16.5。
    Frame,
    /// 软件奇偶校验失败（Mode2/3，RB8 与数据不符）。
    Parity,
}

impl embedded_hal_nb::serial::Error for Error {
    fn kind(&self) -> embedded_hal_nb::serial::ErrorKind {
        match self {
            Error::Frame => embedded_hal_nb::serial::ErrorKind::FrameFormat,
            Error::Parity => embedded_hal_nb::serial::ErrorKind::Parity,
        }
    }
}

// ---------------------------------------------------------------------------
// 波特率计算
// ---------------------------------------------------------------------------

/// Mode1/3 的定时器装载值：`tmload = f·(DBAUD+1)/(baud·32)`，四舍五入。
///
/// 定时器写入值为 `65536 − tmload`，`tmload` 为 0 或超出 16 位时该波特
/// 率不可配置；`baud = 0` 直接不可配置（避免整型除零）。
fn timer_load(f_clock: u32, baud: u32, dbaud: bool) -> Option<u16> {
    if baud == 0 {
        return None;
    }
    let num = f_clock * (dbaud as u32 + 1);
    let load = (num + baud * 16) / (baud * 32);
    if load == 0 || load > 0xFFFF {
        None
    } else {
        Some((0x1_0000 - load) as u16)
    }
}

/// Mode2 固定波特率校验：`(DBAUD+1)·f/64` 与目标误差不超过 3%。
fn mode2_baud_matches(f_clock: u32, baud: u32, dbaud: bool) -> bool {
    if baud == 0 {
        return false;
    }
    let actual = f_clock * (dbaud as u32 + 1) / 64;
    let diff = if actual > baud {
        actual - baud
    } else {
        baud - actual
    };
    diff * 100 <= baud * 3
}

// ---------------------------------------------------------------------------
// 实例能力
// ---------------------------------------------------------------------------

/// UART0/1 实例能力。
#[allow(private_bounds)]
pub trait Instance: crate::peripheral::PeripheralType + 'static {
    /// PAC 寄存器块。
    fn regs() -> pac::uart_ddl_v1::Uart;

    /// 波特率定时器寄存器块（UART0→TIM0、UART1→TIM1，RM 15.1）。
    fn baud_timer() -> pac::bt_ddl_v1::Bt;

    /// 外设时钟门控位。
    fn gate() -> PeripheralGate;
}

/// UART 发送引脚。
pub trait TxPin<T: Instance>: crate::gpio::Pin {
    /// Pxy_SEL 复用编码（RM 表 6-2/逐引脚 SCON 描述）。
    fn af(&self) -> u8;
}

/// UART 接收引脚。
pub trait RxPin<T: Instance>: crate::gpio::Pin {
    /// Pxy_SEL 复用编码。
    fn af(&self) -> u8;
}

/// LPUART 发送引脚。
pub trait LpTxPin: crate::gpio::Pin {
    /// Pxy_SEL 复用编码。
    fn af(&self) -> u8;
}

/// LPUART 接收引脚。
pub trait LpRxPin: crate::gpio::Pin {
    /// Pxy_SEL 复用编码。
    fn af(&self) -> u8;
}

// ---------------------------------------------------------------------------
// 驱动：UART0/1
// ---------------------------------------------------------------------------

/// UART0/1 全双工驱动。
pub struct Uart<'d, T: Instance> {
    /// 持有实例单例维持独占；寄存器访问经 PAC 全局常量。
    #[allow(dead_code)]
    peri: Peri<'d, T>,
    config: Config,
}

/// UART0/1 发送半部。
pub struct UartTx<'d, T: Instance> {
    #[allow(dead_code)]
    peri: Peri<'d, T>,
    config: Config,
}

/// UART0/1 接收半部。
pub struct UartRx<'d, T: Instance> {
    #[allow(dead_code)]
    peri: Peri<'d, T>,
    config: Config,
}

impl<'d, T: Instance> Uart<'d, T> {
    /// 接管实例并配置为全双工串口（含波特率定时器与引脚复用）。
    ///
    /// # Errors
    ///
    /// 目标波特率在当前 PCLK 下无法配置（Mode1/3 装载值超界、Mode2 固定
    /// 波特率偏差超 3%）时返回 [`ConfigError::InvalidBaudrate`]。
    pub fn new(
        peri: Peri<'d, T>,
        tx: Peri<'d, impl TxPin<T>>,
        rx: Peri<'d, impl RxPin<T>>,
        config: Config,
    ) -> Result<Self, ConfigError> {
        if config.mode == Mode::Mode0 {
            // Mode0 同步半双工未实现：REN 语义不同（0 发送/1 接收），
            // 误配会在首次发送处死等 TI（RM 15.9.2 REN 位）。
            return Err(ConfigError::InvalidMode);
        }
        // 先校验波特率再动硬件，错误路径不留下副作用。
        let pclk = clocks().pclk.0;
        let tm = if config.mode.uses_timer() {
            Some(
                timer_load(pclk, config.baudrate, config.double_baud)
                    .ok_or(ConfigError::InvalidBaudrate)?,
            )
        } else if config.mode.has_9th_bit() {
            // Mode2 波特率固定 (DBAUD+1)·PCLK/64，只能校验不能配置。
            if !mode2_baud_matches(pclk, config.baudrate, config.double_baud) {
                return Err(ConfigError::InvalidBaudrate);
            }
            None
        } else {
            None
        };
        let regs = T::regs();
        // 门控：串口本位 + 波特率定时器（BaseTim 覆盖 TIM0/1/2，幂等）。
        enable_peripheral(T::gate(), true);
        if config.mode.uses_timer() {
            enable_peripheral(PeripheralGate::BaseTim, true);
        }
        // 引脚复用：TX 输出、RX 输入（RM 6.4.1 数字功能端口方向约定）。
        gpio::set_pin_af(tx.pin_id(), tx.af(), false, config.tx_pull);
        gpio::set_pin_af(rx.pin_id(), rx.af(), true, config.rx_pull);
        // 波特率定时器：16 位自动重载，CNT 与 ARR 都写装载值（RM 16.4.3 注）。
        if let Some(tm) = tm {
            let bt = T::baud_timer();
            bt.arr().write(|w| w.set_arr(tm));
            bt.cnt().write(|w| w.set_cnt(tm));
            bt.cr().modify(|w| {
                w.set_md(true); // 模式 2：16 位自动重载
                w.set_ct(false); // 定时器功能，PCLK 计数
                w.set_prs(0); // 预分频 1：分频已折入 TM
                w.set_gate(false);
                // TOG_EN 只在 LPUART 侧被 RM 16.1 注明确要求；UART0/1 的
                // tog0/tog1 内部互联是否受 TOG_EN 门控手册未说明，保持
                // 复位值 0（SDK UART 例程同样未开 TOG）。
                w.set_tog_en(false);
                w.set_tr(true); // 启动
            });
        }
        Self::configure(&regs, &config);
        // 清历史标志：ICR 写 0 清零（RM 15.9.6）。
        regs.icr().write(|w| {
            w.set_riclr(false);
            w.set_ticlr(false);
            w.set_feclr(false);
        });
        Ok(Self { peri, config })
    }

    /// 拆分为发送/接收半部，可分别移动。
    pub fn split(self) -> (UartTx<'d, T>, UartRx<'d, T>) {
        let peri = self.peri;
        let config = self.config;
        (
            UartTx {
                peri: unsafe { peri.clone_unchecked() },
                config,
            },
            UartRx { peri, config },
        )
    }

    /// SCON 静态配置（构造共用，不触碰标志与 REN）。
    fn configure(regs: &pac::uart_ddl_v1::Uart, config: &Config) {
        regs.scon().modify(|w| {
            w.set_sm01(config.mode.encode());
            w.set_dbaud(config.double_baud);
            w.set_sm2(config.multi_processor.is_some());
            if config.multi_processor.is_some() {
                // 多机主机默认发地址帧。
                w.set_tb8(true);
            }
        });
        regs.saddr().write(|w| {
            let mp = config.multi_processor;
            w.set_saddr(mp.map_or(0, |m| m.address))
        });
        regs.saden().write(|w| {
            let mp = config.multi_processor;
            w.set_saden(mp.map_or(0, |m| m.mask))
        });
        // 最后置 REN 开接收（RM 15.4.1.3 接收流程第一步）。
        regs.scon().modify(|w| w.set_ren(true));
    }
}

/// ICR 写入辅助：只清 `clear` 列出的标志。
///
/// ICR 为**写 0 清零、写 1 无效**（RM 15.9.6/16.9.6），因此必须显式写全
/// 三个字段——未写字段按 0 处理，会连带清除其他标志（例如全双工下清
/// TI 时抹掉已接收字节的 RI）。
#[inline(always)]
fn clear_flags_u(regs: &pac::uart_ddl_v1::Uart, ri: bool, ti: bool, fe: bool) {
    regs.icr().write(|w| {
        w.set_riclr(!ri);
        w.set_ticlr(!ti);
        w.set_feclr(!fe);
    });
}

/// 同 [`clear_flags_u`]，LPUART 寄存器组（位布局相同，RM 16.9.6）。
#[inline(always)]
fn clear_flags_lp(regs: &pac::lpuart_ddl_v1::Lpuart, ri: bool, ti: bool, fe: bool) {
    regs.icr().write(|w| {
        w.set_riclr(!ri);
        w.set_ticlr(!ti);
        w.set_feclr(!fe);
    });
}

/// 软件奇偶校验的 TB8 取值（Mode2/3，RM 15.4.1.4：TB8 可作校验位）。
///
/// RM 未规定奇偶约定（手册未说明），按通用 UART 惯例：偶校验使含 TB8
/// 的 1 总数为偶、奇校验为奇。
const fn parity_bit(parity: Parity, byte: u8) -> bool {
    let ones = byte.count_ones() & 1;
    match parity {
        // 数据 1 的个数为奇 → TB8 补 1 才能使总数为偶。
        Parity::Even => ones == 1,
        // 数据 1 的个数为偶 → TB8 补 1 才能使总数为奇。
        Parity::Odd => ones == 0,
        Parity::None => false,
    }
}

impl<'d, T: Instance> UartTx<'d, T> {
    /// 阻塞发送一个字节并等待移位完成。
    ///
    /// 查询序列：清 TI → 写 SBUF → 忙等 TI。发送无缓存，写入即启动当前
    /// 帧（RM 15.8.2：发送中写 SBUF 会破坏当前帧）。
    pub fn blocking_write(&mut self, byte: u8) -> Result<(), Error> {
        let regs = T::regs();
        clear_flags_u(&regs, false, true, false);
        if self.config.mode.has_9th_bit() && self.config.parity != Parity::None {
            regs.scon()
                .modify(|w| w.set_tb8(parity_bit(self.config.parity, byte)));
        }
        regs.sbuf().write(|w| w.set_sbuf(byte));
        while !regs.isr().read().ti() {}
        Ok(())
    }

    /// 阻塞发送一段数据。
    pub fn blocking_write_all(&mut self, data: &[u8]) -> Result<(), Error> {
        for &b in data {
            self.blocking_write(b)?;
        }
        Ok(())
    }

    /// 多机通信：选择本帧 TB8（地址帧/数据帧，仅 Mode2/3）。
    ///
    /// 与软件奇偶校验（[`Config::parity`]）争用同一 TB8 位，两者互斥：
    /// 配置了校验时本调用会被 `blocking_write` 覆盖，先勿混用。
    pub fn set_address_frame(&mut self, address: bool) {
        T::regs().scon().modify(|w| w.set_tb8(address));
    }

    /// 使能发送完成中断（SCON.TIEN = 1，RM 15.9.2 位 1）。
    ///
    /// 中断处理骨架见 [`bind_uart_interrupt!`]；NVIC 使能由调用方完成。
    pub fn enable_tx_interrupt(&mut self) {
        T::regs().scon().modify(|w| w.set_tien(true));
    }

    /// 关闭发送完成中断。
    pub fn disable_tx_interrupt(&mut self) {
        T::regs().scon().modify(|w| w.set_tien(false));
    }
}

impl<'d, T: Instance> UartRx<'d, T> {
    /// 阻塞接收一个字节并等待完整帧。
    ///
    /// 查询序列：忙等 RI → 快照 FE/RB8 → 读 SBUF → 清 RI。FE 置位时清
    /// FE 与 RI、丢弃错误帧（RM 15.5）。
    pub fn blocking_read(&mut self) -> Result<u8, Error> {
        let regs = T::regs();
        while !regs.isr().read().ri() {}
        // FE 与 RB8 先于清 RI 快照，避免清标志后新帧覆盖。
        if regs.isr().read().fe() {
            let _ = regs.sbuf().read().sbuf(); // 丢弃错误帧
            clear_flags_u(&regs, true, false, true);
            return Err(Error::Frame);
        }
        let byte = regs.sbuf().read().sbuf();
        let rb8 = regs.scon().read().rb8();
        clear_flags_u(&regs, true, false, false);
        if self.config.mode.has_9th_bit() && self.config.parity != Parity::None {
            // 收方按同一约定核对：偶校验含 RB8 的 1 总数为偶。
            if rb8 != parity_bit(self.config.parity, byte) {
                return Err(Error::Parity);
            }
        }
        Ok(byte)
    }

    /// 多机通信：置/清 SM2 进入或退出地址监听（仅 Mode2/3，RM 15.6）。
    ///
    /// 从机收到匹配地址帧后清 SM2 才能接收后续数据帧。
    pub fn listen_address_only(&mut self, listen: bool) {
        T::regs().scon().modify(|w| w.set_sm2(listen));
    }

    /// 使能接收完成中断（SCON.RIEN = 1，RM 15.9.2 位 0）。
    pub fn enable_rx_interrupt(&mut self) {
        T::regs().scon().modify(|w| w.set_rien(true));
    }

    /// 关闭接收完成中断。
    pub fn disable_rx_interrupt(&mut self) {
        T::regs().scon().modify(|w| w.set_rien(false));
    }
}

impl<'d, T: Instance> Uart<'d, T> {
    /// 见 [`UartTx::blocking_write`]。
    pub fn blocking_write(&mut self, byte: u8) -> Result<(), Error> {
        self.split_tx().blocking_write(byte)
    }

    /// 见 [`UartRx::blocking_read`]。
    pub fn blocking_read(&mut self) -> Result<u8, Error> {
        self.rx().blocking_read()
    }

    fn split_tx(&mut self) -> UartTx<'_, T> {
        UartTx {
            peri: self.peri.reborrow(),
            config: self.config,
        }
    }

    fn rx(&mut self) -> UartRx<'_, T> {
        UartRx {
            peri: self.peri.reborrow(),
            config: self.config,
        }
    }
}

// ---------------------------------------------------------------------------
// 驱动：LPUART
// ---------------------------------------------------------------------------

/// LPUART 全双工驱动。
pub struct LpUart<'d> {
    /// 持有实例单例维持独占；寄存器访问经 PAC 全局常量。
    #[allow(dead_code)]
    peri: Peri<'d, LpUartPeri>,
    config: LpUartConfig,
}

/// LPUART 发送半部。
pub struct LpUartTx<'d> {
    #[allow(dead_code)]
    peri: Peri<'d, LpUartPeri>,
    config: LpUartConfig,
}

/// LPUART 接收半部。
pub struct LpUartRx<'d> {
    #[allow(dead_code)]
    peri: Peri<'d, LpUartPeri>,
    config: LpUartConfig,
}

impl<'d> LpUart<'d> {
    /// 接管实例并按 [`LpUartConfig`] 配置（含引脚复用与波特率）。
    ///
    /// # Errors
    ///
    /// - 低功耗模式配了 Mode0/2 → [`ConfigError::InvalidMode`]；
    /// - 普通模式波特率不可配置 → [`ConfigError::InvalidBaudrate`]。
    pub fn new(
        peri: Peri<'d, LpUartPeri>,
        tx: Peri<'d, impl LpTxPin>,
        rx: Peri<'d, impl LpRxPin>,
        config: LpUartConfig,
    ) -> Result<Self, ConfigError> {
        if config.base.mode == Mode::Mode0 {
            // Mode0 未实现（REN 语义不同，同 UART 侧拒绝理由）。
            return Err(ConfigError::InvalidMode);
        }
        if config.low_power && !config.base.mode.uses_timer() {
            // LPMODE=1 只支持 Mode1/3（RM 16.4.2）。
            return Err(ConfigError::InvalidMode);
        }
        if !config.low_power
            && config.base.mode.uses_timer()
            && config.sclk != Sclk::Pclk
            && config.sclk != Sclk::PclkAlt
        {
            // 普通模式的波特率定时器 TIM2 只能以 PCLK 计数（RM 9.4.4 CT=0），
            // SCLK≠PCLK 时按 f_SCLK 算出的 TM 与定时器节拍不匹配（RM 未说明
            // 该组合的正确单位）。
            return Err(ConfigError::InvalidMode);
        }
        // 先校验波特率再动硬件。
        let sclk_hz = config.sclk.frequency();
        let tm = if config.base.mode.uses_timer() && !config.low_power {
            Some(
                timer_load(sclk_hz, config.base.baudrate, config.base.double_baud)
                    .ok_or(ConfigError::InvalidBaudrate)?,
            )
        } else if config.base.mode.has_9th_bit() && !config.low_power {
            // Mode2 波特率固定 (DBAUD+1)·SCLK/64，普通模式只能校验。
            if !mode2_baud_matches(sclk_hz, config.base.baudrate, config.base.double_baud) {
                return Err(ConfigError::InvalidBaudrate);
            }
            None
        } else {
            None
        };
        let regs = pac::LPUART;
        enable_peripheral(PeripheralGate::Lpuart, true);
        if config.base.mode.uses_timer() && !config.low_power {
            // 普通模式波特率来自 TIM2 的 TOG 输出（RM 16.1/16.4.3）。
            enable_peripheral(PeripheralGate::BaseTim, true);
        }
        gpio::set_pin_af(tx.pin_id(), tx.af(), false, config.base.tx_pull);
        gpio::set_pin_af(rx.pin_id(), rx.af(), true, config.base.rx_pull);
        if let Some(tm) = tm {
            let bt = pac::BT2;
            bt.arr().write(|w| w.set_arr(tm));
            bt.cnt().write(|w| w.set_cnt(tm));
            bt.cr().modify(|w| {
                w.set_md(true);
                w.set_ct(false);
                w.set_prs(0);
                w.set_gate(false);
                // LPMODE=0 必须使能 TIM2 的 TOG 输出（RM 16.4.3 注）。
                w.set_tog_en(true);
                w.set_tr(true);
            });
        }
        Self::configure(&regs, &config);
        regs.icr().write(|w| {
            w.set_riclr(false);
            w.set_ticlr(false);
            w.set_feclr(false);
        });
        Ok(Self { peri, config })
    }

    /// 拆分为发送/接收半部。
    pub fn split(self) -> (LpUartTx<'d>, LpUartRx<'d>) {
        let peri = self.peri;
        let config = self.config;
        (
            LpUartTx {
                peri: unsafe { peri.clone_unchecked() },
                config,
            },
            LpUartRx { peri, config },
        )
    }

    /// SCON 静态配置：LPUART 特有的 LPMODE/SCLKSEL/PRS 与公共字段。
    fn configure(regs: &pac::lpuart_ddl_v1::Lpuart, config: &LpUartConfig) {
        regs.scon().modify(|w| {
            w.set_lpmode(config.low_power);
            w.set_sclksel(config.sclk.encode());
            w.set_prs(config.prescaler as u8);
            w.set_sm01(config.base.mode.encode());
            w.set_dbaud(config.base.double_baud);
            w.set_sm2(config.base.multi_processor.is_some());
            if config.base.multi_processor.is_some() {
                w.set_tb8(true);
            }
            w.set_ren(true);
        });
        let mp = config.base.multi_processor;
        regs.saddr()
            .write(|w| w.set_saddr(mp.map_or(0, |m| m.address)));
        regs.saden()
            .write(|w| w.set_saden(mp.map_or(0, |m| m.mask)));
    }

    /// 低功耗模式下的实际波特率（普通模式返回 None）。
    ///
    /// 低功耗模式波特率为固定档 `f_SCLK / (预分频·4)`（RM 16.4.3），不随
    /// [`Config::baudrate`] 配置改变。
    pub fn low_power_baudrate(&self) -> Option<u32> {
        if self.config.low_power {
            Some(self.config.sclk.frequency() / self.config.prescaler.factor() / 4)
        } else {
            None
        }
    }
}

impl LpUartTx<'_> {
    /// 阻塞发送一个字节并等待完成。
    ///
    /// 与 UART 同为"清 TI → 写 SBUF → 忙等 TI"的查询序列。LPUART 有 1 帧
    /// 发送缓存（ISR.TE），但 TE 极性手册两处描述相反（RM 16.8.2 与
    /// 16.9.5，见差异台账），本驱动每字节等 TI（帧完成）后才写下一个，
    /// 不依赖 TE 判定。
    pub fn blocking_write(&mut self, byte: u8) -> Result<(), Error> {
        let regs = pac::LPUART;
        clear_flags_lp(&regs, false, true, false);
        if self.config.base.mode.has_9th_bit() && self.config.base.parity != Parity::None {
            regs.scon()
                .modify(|w| w.set_tb8(parity_bit(self.config.base.parity, byte)));
        }
        regs.sbuf().write(|w| w.set_sbuf(byte));
        while !regs.isr().read().ti() {}
        Ok(())
    }

    /// 阻塞发送一段数据。
    pub fn blocking_write_all(&mut self, data: &[u8]) -> Result<(), Error> {
        for &b in data {
            self.blocking_write(b)?;
        }
        Ok(())
    }

    /// 多机通信：选择本帧 TB8（地址帧/数据帧，仅 Mode2/3）。
    ///
    /// 与软件奇偶校验互斥（同 [`UartTx::set_address_frame`]）。
    pub fn set_address_frame(&mut self, address: bool) {
        pac::LPUART.scon().modify(|w| w.set_tb8(address));
    }

    /// 使能发送完成中断（SCON.TIEN = 1，RM 16.9.2 位 1）。
    pub fn enable_tx_interrupt(&mut self) {
        pac::LPUART.scon().modify(|w| w.set_tien(true));
    }

    /// 关闭发送完成中断。
    pub fn disable_tx_interrupt(&mut self) {
        pac::LPUART.scon().modify(|w| w.set_tien(false));
    }
}

impl LpUartRx<'_> {
    /// 阻塞接收一个字节并等待完整帧。
    ///
    /// 序列与 [`UartRx::blocking_read`] 相同：忙等 RI → 快照 FE/RB8 →
    /// 读 SBUF → 清 RI；FE 置位时连同 RI 一起清并丢弃错误帧。
    pub fn blocking_read(&mut self) -> Result<u8, Error> {
        let regs = pac::LPUART;
        while !regs.isr().read().ri() {}
        if regs.isr().read().fe() {
            let _ = regs.sbuf().read().sbuf();
            clear_flags_lp(&regs, true, false, true);
            return Err(Error::Frame);
        }
        let byte = regs.sbuf().read().sbuf();
        let rb8 = regs.scon().read().rb8();
        clear_flags_lp(&regs, true, false, false);
        if self.config.base.mode.has_9th_bit() && self.config.base.parity != Parity::None {
            if rb8 != parity_bit(self.config.base.parity, byte) {
                return Err(Error::Parity);
            }
        }
        Ok(byte)
    }

    /// 多机通信：置/清 SM2 进入或退出地址监听（仅 Mode2/3，RM 16.6）。
    pub fn listen_address_only(&mut self, listen: bool) {
        pac::LPUART.scon().modify(|w| w.set_sm2(listen));
    }

    /// 使能接收完成中断（SCON.RIEN = 1，RM 16.9.2 位 0）。
    pub fn enable_rx_interrupt(&mut self) {
        pac::LPUART.scon().modify(|w| w.set_rien(true));
    }

    /// 关闭接收完成中断。
    pub fn disable_rx_interrupt(&mut self) {
        pac::LPUART.scon().modify(|w| w.set_rien(false));
    }
}

impl LpUart<'_> {
    /// 见 [`LpUartTx::blocking_write`]。
    pub fn blocking_write(&mut self, byte: u8) -> Result<(), Error> {
        let config = self.config;
        LpUartTx {
            peri: self.peri.reborrow(),
            config,
        }
        .blocking_write(byte)
    }

    /// 见 [`LpUartRx::blocking_read`]。
    pub fn blocking_read(&mut self) -> Result<u8, Error> {
        let config = self.config;
        LpUartRx {
            peri: self.peri.reborrow(),
            config,
        }
        .blocking_read()
    }
}

// ---------------------------------------------------------------------------
// 实例与引脚单例
// ---------------------------------------------------------------------------

/// UART0 实例单例。
#[derive(Copy, Clone)]
pub struct Uart0 {
    _private: (),
}

/// UART1 实例单例。
#[derive(Copy, Clone)]
pub struct Uart1 {
    _private: (),
}

/// LPUART 实例单例。
#[derive(Copy, Clone)]
pub struct LpUartPeri {
    _private: (),
}

impl crate::peripheral::PeripheralType for Uart0 {}
impl crate::peripheral::PeripheralType for Uart1 {}
impl crate::peripheral::PeripheralType for LpUartPeri {}

macro_rules! uart_peri_steal {
    ($t:ty) => {
        impl $t {
            /// 直接取得该实例，绕过 [`crate::init`] 的独占检查。
            ///
            /// # Safety
            ///
            /// 调用方须保证同一实例不被两处同时持有。
            pub unsafe fn steal() -> Peri<'static, Self> {
                unsafe { Peri::new_unchecked(Self { _private: () }) }
            }
        }
    };
}

uart_peri_steal!(Uart0);
uart_peri_steal!(Uart1);
uart_peri_steal!(LpUartPeri);

impl Instance for Uart0 {
    fn regs() -> pac::uart_ddl_v1::Uart {
        pac::UART0
    }

    fn baud_timer() -> pac::bt_ddl_v1::Bt {
        pac::BT0
    }

    fn gate() -> PeripheralGate {
        PeripheralGate::Uart0
    }
}

impl Instance for Uart1 {
    fn regs() -> pac::uart_ddl_v1::Uart {
        pac::UART1
    }

    fn baud_timer() -> pac::bt_ddl_v1::Bt {
        pac::BT1
    }

    fn gate() -> PeripheralGate {
        PeripheralGate::Uart1
    }
}

/// 为单例引脚实现 UART0/1 的引脚 trait。
///
/// 复用编码取自 RM 表 6-2 与逐引脚 SCON 描述（两处一致），方向与
/// SDK `Gpio_SetFunc_*` 宏相同（TX 输出、RX 输入）。
macro_rules! impl_pin_af {
    ($pin:ident, $($trait:ident<$instance:ident> => $af:expr),* $(,)?) => {
        $(
            impl $trait<$instance> for $pin {
                fn af(&self) -> u8 {
                    $af
                }
            }
        )*
    };
}

/// 为单例引脚实现 LPUART 的引脚 trait（无实例类型参数）。
macro_rules! impl_lp_pin_af {
    ($pin:ident, $($trait:ident => $af:expr),* $(,)?) => {
        $(
            impl $trait for $pin {
                fn af(&self) -> u8 {
                    $af
                }
            }
        )*
    };
}

impl_pin_af!(P02, TxPin<Uart0> => 1);
impl_pin_af!(P01, RxPin<Uart0> => 1);
// P31 = SWCLK、P27 = SWDIO：复用为 UART 前须先经 SYSCTRL1 位 8
// （SWD_USE_IO）把 SWD 引脚交还 GPIO，SEL 复用与 SWD 的优先级手册未说明。
impl_pin_af!(P31, TxPin<Uart0> => 5);
impl_pin_af!(P27, RxPin<Uart0> => 5);
impl_pin_af!(P35, TxPin<Uart0> => 3);
impl_pin_af!(P36, RxPin<Uart0> => 3);
impl_pin_af!(P14, TxPin<Uart0> => 6);
impl_pin_af!(P15, RxPin<Uart0> => 6);

impl_pin_af!(P23, TxPin<Uart1> => 6);
impl_pin_af!(P24, RxPin<Uart1> => 6);
impl_pin_af!(P01, TxPin<Uart1> => 3);
impl_pin_af!(P02, RxPin<Uart1> => 3);
impl_pin_af!(P35, TxPin<Uart1> => 1);
impl_pin_af!(P36, RxPin<Uart1> => 1);

/// 仅 20 引脚封装（C 系列）引出的 P32/P33/P34 映射。
#[cfg(feature = "hc32l110-full-pins")]
mod full_pins {
    use super::{LpRxPin, LpTxPin, P32, P33, P34, RxPin, TxPin, Uart1};

    impl_pin_af!(P32, TxPin<Uart1> => 5);
    impl_pin_af!(P33, RxPin<Uart1> => 5);
    impl_lp_pin_af!(P33, LpRxPin => 1);
    impl_lp_pin_af!(P34, LpTxPin => 2);
}

impl_lp_pin_af!(P26, LpTxPin => 5);
impl_lp_pin_af!(P25, LpRxPin => 5);

// ---------------------------------------------------------------------------
// embedded-hal-nb 串口 trait
// ---------------------------------------------------------------------------

/// `nb` 收发实现缺收发侧上下文的兜底。
///
/// 半部类型持有 `Peri` 与配置，`nb` 实现需要 `&mut self`，与阻塞方法一致。
impl<'d, T: Instance> embedded_hal_nb::serial::ErrorType for UartTx<'d, T> {
    type Error = Error;
}

// `nb::Write` 的实现为**阻塞语义**：UART0/1 发送无缓存（RM 15.8.2），
// 无"忙"状态可查——发送就绪前唯一能做的是忙等 TI。因此 `write()` 委托
// `blocking_write` 而非返回 `WouldBlock`。LPUART 有 1 帧发送缓存（TE），
// 但 TE 极性手册两处描述相反（RM 16.8.2 vs 16.9.5，见差异台账），同样
// 统一为阻塞语义。`Read` 侧有 RI 可查，正确返回 `WouldBlock`。
impl<'d, T: Instance> embedded_hal_nb::serial::Write<u8> for UartTx<'d, T> {
    fn write(&mut self, byte: u8) -> embedded_hal_nb::nb::Result<(), Error> {
        self.blocking_write(byte)?;
        Ok(())
    }

    fn flush(&mut self) -> embedded_hal_nb::nb::Result<(), Error> {
        // blocking_write 已等到 TI（发送完成），无需额外等待。
        Ok(())
    }
}

impl<'d, T: Instance> embedded_hal_nb::serial::ErrorType for UartRx<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_hal_nb::serial::Read<u8> for UartRx<'d, T> {
    fn read(&mut self) -> embedded_hal_nb::nb::Result<u8, Error> {
        let regs = T::regs();
        if !regs.isr().read().ri() {
            return Err(embedded_hal_nb::nb::Error::WouldBlock);
        }
        if regs.isr().read().fe() {
            let _ = regs.sbuf().read().sbuf();
            clear_flags_u(&regs, true, false, true);
            return Err(embedded_hal_nb::nb::Error::Other(Error::Frame));
        }
        let byte = regs.sbuf().read().sbuf();
        let rb8 = regs.scon().read().rb8();
        clear_flags_u(&regs, true, false, false);
        if self.config.mode.has_9th_bit() && self.config.parity != Parity::None {
            if rb8 != parity_bit(self.config.parity, byte) {
                return Err(embedded_hal_nb::nb::Error::Other(Error::Parity));
            }
        }
        Ok(byte)
    }
}

impl<'d, T: Instance> embedded_hal_nb::serial::ErrorType for Uart<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_hal_nb::serial::Write<u8> for Uart<'d, T> {
    fn write(&mut self, byte: u8) -> embedded_hal_nb::nb::Result<(), Error> {
        self.blocking_write(byte)?;
        Ok(())
    }

    fn flush(&mut self) -> embedded_hal_nb::nb::Result<(), Error> {
        Ok(())
    }
}

impl<'d, T: Instance> embedded_hal_nb::serial::Read<u8> for Uart<'d, T> {
    fn read(&mut self) -> embedded_hal_nb::nb::Result<u8, Error> {
        let regs = T::regs();
        if !regs.isr().read().ri() {
            return Err(embedded_hal_nb::nb::Error::WouldBlock);
        }
        if regs.isr().read().fe() {
            let _ = regs.sbuf().read().sbuf();
            clear_flags_u(&regs, true, false, true);
            return Err(embedded_hal_nb::nb::Error::Other(Error::Frame));
        }
        let byte = regs.sbuf().read().sbuf();
        let rb8 = regs.scon().read().rb8();
        clear_flags_u(&regs, true, false, false);
        if self.config.mode.has_9th_bit() && self.config.parity != Parity::None {
            if rb8 != parity_bit(self.config.parity, byte) {
                return Err(embedded_hal_nb::nb::Error::Other(Error::Parity));
            }
        }
        Ok(byte)
    }
}

impl<'d> embedded_hal_nb::serial::ErrorType for LpUartTx<'d> {
    type Error = Error;
}

impl<'d> embedded_hal_nb::serial::Write<u8> for LpUartTx<'d> {
    fn write(&mut self, byte: u8) -> embedded_hal_nb::nb::Result<(), Error> {
        self.blocking_write(byte)?;
        Ok(())
    }

    fn flush(&mut self) -> embedded_hal_nb::nb::Result<(), Error> {
        Ok(())
    }
}

impl<'d> embedded_hal_nb::serial::ErrorType for LpUartRx<'d> {
    type Error = Error;
}

impl<'d> embedded_hal_nb::serial::Read<u8> for LpUartRx<'d> {
    fn read(&mut self) -> embedded_hal_nb::nb::Result<u8, Error> {
        let regs = pac::LPUART;
        if !regs.isr().read().ri() {
            return Err(embedded_hal_nb::nb::Error::WouldBlock);
        }
        if regs.isr().read().fe() {
            let _ = regs.sbuf().read().sbuf();
            clear_flags_lp(&regs, true, false, true);
            return Err(embedded_hal_nb::nb::Error::Other(Error::Frame));
        }
        let byte = regs.sbuf().read().sbuf();
        let rb8 = regs.scon().read().rb8();
        clear_flags_lp(&regs, true, false, false);
        if self.config.base.mode.has_9th_bit() && self.config.base.parity != Parity::None {
            if rb8 != parity_bit(self.config.base.parity, byte) {
                return Err(embedded_hal_nb::nb::Error::Other(Error::Parity));
            }
        }
        Ok(byte)
    }
}

impl<'d> embedded_hal_nb::serial::ErrorType for LpUart<'d> {
    type Error = Error;
}

impl<'d> embedded_hal_nb::serial::Write<u8> for LpUart<'d> {
    fn write(&mut self, byte: u8) -> embedded_hal_nb::nb::Result<(), Error> {
        self.blocking_write(byte)?;
        Ok(())
    }

    fn flush(&mut self) -> embedded_hal_nb::nb::Result<(), Error> {
        Ok(())
    }
}

impl<'d> embedded_hal_nb::serial::Read<u8> for LpUart<'d> {
    fn read(&mut self) -> embedded_hal_nb::nb::Result<u8, Error> {
        let regs = pac::LPUART;
        if !regs.isr().read().ri() {
            return Err(embedded_hal_nb::nb::Error::WouldBlock);
        }
        if regs.isr().read().fe() {
            let _ = regs.sbuf().read().sbuf();
            clear_flags_lp(&regs, true, false, true);
            return Err(embedded_hal_nb::nb::Error::Other(Error::Frame));
        }
        let byte = regs.sbuf().read().sbuf();
        let rb8 = regs.scon().read().rb8();
        clear_flags_lp(&regs, true, false, false);
        if self.config.base.mode.has_9th_bit() && self.config.base.parity != Parity::None {
            if rb8 != parity_bit(self.config.base.parity, byte) {
                return Err(embedded_hal_nb::nb::Error::Other(Error::Parity));
            }
        }
        Ok(byte)
    }
}

// ---------------------------------------------------------------------------
// 中断绑定
// ---------------------------------------------------------------------------

/// 在指定中断向量上绑定 UART0/1 中断处理骨架。
///
/// 处理顺序对齐标志语义：先取 ISR 快照，FE 置位则清 FE（放弃本帧，
/// RM 15.5）；RI 置位则读 SBUF 并清 RI；TI 置位则清 TI。快照后按快照
/// 分支处理，单次 ICR 写只清目标位（其余位写 1，写 1 无效，RM 15.9.6）。
///
/// **骨架宏**：不使能 RIEN/TIEN（SCON 中断允许位保持 0 时 NVIC 不会收到
/// 该中断，RM 15.9.2 位 0/1），也不向用户交付字节——`on_interrupt` 返回
/// 收到的字节与错误位，由调用方按需取用/丢弃并绑定向量。完整的中断驱动
/// （回调通道、使能 API）待中断框架接入时补全。
///
/// # Safety
///
/// 宏生成的 `on_interrupt` 直接访问 PAC 全局常量，调用方须保证 HAL 驱动
/// 唯一持有该外设；同一向量只能绑定一次。
#[macro_export]
macro_rules! bind_uart_interrupt {
    ($irq:ident, $instance:ty) => {
        #[allow(non_snake_case)]
        mod $irq {
            /// 中断处理：返回 `(接收字节, 帧错误, 发送完成)`；未发生的事件
            /// 为 `(None/…, false, false)` 形态。
            #[allow(clippy::type_complexity)]
            pub unsafe fn on_interrupt() -> (Option<u8>, bool, bool) {
                let regs = <$instance as $crate::uart::Instance>::regs();
                let isr = regs.isr().read();
                let mut byte = None;
                if isr.fe() {
                    // 帧错误：丢弃错误帧并连同 FE 清除。
                    byte = None;
                    let _ = regs.sbuf().read().sbuf();
                    regs.icr().write(|w| {
                        w.set_riclr(!isr.ri());
                        w.set_ticlr(true);
                        w.set_feclr(false);
                    });
                    return (byte, true, false);
                }
                if isr.ri() {
                    byte = Some(regs.sbuf().read().sbuf());
                }
                regs.icr().write(|w| {
                    w.set_riclr(!isr.ri());
                    w.set_ticlr(!isr.ti());
                    w.set_feclr(true);
                });
                (byte, false, isr.ti())
            }
        }
    };
}

/// LPUART 的中断绑定骨架，同 [`bind_uart_interrupt!`] 但走 LPUART 寄存器组。
///
/// LPUART 的 ICR 位布局与 UART 相同（RM 16.9.6），TE 标志硬件自清不须
/// 软件处理（RM 16.9.5）。
#[macro_export]
macro_rules! bind_lpuart_interrupt {
    ($irq:ident) => {
        #[allow(non_snake_case)]
        mod $irq {
            /// 中断处理：返回 `(接收字节, 帧错误, 发送完成)`。
            #[allow(clippy::type_complexity)]
            pub unsafe fn on_interrupt() -> (Option<u8>, bool, bool) {
                let regs = $crate::pac::LPUART;
                let isr = regs.isr().read();
                let mut byte = None;
                if isr.fe() {
                    let _ = regs.sbuf().read().sbuf();
                    regs.icr().write(|w| {
                        w.set_riclr(!isr.ri());
                        w.set_ticlr(true);
                        w.set_feclr(false);
                    });
                    return (byte, true, false);
                }
                if isr.ri() {
                    byte = Some(regs.sbuf().read().sbuf());
                }
                regs.icr().write(|w| {
                    w.set_riclr(!isr.ri());
                    w.set_ticlr(!isr.ti());
                    w.set_feclr(true);
                });
                (byte, false, isr.ti())
            }
        }
    };
}

// 引脚单例导入：impl_pin_af! 宏展开按短名解析引脚类型。
use crate::peripherals::{P01, P02, P14, P15, P23, P24, P25, P26, P27, P31, P35, P36};

#[cfg(feature = "hc32l110-full-pins")]
use crate::peripherals::{P32, P33, P34};
