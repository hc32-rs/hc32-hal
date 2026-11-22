//! GPIO 驱动。
//!
//! 模块入口保留应用可见的引脚类型与 API：单例引脚（`peripherals::P01`
//! 等）实现 [`Pin`]，[`Flex`] 可在输入/输出/开漏/模拟间任意切换，
//! [`Input`]/[`Output`]/[`OutputOpenDrain`] 在构造时固定模式。
//!
//! 寄存器操作按 IP 后端拆分：HC32L110 等精简端口布局为 DDL v2
//! （[`gpio_v2`]），HC32L13x 端口阵列为 DDL v3_3（[`gpio_v3_3`]），
//! HC32F420 每端口一个独立实例、一份后端服务四个端口（[`gpio_v3_1`]）。
//! 接入新布局时沿用本入口的公开类型与 embedded-hal trait 实现，仅把有
//! 差异的寄存器操作移入新的私有 IP 文件（AGENTS.md「HAL 源码布局与
//! 命名」）。
//!
//! 各布局都没有输出速度档位，只有驱动能力档位（v2/v3_3 的 PxDR、v3_1
//! 的 GPIO_DRIVER），因此用 [`Drive`] 取代 Embassy STM32 HAL 的 `Speed`。
//!
//! 写路径差异：v3_3 的 `set_high`/`set_low` 走位置位/位清零寄存器
//! （PxBSET/PxBCLR，W1 位，整字只写目标引脚掩码，SDK `Gpio_SetIO`/
//! `Gpio_ClrIO` 同做法）；v3_1 走 GPIO_BSRR.BS/GPIO_BRR.PIN（SDK
//! `GPIO_PxNN_SET`/`GPIO_PxNN_RESET` 同做法）；v2 没有这对寄存器，退化
//! 为 PxOUT 读-改-写。输入电平一律从 PxIN/GPIO_IN 采样寄存器读，输出
//! 锁存（StatefulOutputPin 的 `is_set_high` 等）一律从 PxOUT/GPIO_OUT
//! 读回，各布局一致。
//!
//! 门控差异：v2/v3_3 的全部端口汇集在单一 GPIO 外设内共用一个门控位；
//! v3_1 每个端口是独立实例（GPIOA~GPIOD）、独立门控位。公共 API 经
//! 后端的 `enable_port` 打开引脚所在端口的门控，单外设布局退化为统一
//! 门控，行为不变。上/下拉（[`Pull`]）也交由后端一次写入：v2/v3_3 是
//! 独立的 PxPU/PxPD 位对，v3_1 合并在 GPIO_PUPD 一个寄存器内。
//!
//! [`gpio_v2`]: self::gpio_v2
//! [`gpio_v3_3`]: self::gpio_v3_3
//! [`gpio_v3_1`]: self::gpio_v3_1

#[cfg(gpio_ddl_v2)]
mod gpio_v2;
#[cfg(gpio_ddl_v2)]
use gpio_v2 as backend;
#[cfg(gpio_ddl_v2)]
// 后端原语对 clock 模块可见（晶振引脚模拟模式与波形输出引脚都是 GPIO
// 寄存器操作，经此入口转发，不重复实现）。
pub(crate) use gpio_v2::{
    disable_hclk_output_p24 as backend_disable_hclk_p24,
    disable_pclk_output_p31 as backend_disable_pclk_p31,
    enable_hclk_output_p24 as backend_enable_hclk_p24,
    enable_pclk_output_p31 as backend_enable_pclk_p31, setup_xth_pins as backend_setup_xth_pins,
    setup_xtl_pins as backend_setup_xtl_pins,
};
// 16 引脚封装未引出脚上拉：仅由适用的 L110 小封装料号带出（DS 第 4 章
// 表下注），[`crate::init`] 调用。
#[cfg(all(gpio_ddl_v2, feature = "hc32l110-16pin"))]
pub(crate) use gpio_v2::pull_up_unbonded as backend_pull_up_unbonded;

#[cfg(gpio_ddl_v3_3)]
mod gpio_v3_3;
#[cfg(gpio_ddl_v3_3)]
use gpio_v3_3 as backend;
#[cfg(gpio_ddl_v3_3)]
pub(crate) use gpio_v3_3::{
    disable_hclk_output_pa01 as backend_disable_hclk_pa01,
    disable_pclk_output_pa02 as backend_disable_pclk_pa02,
    enable_hclk_output_pa01 as backend_enable_hclk_pa01,
    enable_pclk_output_pa02 as backend_enable_pclk_pa02, setup_xth_pins as backend_setup_xth_pins,
    setup_xtl_pins as backend_setup_xtl_pins,
};
// HC32L13x 各封装未引出脚处理（SDK `_HidePinInit` 同源），仅由带
// hc32l13x-*pin 封装 feature 的料号编译，[`crate::init`] 调用。
#[cfg(all(gpio_ddl_v3_3, feature = "hc32l13x"))]
pub(crate) use gpio_v3_3::pull_up_unbonded as backend_pull_up_unbonded;

#[cfg(gpio_ddl_v3_1)]
mod gpio_v3_1;
#[cfg(gpio_ddl_v3_1)]
use gpio_v3_1 as backend;
#[cfg(gpio_ddl_v3_1)]
pub(crate) use gpio_v3_1::setup_xth_pins as backend_setup_xth_pins;

use core::convert::Infallible;
use core::marker::PhantomData;

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
/// 上拉典型值约 80 kΩ、下拉典型值约 40 kΩ（VCC = 3.3 V；L110 DS
/// 7.3.12、L13x DS Rev2.20 7.3.13.2、F420 DS Rev1.20 表 6-19，各侧
/// 规格一致）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Pull {
    /// 无上拉/下拉。
    #[default]
    None,
    /// 上拉（PxPU / GPIO_PUPD.PU = 1）。
    Up,
    /// 下拉（PxPD / GPIO_PUPD.PD = 1）。
    Down,
}

/// 输出驱动能力档位（PxDR / GPIO_DRIVER）。
///
/// 复位默认为高驱动。电流规格：高驱动对应双驱规格（L13x 约 20 mA，
/// DS 表 7-8 VOHD/VOLD 行；F420 双驱见 DS Rev1.20 表 6-18），低驱动
/// 对应单驱规格（L110 DS 7.3.12、L13x DS 7.3.13.1、F420 表 6-18）。
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Drive {
    /// 高驱动（PxDR / GPIO_DRIVER = 0），复位默认。
    #[default]
    High,
    /// 低驱动（PxDR / GPIO_DRIVER = 1）。
    Low,
}

/// 引脚身份：端口 + 端口内位号。
///
/// 独立表达两个字段，不压缩成单个编号（`port * 8 + pin` 在 16 位端口上
/// 会让 PA15 与 PB07 冲突）。
#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) struct PinId {
    /// 端口号。
    pub(crate) port: u8,
    /// 端口内位号。
    pub(crate) pin: u8,
}

impl PinId {
    pub(crate) const fn new(port: u8, pin: u8) -> Self {
        Self { port, pin }
    }
}

/// 单例引脚的私有能力。
pub(crate) trait SealedPin {
    fn pin_id(&self) -> PinId;
}

/// GPIO 引脚。
///
/// 由 [`crate::init`] 发放的引脚单例（`peripherals::P01` 等）实现本 trait。
#[allow(private_bounds)]
pub trait Pin: crate::peripheral::PeripheralType + SealedPin + Sized + 'static {
    /// 端口号（如 P14 → 1）。
    fn port(&self) -> u8 {
        self.pin_id().port
    }

    /// 端口内位号（如 P14 → 4）。
    fn pin(&self) -> u8 {
        self.pin_id().pin
    }
}

/// 把引脚切到数字复用功能（Pxy_SEL = af，PxADS = 0）。
///
/// `input` 选择方向：接收脚（RXD 等）为输入，发送脚（TXD 等）为输出。
/// 通信外设驱动按其引脚 trait 提供的复用编号调用；GPIO 驱动本身只用
/// `af = 0` 回到 GPIO 功能。
///
/// 先打开引脚所在端口的 GPIO 门控（L110 RM 3.5.8 / L13x RM 3.5.9 的
/// PERI_CLKEN 位 28 复位为 0，F420 为 SYSCTRL_AHBEN1 按端口位 0~3，
/// RM 3.5.16；门控关闭时对 GPIO 寄存器的写入不生效），幂等。
///
/// SDK 复用宏的固定动作序列：ADS 清零 → SEL 写功能号 → DIR 按信号方向
/// （L110 driver/inc/gpio.h 的 `Gpio_SetFunc_*` 系列；L13x 对应
/// `Gpio_SetAfMode` 直写 Pxy_SEL）。上/下拉经后端一次写入目标态。
#[allow(dead_code)] // L13x 侧由后续接入的通信驱动（UART 等）调用
pub(crate) fn set_pin_af(n: PinId, af: u8, input: bool, pull: Pull) {
    backend::enable_port(n.port);
    backend::set_ads(n, false);
    backend::set_dr(n, false);
    backend::set_od(n, false);
    backend::set_pull(n, pull);
    backend::set_dir(n, input);
    backend::set_sel(n, af);
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
    pub(crate) pin: PinId,
    _phantom: PhantomData<&'d mut ()>,
}

impl<'d> Flex<'d> {
    /// 接管一个引脚单例，不改变其当前配置。
    pub fn new(pin: Peri<'d, impl Pin>) -> Self {
        // 引脚所在端口的 GPIO 门控复位后为关（L110 RM 3.5.8 位 28 /
        // L13x RM 3.5.9 位 28），先打开；多端口实例布局按端口门控
        // （后端 enable_port）。
        backend::enable_port(pin.pin_id().port);
        Self {
            pin: pin.pin_id(),
            _phantom: PhantomData,
        }
    }

    /// 配置为数字输入。
    ///
    /// SEL 置 0（GPIO 功能）、ADS 置 0（数字）、DR/OD 复位，方向置输入，
    /// 再按 `pull` 经后端一次配置上/下拉。
    pub fn set_as_input(&mut self, pull: Pull) {
        let n = self.pin;
        backend::set_sel(n, 0);
        backend::set_ads(n, false);
        backend::set_dr(n, false);
        backend::set_od(n, false);
        backend::set_pull(n, pull);
        backend::set_dir(n, true);
    }

    /// 配置为推挽输出，驱动能力 `drive`。
    pub fn set_as_output(&mut self, drive: Drive) {
        let n = self.pin;
        backend::set_sel(n, 0);
        backend::set_ads(n, false);
        backend::set_pull(n, Pull::None);
        backend::set_od(n, false);
        backend::set_dr(n, matches!(drive, Drive::Low));
        backend::set_dir(n, false);
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
        backend::set_sel(n, 0);
        backend::set_ads(n, false);
        backend::set_dr(n, matches!(drive, Drive::Low));
        backend::set_pull(n, pull);
        backend::set_od(n, true);
        backend::set_dir(n, false);
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
        backend::set_sel(n, 0);
        backend::set_dir(n, true);
        backend::set_pull(n, Pull::None);
        backend::set_od(n, false);
        backend::set_dr(n, false);
        backend::set_ads(n, true);
    }

    /// 断开引脚：模拟态（PxADS = 1），上下拉与开漏全部关闭。
    ///
    /// Drop 时自动调用。模拟态关闭数字输入缓冲，漏电流最小。
    pub fn set_as_disconnected(&mut self) {
        self.set_as_analog();
    }

    /// 读引脚输入电平（PxIN）。
    pub fn is_high(&self) -> bool {
        backend::read_in(self.pin)
    }

    /// 读引脚输入电平（PxIN）。
    pub fn is_low(&self) -> bool {
        !backend::read_in(self.pin)
    }

    /// 读引脚输入电平（PxIN）。
    pub fn level(&self) -> Level {
        Level::from(backend::read_in(self.pin))
    }

    /// 输出锁存置高（PxOUT = 1）。
    pub fn set_high(&mut self) {
        backend::set_out(self.pin, true);
    }

    /// 输出锁存置低（PxOUT = 0）。
    pub fn set_low(&mut self) {
        backend::set_out(self.pin, false);
    }

    /// 输出锁存写入指定电平（PxOUT）。
    pub fn set_level(&mut self, level: Level) {
        backend::set_out(self.pin, bool::from(level));
    }

    /// 读输出锁存（PxOUT）。
    pub fn is_set_high(&self) -> bool {
        backend::read_out(self.pin)
    }

    /// 读输出锁存（PxOUT）。
    pub fn is_set_low(&self) -> bool {
        !backend::read_out(self.pin)
    }

    /// 读输出锁存（PxOUT）。
    pub fn output_level(&self) -> Level {
        Level::from(backend::read_out(self.pin))
    }

    /// 翻转输出锁存（PxOUT）。
    pub fn toggle(&mut self) {
        let high = backend::read_out(self.pin);
        backend::set_out(self.pin, !high);
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
    /// 先写输出锁存再切方向，确保 DIR 置输出的瞬间锁存器里已是
    /// `initial_output`，不会出现初值毛刺。L110/L13x 的 SDK `Gpio_Init`
    /// 同序（OUT → DIR）；F420 SDK 的 `GPIO_Init` 不写 OUT，该写序见其
    /// 例程 `gpio_inout`（`GPIO_PB12_RESET()` 先于 `GPIOB_Init()`）。
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
        self.pin.toggle()
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
