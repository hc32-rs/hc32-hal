//! GPIO 驱动（HC32L110，DDL v2 精简端口布局）。
//!
//! HC32L110 的 GPIO 是单外设多端口结构：端口 P0–P3 的控制寄存器集中在一个
//! 寄存器块内，复用选择为逐引脚的 `Pxy_SEL` 寄存器。所有配置与输出寄存器都
//! 是普通的读-改-写位域，没有 BSRR 型置位/清零镜像寄存器。
//!
//! API 形态参照 Embassy STM32 HAL：[`Flex`] 可在输入/输出/开漏/模拟间任意
//! 切换，[`Input`]/[`Output`]/[`OutputOpenDrain`] 在构造时固定模式。差异在于
//! HC32L110 没有输出速度档位，只有驱动能力档位（PxDR），因此用 [`Drive`]
//! 取代 Embassy 的 `Speed`。

use core::convert::Infallible;
use core::marker::PhantomData;

use crate::pac;
use crate::peripheral::Peri;

/// 数字电平。
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Level {
    /// 低电平。
    Low,
    /// 高电平。
    High,
}

impl From<bool> for Level {
    fn from(value: bool) -> Self {
        if value { Level::High } else { Level::Low }
    }
}

impl From<Level> for bool {
    fn from(level: Level) -> Self {
        matches!(level, Level::High)
    }
}

/// 内部上拉/下拉选择。
///
/// 上拉典型值约 80 kΩ、下拉典型值约 40 kΩ（VCC = 3.3 V，DS 7.3.12）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Pull {
    /// 无上拉/下拉。
    #[default]
    None,
    /// 上拉（PxPU = 1）。
    Up,
    /// 下拉（PxPD = 1）。
    Down,
}

/// 输出驱动能力档位（PxDR）。
///
/// 复位默认为高驱动。电流规格见 DS 7.3.12：高驱动对应双驱规格，低驱动
/// 对应单驱规格。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Drive {
    /// 高驱动（PxDR = 0），复位默认。
    #[default]
    High,
    /// 低驱动（PxDR = 1）。
    Low,
}

/// 引脚运行时编号：`port * 8 + pin`。
#[derive(Copy, Clone)]
pub(crate) struct PinNumber(u8);

impl PinNumber {
    pub(crate) const fn new(number: u8) -> Self {
        Self(number)
    }

    pub(crate) const fn port(self) -> u8 {
        self.0 / 8
    }

    pub(crate) const fn pin(self) -> u8 {
        self.0 % 8
    }
}

/// 单例引脚的私有能力。
pub(crate) trait SealedPin {
    fn pin_number(&self) -> PinNumber;
}

/// GPIO 引脚。
///
/// 由 [`crate::init`] 发放的引脚单例（`peripherals::P01` 等）实现本 trait。
#[allow(private_bounds)]
pub trait Pin: crate::peripheral::PeripheralType + SealedPin + Sized + 'static {
    /// 端口号（0–3）。
    fn port(&self) -> u8 {
        self.pin_number().port()
    }

    /// 端口内位号。
    fn pin(&self) -> u8 {
        self.pin_number().pin()
    }
}

// ---------------------------------------------------------------------------
// 寄存器分发：按 (port, pin) 选取对应的 typed 访问器。
// 位域字段名即引脚名（P01…P36），与 PxDIR/PxOUT/PxIN/PxPU/PxPD/PxOD/PxDR/
// PxADS 的字段一一对应。
// ---------------------------------------------------------------------------

/// 方向位（PxDIR）：`true` = 输入，`false` = 输出。
#[inline(always)]
fn set_dir(n: PinNumber, input: bool) {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p0dir().modify(|w| w.set_p01(input)),
        (0, 2) => pac::GPIO.p0dir().modify(|w| w.set_p02(input)),
        (0, 3) => pac::GPIO.p0dir().modify(|w| w.set_p03(input)),
        (1, 4) => pac::GPIO.p1dir().modify(|w| w.set_p14(input)),
        (1, 5) => pac::GPIO.p1dir().modify(|w| w.set_p15(input)),
        (2, 3) => pac::GPIO.p2dir().modify(|w| w.set_p23(input)),
        (2, 4) => pac::GPIO.p2dir().modify(|w| w.set_p24(input)),
        (2, 5) => pac::GPIO.p2dir().modify(|w| w.set_p25(input)),
        (2, 6) => pac::GPIO.p2dir().modify(|w| w.set_p26(input)),
        (2, 7) => pac::GPIO.p2dir().modify(|w| w.set_p27(input)),
        (3, 1) => pac::GPIO.p3dir().modify(|w| w.set_p31(input)),
        (3, 2) => pac::GPIO.p3dir().modify(|w| w.set_p32(input)),
        (3, 3) => pac::GPIO.p3dir().modify(|w| w.set_p33(input)),
        (3, 4) => pac::GPIO.p3dir().modify(|w| w.set_p34(input)),
        (3, 5) => pac::GPIO.p3dir().modify(|w| w.set_p35(input)),
        (3, 6) => pac::GPIO.p3dir().modify(|w| w.set_p36(input)),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

/// 输出锁存位（PxOUT）：`true` = 高。
#[inline(always)]
fn set_out(n: PinNumber, high: bool) {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p0out().modify(|w| w.set_p01(high)),
        (0, 2) => pac::GPIO.p0out().modify(|w| w.set_p02(high)),
        (0, 3) => pac::GPIO.p0out().modify(|w| w.set_p03(high)),
        (1, 4) => pac::GPIO.p1out().modify(|w| w.set_p14(high)),
        (1, 5) => pac::GPIO.p1out().modify(|w| w.set_p15(high)),
        (2, 3) => pac::GPIO.p2out().modify(|w| w.set_p23(high)),
        (2, 4) => pac::GPIO.p2out().modify(|w| w.set_p24(high)),
        (2, 5) => pac::GPIO.p2out().modify(|w| w.set_p25(high)),
        (2, 6) => pac::GPIO.p2out().modify(|w| w.set_p26(high)),
        (2, 7) => pac::GPIO.p2out().modify(|w| w.set_p27(high)),
        (3, 1) => pac::GPIO.p3out().modify(|w| w.set_p31(high)),
        (3, 2) => pac::GPIO.p3out().modify(|w| w.set_p32(high)),
        (3, 3) => pac::GPIO.p3out().modify(|w| w.set_p33(high)),
        (3, 4) => pac::GPIO.p3out().modify(|w| w.set_p34(high)),
        (3, 5) => pac::GPIO.p3out().modify(|w| w.set_p35(high)),
        (3, 6) => pac::GPIO.p3out().modify(|w| w.set_p36(high)),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

/// 输出锁存位（PxOUT）读取：`true` = 高。
#[inline(always)]
fn read_out(n: PinNumber) -> bool {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p0out().read().p01(),
        (0, 2) => pac::GPIO.p0out().read().p02(),
        (0, 3) => pac::GPIO.p0out().read().p03(),
        (1, 4) => pac::GPIO.p1out().read().p14(),
        (1, 5) => pac::GPIO.p1out().read().p15(),
        (2, 3) => pac::GPIO.p2out().read().p23(),
        (2, 4) => pac::GPIO.p2out().read().p24(),
        (2, 5) => pac::GPIO.p2out().read().p25(),
        (2, 6) => pac::GPIO.p2out().read().p26(),
        (2, 7) => pac::GPIO.p2out().read().p27(),
        (3, 1) => pac::GPIO.p3out().read().p31(),
        (3, 2) => pac::GPIO.p3out().read().p32(),
        (3, 3) => pac::GPIO.p3out().read().p33(),
        (3, 4) => pac::GPIO.p3out().read().p34(),
        (3, 5) => pac::GPIO.p3out().read().p35(),
        (3, 6) => pac::GPIO.p3out().read().p36(),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

/// 输入电平位（PxIN）读取：`true` = 高。
#[inline(always)]
fn read_in(n: PinNumber) -> bool {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p0in().read().p01(),
        (0, 2) => pac::GPIO.p0in().read().p02(),
        (0, 3) => pac::GPIO.p0in().read().p03(),
        (1, 4) => pac::GPIO.p1in().read().p14(),
        (1, 5) => pac::GPIO.p1in().read().p15(),
        (2, 3) => pac::GPIO.p2in().read().p23(),
        (2, 4) => pac::GPIO.p2in().read().p24(),
        (2, 5) => pac::GPIO.p2in().read().p25(),
        (2, 6) => pac::GPIO.p2in().read().p26(),
        (2, 7) => pac::GPIO.p2in().read().p27(),
        (3, 1) => pac::GPIO.p3in().read().p31(),
        (3, 2) => pac::GPIO.p3in().read().p32(),
        (3, 3) => pac::GPIO.p3in().read().p33(),
        (3, 4) => pac::GPIO.p3in().read().p34(),
        (3, 5) => pac::GPIO.p3in().read().p35(),
        (3, 6) => pac::GPIO.p3in().read().p36(),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

/// 上拉使能位（PxPU）。
#[inline(always)]
fn set_pu(n: PinNumber, enable: bool) {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p0pu().modify(|w| w.set_p01(enable)),
        (0, 2) => pac::GPIO.p0pu().modify(|w| w.set_p02(enable)),
        (0, 3) => pac::GPIO.p0pu().modify(|w| w.set_p03(enable)),
        (1, 4) => pac::GPIO.p1pu().modify(|w| w.set_p14(enable)),
        (1, 5) => pac::GPIO.p1pu().modify(|w| w.set_p15(enable)),
        (2, 3) => pac::GPIO.p2pu().modify(|w| w.set_p23(enable)),
        (2, 4) => pac::GPIO.p2pu().modify(|w| w.set_p24(enable)),
        (2, 5) => pac::GPIO.p2pu().modify(|w| w.set_p25(enable)),
        (2, 6) => pac::GPIO.p2pu().modify(|w| w.set_p26(enable)),
        (2, 7) => pac::GPIO.p2pu().modify(|w| w.set_p27(enable)),
        (3, 1) => pac::GPIO.p3pu().modify(|w| w.set_p31(enable)),
        (3, 2) => pac::GPIO.p3pu().modify(|w| w.set_p32(enable)),
        (3, 3) => pac::GPIO.p3pu().modify(|w| w.set_p33(enable)),
        (3, 4) => pac::GPIO.p3pu().modify(|w| w.set_p34(enable)),
        (3, 5) => pac::GPIO.p3pu().modify(|w| w.set_p35(enable)),
        (3, 6) => pac::GPIO.p3pu().modify(|w| w.set_p36(enable)),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

/// 下拉使能位（PxPD）。
#[inline(always)]
fn set_pd(n: PinNumber, enable: bool) {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p0pd().modify(|w| w.set_p01(enable)),
        (0, 2) => pac::GPIO.p0pd().modify(|w| w.set_p02(enable)),
        (0, 3) => pac::GPIO.p0pd().modify(|w| w.set_p03(enable)),
        (1, 4) => pac::GPIO.p1pd().modify(|w| w.set_p14(enable)),
        (1, 5) => pac::GPIO.p1pd().modify(|w| w.set_p15(enable)),
        (2, 3) => pac::GPIO.p2pd().modify(|w| w.set_p23(enable)),
        (2, 4) => pac::GPIO.p2pd().modify(|w| w.set_p24(enable)),
        (2, 5) => pac::GPIO.p2pd().modify(|w| w.set_p25(enable)),
        (2, 6) => pac::GPIO.p2pd().modify(|w| w.set_p26(enable)),
        (2, 7) => pac::GPIO.p2pd().modify(|w| w.set_p27(enable)),
        (3, 1) => pac::GPIO.p3pd().modify(|w| w.set_p31(enable)),
        (3, 2) => pac::GPIO.p3pd().modify(|w| w.set_p32(enable)),
        (3, 3) => pac::GPIO.p3pd().modify(|w| w.set_p33(enable)),
        (3, 4) => pac::GPIO.p3pd().modify(|w| w.set_p34(enable)),
        (3, 5) => pac::GPIO.p3pd().modify(|w| w.set_p35(enable)),
        (3, 6) => pac::GPIO.p3pd().modify(|w| w.set_p36(enable)),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

/// 开漏使能位（PxOD）。
#[inline(always)]
fn set_od(n: PinNumber, enable: bool) {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p0od().modify(|w| w.set_p01(enable)),
        (0, 2) => pac::GPIO.p0od().modify(|w| w.set_p02(enable)),
        (0, 3) => pac::GPIO.p0od().modify(|w| w.set_p03(enable)),
        (1, 4) => pac::GPIO.p1od().modify(|w| w.set_p14(enable)),
        (1, 5) => pac::GPIO.p1od().modify(|w| w.set_p15(enable)),
        (2, 3) => pac::GPIO.p2od().modify(|w| w.set_p23(enable)),
        (2, 4) => pac::GPIO.p2od().modify(|w| w.set_p24(enable)),
        (2, 5) => pac::GPIO.p2od().modify(|w| w.set_p25(enable)),
        (2, 6) => pac::GPIO.p2od().modify(|w| w.set_p26(enable)),
        (2, 7) => pac::GPIO.p2od().modify(|w| w.set_p27(enable)),
        (3, 1) => pac::GPIO.p3od().modify(|w| w.set_p31(enable)),
        (3, 2) => pac::GPIO.p3od().modify(|w| w.set_p32(enable)),
        (3, 3) => pac::GPIO.p3od().modify(|w| w.set_p33(enable)),
        (3, 4) => pac::GPIO.p3od().modify(|w| w.set_p34(enable)),
        (3, 5) => pac::GPIO.p3od().modify(|w| w.set_p35(enable)),
        (3, 6) => pac::GPIO.p3od().modify(|w| w.set_p36(enable)),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

/// 驱动能力位（PxDR）：`true` = 低驱动。
#[inline(always)]
fn set_dr(n: PinNumber, low_drive: bool) {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p0dr().modify(|w| w.set_p01(low_drive)),
        (0, 2) => pac::GPIO.p0dr().modify(|w| w.set_p02(low_drive)),
        (0, 3) => pac::GPIO.p0dr().modify(|w| w.set_p03(low_drive)),
        (1, 4) => pac::GPIO.p1dr().modify(|w| w.set_p14(low_drive)),
        (1, 5) => pac::GPIO.p1dr().modify(|w| w.set_p15(low_drive)),
        (2, 3) => pac::GPIO.p2dr().modify(|w| w.set_p23(low_drive)),
        (2, 4) => pac::GPIO.p2dr().modify(|w| w.set_p24(low_drive)),
        (2, 5) => pac::GPIO.p2dr().modify(|w| w.set_p25(low_drive)),
        (2, 6) => pac::GPIO.p2dr().modify(|w| w.set_p26(low_drive)),
        (2, 7) => pac::GPIO.p2dr().modify(|w| w.set_p27(low_drive)),
        (3, 1) => pac::GPIO.p3dr().modify(|w| w.set_p31(low_drive)),
        (3, 2) => pac::GPIO.p3dr().modify(|w| w.set_p32(low_drive)),
        (3, 3) => pac::GPIO.p3dr().modify(|w| w.set_p33(low_drive)),
        (3, 4) => pac::GPIO.p3dr().modify(|w| w.set_p34(low_drive)),
        (3, 5) => pac::GPIO.p3dr().modify(|w| w.set_p35(low_drive)),
        (3, 6) => pac::GPIO.p3dr().modify(|w| w.set_p36(low_drive)),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

/// 模拟/数字选择位（PxADS）：`true` = 模拟。
#[inline(always)]
fn set_ads(n: PinNumber, analog: bool) {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p0ads().modify(|w| w.set_p01(analog)),
        (0, 2) => pac::GPIO.p0ads().modify(|w| w.set_p02(analog)),
        (0, 3) => pac::GPIO.p0ads().modify(|w| w.set_p03(analog)),
        (1, 4) => pac::GPIO.p1ads().modify(|w| w.set_p14(analog)),
        (1, 5) => pac::GPIO.p1ads().modify(|w| w.set_p15(analog)),
        (2, 3) => pac::GPIO.p2ads().modify(|w| w.set_p23(analog)),
        (2, 4) => pac::GPIO.p2ads().modify(|w| w.set_p24(analog)),
        (2, 5) => pac::GPIO.p2ads().modify(|w| w.set_p25(analog)),
        (2, 6) => pac::GPIO.p2ads().modify(|w| w.set_p26(analog)),
        (2, 7) => pac::GPIO.p2ads().modify(|w| w.set_p27(analog)),
        (3, 1) => pac::GPIO.p3ads().modify(|w| w.set_p31(analog)),
        (3, 2) => pac::GPIO.p3ads().modify(|w| w.set_p32(analog)),
        (3, 3) => pac::GPIO.p3ads().modify(|w| w.set_p33(analog)),
        (3, 4) => pac::GPIO.p3ads().modify(|w| w.set_p34(analog)),
        (3, 5) => pac::GPIO.p3ads().modify(|w| w.set_p35(analog)),
        (3, 6) => pac::GPIO.p3ads().modify(|w| w.set_p36(analog)),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

/// 复用功能选择（Pxy_SEL）：0 = GPIO 功能，1–7 为复用功能。
///
/// 本驱动只在数字模式切换时写 0；具体的复用编号 API 随通信外设驱动提供。
#[inline(always)]
fn set_sel(n: PinNumber, func: u8) {
    match (n.port(), n.pin()) {
        (0, 1) => pac::GPIO.p01_sel().modify(|w| w.set_sel(func)),
        (0, 2) => pac::GPIO.p02_sel().modify(|w| w.set_sel(func)),
        (0, 3) => pac::GPIO.p03_sel().modify(|w| w.set_sel(func)),
        (1, 4) => pac::GPIO.p14_sel().modify(|w| w.set_sel(func)),
        (1, 5) => pac::GPIO.p15_sel().modify(|w| w.set_sel(func)),
        (2, 3) => pac::GPIO.p23_sel().modify(|w| w.set_sel(func)),
        (2, 4) => pac::GPIO.p24_sel().modify(|w| w.set_sel(func)),
        (2, 5) => pac::GPIO.p25_sel().modify(|w| w.set_sel(func)),
        (2, 6) => pac::GPIO.p26_sel().modify(|w| w.set_sel(func)),
        (2, 7) => pac::GPIO.p27_sel().modify(|w| w.set_sel(func)),
        (3, 1) => pac::GPIO.p31_sel().modify(|w| w.set_sel(func)),
        (3, 2) => pac::GPIO.p32_sel().modify(|w| w.set_sel(func)),
        (3, 3) => pac::GPIO.p33_sel().modify(|w| w.set_sel(func)),
        (3, 4) => pac::GPIO.p34_sel().modify(|w| w.set_sel(func)),
        (3, 5) => pac::GPIO.p35_sel().modify(|w| w.set_sel(func)),
        (3, 6) => pac::GPIO.p36_sel().modify(|w| w.set_sel(func)),
        _ => unreachable!("HC32L110 无此引脚"),
    }
}

// ---------------------------------------------------------------------------
// Flex
// ---------------------------------------------------------------------------

/// 模式未定的引脚。
///
/// 构造后引脚保持原状态，可随时在输入/输出/开漏/模拟间切换；释放（Drop）
/// 时自动断开为模拟态。模式固定的包装类型见 [`Input`]、[`Output`] 和
/// [`OutputOpenDrain`]。
pub struct Flex<'d> {
    pub(crate) pin: PinNumber,
    _phantom: PhantomData<&'d mut ()>,
}

impl<'d> Flex<'d> {
    /// 接管一个引脚单例，不改变其当前配置。
    pub fn new(pin: Peri<'d, impl Pin>) -> Self {
        // GPIO 门控复位后为关（RM 3.5.8 位 28，复位值 0xC080_0000），先打开。
        crate::clock::enable_gpio();
        Self {
            pin: pin.pin_number(),
            _phantom: PhantomData,
        }
    }

    /// 配置为数字输入。
    ///
    /// SEL 置 0（GPIO 功能）、ADS 置 0（数字）、DR/OD 复位，方向置输入，
    /// 再按 `pull` 配置上/下拉。
    pub fn set_as_input(&mut self, pull: Pull) {
        let n = self.pin;
        set_sel(n, 0);
        set_ads(n, false);
        set_dr(n, false);
        set_od(n, false);
        set_pd(n, matches!(pull, Pull::Down));
        set_pu(n, matches!(pull, Pull::Up));
        set_dir(n, true);
    }

    /// 配置为推挽输出，驱动能力 `drive`。
    pub fn set_as_output(&mut self, drive: Drive) {
        let n = self.pin;
        set_sel(n, 0);
        set_ads(n, false);
        set_pd(n, false);
        set_pu(n, false);
        set_od(n, false);
        set_dr(n, matches!(drive, Drive::Low));
        set_dir(n, false);
    }

    /// 配置为开漏输出（无内部上/下拉），驱动能力 `drive`。
    pub fn set_as_input_output(&mut self, drive: Drive) {
        self.set_as_input_output_pull(drive, Pull::None);
    }

    /// 配置为开漏输出并指定内部上/下拉，驱动能力 `drive`。
    ///
    /// 开漏模式下输出 0 拉低总线，输出 1 释放总线由外部或内部上拉决定，
    /// 输入路径保持可用（可读 PxIN 观察总线实际电平）。
    pub fn set_as_input_output_pull(&mut self, drive: Drive, pull: Pull) {
        let n = self.pin;
        set_sel(n, 0);
        set_ads(n, false);
        set_dr(n, matches!(drive, Drive::Low));
        set_pd(n, matches!(pull, Pull::Down));
        set_pu(n, matches!(pull, Pull::Up));
        set_od(n, true);
        set_dir(n, false);
    }

    /// 配置为模拟功能（PxADS = 1，数字输入缓冲断开）。
    ///
    /// P01/P02 与 P14/P15 同时是外部晶振 XTH/XTL 引脚（RM 3.2.2/3.2.3）：
    ///
    /// - 使用外部晶振时，同组两个引脚都须配置为模拟端口；
    /// - 使用外部时钟直接注入（XTH 从 P01 输入、XTL 从 P14 输入）时，
    ///   注入脚保持数字 GPIO 输入，同组另一脚（P02/P15）必须配置为模拟
    ///   端口且禁作 GPIO（RM 3.1）。
    pub fn set_as_analog(&mut self) {
        let n = self.pin;
        set_sel(n, 0);
        set_dir(n, true);
        set_pu(n, false);
        set_pd(n, false);
        set_od(n, false);
        set_dr(n, false);
        set_ads(n, true);
    }

    /// 断开引脚：模拟态（PxADS = 1），上下拉与开漏全部关闭。
    ///
    /// Drop 时自动调用。模拟态关闭数字输入缓冲，漏电流最小。
    pub fn set_as_disconnected(&mut self) {
        self.set_as_analog();
    }

    /// 读引脚输入电平（PxIN）。
    pub fn is_high(&self) -> bool {
        read_in(self.pin)
    }

    /// 读引脚输入电平（PxIN）。
    pub fn is_low(&self) -> bool {
        !read_in(self.pin)
    }

    /// 读引脚输入电平（PxIN）。
    pub fn level(&self) -> Level {
        Level::from(read_in(self.pin))
    }

    /// 输出锁存置高（PxOUT = 1）。
    pub fn set_high(&mut self) {
        set_out(self.pin, true);
    }

    /// 输出锁存置低（PxOUT = 0）。
    pub fn set_low(&mut self) {
        set_out(self.pin, false);
    }

    /// 输出锁存写入指定电平（PxOUT）。
    pub fn set_level(&mut self, level: Level) {
        set_out(self.pin, bool::from(level));
    }

    /// 读输出锁存（PxOUT）。
    pub fn is_set_high(&self) -> bool {
        read_out(self.pin)
    }

    /// 读输出锁存（PxOUT）。
    pub fn is_set_low(&self) -> bool {
        !read_out(self.pin)
    }

    /// 读输出锁存（PxOUT）。
    pub fn output_level(&self) -> Level {
        Level::from(read_out(self.pin))
    }

    /// 翻转输出锁存（PxOUT）。
    pub fn toggle(&mut self) {
        let high = read_out(self.pin);
        set_out(self.pin, !high);
    }

    /// 阻止 Drop 时自动断开，保留引脚当前配置。
    pub fn persist(self) {
        core::mem::forget(self)
    }
}

impl Drop for Flex<'_> {
    fn drop(&mut self) {
        self.set_as_disconnected();
    }
}

// ---------------------------------------------------------------------------
// 模式固定的包装类型
// ---------------------------------------------------------------------------

/// 数字输入引脚。
pub struct Input<'d> {
    pub(crate) pin: Flex<'d>,
}

impl<'d> Input<'d> {
    /// 以 `pull` 配置接管引脚为数字输入。
    pub fn new(pin: Peri<'d, impl Pin>, pull: Pull) -> Self {
        let mut pin = Flex::new(pin);
        pin.set_as_input(pull);
        Self { pin }
    }

    /// 从已配置的 [`Flex`] 构造，不重写寄存器。
    pub fn from_flex(pin: Flex<'d>) -> Self {
        Self { pin }
    }

    /// 读引脚输入电平（PxIN）。
    pub fn is_high(&self) -> bool {
        self.pin.is_high()
    }

    /// 读引脚输入电平（PxIN）。
    pub fn is_low(&self) -> bool {
        self.pin.is_low()
    }

    /// 读引脚输入电平（PxIN）。
    pub fn level(&self) -> Level {
        self.pin.level()
    }

    /// 阻止 Drop 时自动断开，保留引脚当前配置。
    pub fn persist(self) {
        self.pin.persist()
    }
}

/// 推挽输出引脚。
pub struct Output<'d> {
    pub(crate) pin: Flex<'d>,
}

impl<'d> Output<'d> {
    /// 以 `drive` 驱动能力接管引脚为推挽输出。
    ///
    /// 先写输出锁存再切方向，避免切换瞬间的初值毛刺。
    pub fn new(pin: Peri<'d, impl Pin>, initial_output: Level, drive: Drive) -> Self {
        let mut pin = Flex::new(pin);
        pin.set_level(initial_output);
        pin.set_as_output(drive);
        Self { pin }
    }

    /// 从已配置的 [`Flex`] 构造，不重写寄存器。
    pub fn from_flex(pin: Flex<'d>) -> Self {
        Self { pin }
    }

    /// 输出锁存置高（PxOUT = 1）。
    pub fn set_high(&mut self) {
        self.pin.set_high();
    }

    /// 输出锁存置低（PxOUT = 0）。
    pub fn set_low(&mut self) {
        self.pin.set_low();
    }

    /// 输出锁存写入指定电平（PxOUT）。
    pub fn set_level(&mut self, level: Level) {
        self.pin.set_level(level);
    }

    /// 读输出锁存（PxOUT）。
    pub fn is_set_high(&self) -> bool {
        self.pin.is_set_high()
    }

    /// 读输出锁存（PxOUT）。
    pub fn is_set_low(&self) -> bool {
        self.pin.is_set_low()
    }

    /// 读输出锁存（PxOUT）。
    pub fn output_level(&self) -> Level {
        self.pin.output_level()
    }

    /// 翻转输出锁存（PxOUT）。
    pub fn toggle(&mut self) {
        self.pin.toggle();
    }

    /// 阻止 Drop 时自动断开，保留引脚当前配置。
    pub fn persist(self) {
        self.pin.persist()
    }
}

/// 开漏输出引脚。
///
/// 输入路径保持可用：`is_high` 等读的是 PxIN（总线实际电平），`is_set_high`
/// 等读的是 PxOUT（输出锁存）。
pub struct OutputOpenDrain<'d> {
    pub(crate) pin: Flex<'d>,
}

impl<'d> OutputOpenDrain<'d> {
    /// 以 `drive` 驱动能力接管引脚为开漏输出，无内部上/下拉。
    pub fn new(pin: Peri<'d, impl Pin>, initial_output: Level, drive: Drive) -> Self {
        Self::new_pull(pin, initial_output, drive, Pull::None)
    }

    /// 以 `drive` 驱动能力接管引脚为开漏输出并指定内部上/下拉。
    pub fn new_pull(
        pin: Peri<'d, impl Pin>,
        initial_output: Level,
        drive: Drive,
        pull: Pull,
    ) -> Self {
        let mut pin = Flex::new(pin);
        pin.set_level(initial_output);
        pin.set_as_input_output_pull(drive, pull);
        Self { pin }
    }

    /// 从已配置的 [`Flex`] 构造，不重写寄存器。
    pub fn from_flex(pin: Flex<'d>) -> Self {
        Self { pin }
    }

    /// 读总线实际电平（PxIN）。
    pub fn is_high(&self) -> bool {
        self.pin.is_high()
    }

    /// 读总线实际电平（PxIN）。
    pub fn is_low(&self) -> bool {
        self.pin.is_low()
    }

    /// 读总线实际电平（PxIN）。
    pub fn level(&self) -> Level {
        self.pin.level()
    }

    /// 释放总线（PxOUT = 1，输出管不拉低）。
    pub fn set_high(&mut self) {
        self.pin.set_high();
    }

    /// 拉低总线（PxOUT = 0）。
    pub fn set_low(&mut self) {
        self.pin.set_low();
    }

    /// 写输出锁存（PxOUT）。
    pub fn set_level(&mut self, level: Level) {
        self.pin.set_level(level);
    }

    /// 读输出锁存（PxOUT）。
    pub fn is_set_high(&self) -> bool {
        self.pin.is_set_high()
    }

    /// 读输出锁存（PxOUT）。
    pub fn is_set_low(&self) -> bool {
        self.pin.is_set_low()
    }

    /// 读输出锁存（PxOUT）。
    pub fn output_level(&self) -> Level {
        self.pin.output_level()
    }

    /// 翻转输出锁存（PxOUT）。
    pub fn toggle(&mut self) {
        self.pin.toggle();
    }

    /// 阻止 Drop 时自动断开，保留引脚当前配置。
    pub fn persist(self) {
        self.pin.persist()
    }
}

// ---------------------------------------------------------------------------
// embedded-hal 1.0 数字 trait
// ---------------------------------------------------------------------------

macro_rules! impl_error_type {
    ($ty:ty) => {
        impl embedded_hal::digital::ErrorType for $ty {
            type Error = Infallible;
        }
    };
}

macro_rules! impl_input_pin {
    ($ty:ty) => {
        impl embedded_hal::digital::InputPin for $ty {
            #[inline]
            fn is_high(&mut self) -> Result<bool, Self::Error> {
                Ok(Self::is_high(self))
            }

            #[inline]
            fn is_low(&mut self) -> Result<bool, Self::Error> {
                Ok(Self::is_low(self))
            }
        }
    };
}

macro_rules! impl_output_pin {
    ($ty:ty) => {
        impl embedded_hal::digital::OutputPin for $ty {
            #[inline]
            fn set_high(&mut self) -> Result<(), Self::Error> {
                Self::set_high(self);
                Ok(())
            }

            #[inline]
            fn set_low(&mut self) -> Result<(), Self::Error> {
                Self::set_low(self);
                Ok(())
            }
        }

        impl embedded_hal::digital::StatefulOutputPin for $ty {
            #[inline]
            fn is_set_high(&mut self) -> Result<bool, Self::Error> {
                Ok(Self::is_set_high(self))
            }

            #[inline]
            fn is_set_low(&mut self) -> Result<bool, Self::Error> {
                Ok(Self::is_set_low(self))
            }

            // eh 1.0 的 toggle 是 StatefulOutputPin 的默认方法，
            // 覆写为单次 PxOUT 读-改-写翻转。
            #[inline]
            fn toggle(&mut self) -> Result<(), Self::Error> {
                Self::toggle(self);
                Ok(())
            }
        }
    };
}

impl_error_type!(Flex<'_>);
impl_input_pin!(Flex<'_>);
impl_output_pin!(Flex<'_>);
impl_error_type!(Input<'_>);
impl_input_pin!(Input<'_>);
impl_error_type!(Output<'_>);
impl_output_pin!(Output<'_>);
impl_error_type!(OutputOpenDrain<'_>);
impl_input_pin!(OutputOpenDrain<'_>);
impl_output_pin!(OutputOpenDrain<'_>);
