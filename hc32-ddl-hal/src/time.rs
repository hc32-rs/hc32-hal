//! 频率与分频基础类型。
//!
//! 仿照 Embassy STM32 HAL 的 `time` 模块：`Hertz` 表达频率，
//! 实现 `Prescaler` 的分频枚举可以直接对 `Hertz` 做除法，
//! 例如 `sys / HclkPrescaler::Div8`。

use core::fmt;
use core::ops::{Div, Mul};

/// 频率，单位 Hz。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Hertz(pub u32);

impl Hertz {
    /// 以 Hz 构造。
    pub const fn hz(hz: u32) -> Self {
        Self(hz)
    }

    /// 以 kHz 构造。
    pub const fn khz(khz: u32) -> Self {
        Self(khz * 1_000)
    }

    /// 以 MHz 构造。
    pub const fn mhz(mhz: u32) -> Self {
        Self(mhz * 1_000_000)
    }
}

impl From<u32> for Hertz {
    fn from(hz: u32) -> Self {
        Self(hz)
    }
}

impl fmt::Display for Hertz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}

impl fmt::Debug for Hertz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}

impl Div<u32> for Hertz {
    type Output = Hertz;

    fn div(self, rhs: u32) -> Self::Output {
        Hertz(self.0 / rhs)
    }
}

impl Mul<u32> for Hertz {
    type Output = Hertz;

    fn mul(self, rhs: u32) -> Self::Output {
        Hertz(self.0 * rhs)
    }
}

impl Div<Hertz> for Hertz {
    type Output = u32;

    fn div(self, rhs: Hertz) -> Self::Output {
        self.0 / rhs.0
    }
}

/// 分频系数：`denom / num` 为实际分频比。
///
/// 实现该 trait 的分频枚举可用于 `Hertz / prescaler`。
pub trait Prescaler {
    /// 分子。
    fn num(&self) -> u32;

    /// 分母。
    fn denom(&self) -> u32;
}

impl<P: Prescaler> Div<P> for Hertz {
    type Output = Hertz;

    fn div(self, rhs: P) -> Self::Output {
        Hertz(self.0 * rhs.num() / rhs.denom())
    }
}

impl<P: Prescaler> Mul<P> for Hertz {
    type Output = Hertz;

    fn mul(self, rhs: P) -> Self::Output {
        Hertz(self.0 * rhs.denom() / rhs.num())
    }
}
