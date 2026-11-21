//! 外设独占引用包装，裁剪自 embassy-hal-internal。
//!
//! [`Peri`] 在语义上等同于 `&'a mut T`，但按值存储单例外设类型（零大小类型不占空间），
//! 避免所有外设驱动按 `&mut T` 单态化出第二份代码。

use core::marker::PhantomData;
use core::ops::Deref;

/// 单例外设类型的标记 trait。
pub trait PeripheralType: Copy + Sized {}

/// 指向外设的独占引用。
///
/// 这是 `&'a mut T` 的零开销替身：
///
/// - 空间效率：单例外设类型通常零大小，`&mut T` 在 32 位目标上恒为 4 字节，
///   而 `Peri` 只存储 `T` 本身。
/// - 代码体积：驱动只对生命周期泛型，不对具体类型泛型，`P01` 与 `&mut P01`
///   不会单态化出两份驱动代码。
pub struct Peri<'a, T: PeripheralType> {
    inner: T,
    _lifetime: PhantomData<&'a mut ()>,
}

impl<'a, T: PeripheralType> Peri<'a, T> {
    /// 用单例类型构造 `Peri`。
    ///
    /// 仅供 HAL 内部使用；应用代码应通过 [`crate::init`] 取得的
    /// [`crate::Peripherals`] 字段或单例的 `steal()` 获取。
    #[inline]
    pub const unsafe fn new_unchecked(inner: T) -> Self {
        Self {
            inner,
            _lifetime: PhantomData,
        }
    }

    /// 不安全地克隆单例外设。
    ///
    /// # Safety
    ///
    /// 调用方必须保证同一时刻只有一份副本在用。优先使用 [`Self::reborrow`]，
    /// 由借用检查器静态保证这一点。
    pub const unsafe fn clone_unchecked(&self) -> Peri<'a, T> {
        // safety: 由本函数的 safety 契约保证。
        unsafe { Peri::new_unchecked(self.inner) }
    }

    /// 再借用出一份子 `Peri`。
    ///
    /// 子 `Peri` 存续期间借用 `self`，父引用随之冻结。
    pub const fn reborrow(&mut self) -> Peri<'_, T> {
        // safety: 返回值借用了 self，父引用在子 Peri 生存期内不可用。
        unsafe { Peri::new_unchecked(self.inner) }
    }

    /// 经由 `Into` 把内层单例映射为另一个外设类型。
    ///
    /// 例如把 `Peri<'a, P01>` 擦除为类型无关的引脚引用。
    #[inline]
    pub fn into<U>(self) -> Peri<'a, U>
    where
        T: Into<U>,
        U: PeripheralType,
    {
        unsafe { Peri::new_unchecked(self.inner.into()) }
    }
}

impl<'a, T: PeripheralType> Deref for Peri<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
