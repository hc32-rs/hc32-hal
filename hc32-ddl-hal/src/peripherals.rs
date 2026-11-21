//! HC32L110 引脚单例与外设集合。
//!
//! 每个物理引脚一个零大小单例类型，由 [`crate::init`] 一次性发放；
//! 引脚的存在性以端口/位字段为准，不由 PAC 元数据生成（上游暂缺引脚表）。
//!
//! 外设字段由 build.rs 从 PAC 元数据派生的实例 cfg（`peri_<实例>`）门控：
//! 芯片拥有该实例才发放对应单例。

// 引脚单例沿用手册引脚名（P14 等）。
#![allow(non_snake_case)]

use core::sync::atomic::{AtomicBool, Ordering};

use crate::peripheral::PeripheralType;

/// 生成引脚单例类型及其 `steal`。
macro_rules! pin_singletons {
    ($( $(#[$cfg:meta])* $name:ident ),* $(,)?) => {
        $(
            $(#[$cfg])*
            #[allow(non_camel_case_types)]
            #[derive(Copy, Clone)]
            pub struct $name { _private: () }

            $(#[$cfg])*
            impl $name {
                /// 直接取得该引脚，绕过 [`crate::init`] 的独占检查。
                ///
                /// # Safety
                ///
                /// 调用方须保证同一引脚不被两处同时持有。
                pub const unsafe fn steal() -> Peri<'static, Self> {
                    unsafe { Peri::new_unchecked(Self { _private: () }) }
                }
            }

            $(#[$cfg])*
            impl PeripheralType for $name {}

            $(#[$cfg])*
            impl crate::gpio::SealedPin for $name {
                fn pin_number(&self) -> crate::gpio::PinNumber {
                    crate::gpio::PinNumber::new(pin_singletons!(@num $name))
                }
            }

            $(#[$cfg])*
            impl crate::gpio::Pin for $name {}
        )*
    };
    // 引脚名 -> port*8+pin 编码，由宏展开时静态求值。
    (@num P01) => { 0 * 8 + 1 };
    (@num P02) => { 0 * 8 + 2 };
    (@num P03) => { 0 * 8 + 3 };
    (@num P14) => { 1 * 8 + 4 };
    (@num P15) => { 1 * 8 + 5 };
    (@num P23) => { 2 * 8 + 3 };
    (@num P24) => { 2 * 8 + 4 };
    (@num P25) => { 2 * 8 + 5 };
    (@num P26) => { 2 * 8 + 6 };
    (@num P27) => { 2 * 8 + 7 };
    (@num P31) => { 3 * 8 + 1 };
    (@num P32) => { 3 * 8 + 2 };
    (@num P33) => { 3 * 8 + 3 };
    (@num P34) => { 3 * 8 + 4 };
    (@num P35) => { 3 * 8 + 5 };
    (@num P36) => { 3 * 8 + 6 };
}

use crate::peripheral::Peri;

pin_singletons!(
    P01, P02,
    /// 仅 20 引脚封装（C 系列）引出。
    #[cfg(feature = "hc32l110-full-pins")]
    P03,
    P14, P15,
    P23, P24, P25, P26, P27,
    P31,
    /// 仅 20 引脚封装（C 系列）引出。
    #[cfg(feature = "hc32l110-full-pins")]
    P32,
    /// 仅 20 引脚封装（C 系列）引出。
    #[cfg(feature = "hc32l110-full-pins")]
    P33,
    /// 仅 20 引脚封装（C 系列）引出。
    #[cfg(feature = "hc32l110-full-pins")]
    P34,
    P35, P36,
);

/// P00/RESETB 为复位输入，方向与上拉等控制位保留，暂不作为 GPIO 暴露。

/// 外设集合：每字段是对应单例的独占引用。
///
/// 引脚单例属于 GPIO 驱动（后端 `gpio_ddl_v2`）；定时器、串口等字段
/// 由实例 cfg `peri_<实例>`（build.rs 从 PAC 元数据派生）控制，后续
/// 型号按其实例清单自动增减。
pub struct Peripherals {
    /// 端口 P0 引脚（P00/RESETB 除外，见模块文档）。
    #[cfg(feature = "hc32l110-full-pins")]
    pub P03: Peri<'static, P03>,
    pub P01: Peri<'static, P01>,
    pub P02: Peri<'static, P02>,
    /// 端口 P1 引脚。
    pub P14: Peri<'static, P14>,
    pub P15: Peri<'static, P15>,
    /// 端口 P2 引脚。
    pub P23: Peri<'static, P23>,
    pub P24: Peri<'static, P24>,
    pub P25: Peri<'static, P25>,
    pub P26: Peri<'static, P26>,
    pub P27: Peri<'static, P27>,
    /// 端口 P3 引脚。
    pub P31: Peri<'static, P31>,
    #[cfg(feature = "hc32l110-full-pins")]
    pub P32: Peri<'static, P32>,
    #[cfg(feature = "hc32l110-full-pins")]
    pub P33: Peri<'static, P33>,
    #[cfg(feature = "hc32l110-full-pins")]
    pub P34: Peri<'static, P34>,
    pub P35: Peri<'static, P35>,
    pub P36: Peri<'static, P36>,
    /// 基本定时器 TIM0。
    #[cfg(all(bt_ddl_v1, peri_bt0))]
    pub BT0: Peri<'static, crate::bt::Bt0>,
    /// 基本定时器 TIM1。
    #[cfg(all(bt_ddl_v1, peri_bt1))]
    pub BT1: Peri<'static, crate::bt::Bt1>,
    /// 基本定时器 TIM2。
    #[cfg(all(bt_ddl_v1, peri_bt2))]
    pub BT2: Peri<'static, crate::bt::Bt2>,
    /// 低功耗定时器。
    #[cfg(all(lptim_ddl_v1, peri_lptimer))]
    pub LPTIM: Peri<'static, crate::lptim::LpTim>,
    /// 可编程计数阵列。
    #[cfg(all(pca_ddl_v1, peri_pca))]
    pub PCA: Peri<'static, crate::pca::PcaPeri>,
    /// 高级定时器 TIM4。
    #[cfg(all(adt_ddl_v1, peri_adt0))]
    pub ADT0: Peri<'static, crate::adt::Adt0>,
    /// 高级定时器 TIM5。
    #[cfg(all(adt_ddl_v1, peri_adt1))]
    pub ADT1: Peri<'static, crate::adt::Adt1>,
    /// 高级定时器 TIM6。
    #[cfg(all(adt_ddl_v1, peri_adt2))]
    pub ADT2: Peri<'static, crate::adt::Adt2>,
    /// UART0。
    #[cfg(all(uart_ddl_v1, peri_uart0))]
    pub UART0: Peri<'static, crate::uart::Uart0>,
    /// UART1。
    #[cfg(all(uart_ddl_v1, peri_uart1))]
    pub UART1: Peri<'static, crate::uart::Uart1>,
    /// 低功耗 UART。
    #[cfg(all(lpuart_ddl_v1, peri_lpuart))]
    pub LPUART: Peri<'static, crate::uart::LpUartPeri>,
}

static TAKEN: AtomicBool = AtomicBool::new(false);

impl Peripherals {
    /// 以安全方式取得全部单例，第二次调用 panic。
    ///
    /// Cortex-M0+ 没有原子读-改-写指令，用 load+store 检查；[`crate::init`]
    /// 属于启动期代码，不与中断并发，该检查足以防止重复初始化。
    pub(crate) fn take() -> Self {
        if TAKEN.load(Ordering::Relaxed) {
            panic!("hc32-ddl-hal: peripherals already taken");
        }
        TAKEN.store(true, Ordering::Relaxed);
        // safety: TAKEN 保证这是唯一一次构造。
        unsafe { Self::steal() }
    }

    /// 不经独占检查构造外设集合。
    ///
    /// # Safety
    ///
    /// 调用方须保证所有单例不被两处同时持有。
    pub unsafe fn steal() -> Self {
        unsafe {
            Self {
                P01: P01::steal(),
                P02: P02::steal(),
                #[cfg(feature = "hc32l110-full-pins")]
                P03: P03::steal(),
                P14: P14::steal(),
                P15: P15::steal(),
                P23: P23::steal(),
                P24: P24::steal(),
                P25: P25::steal(),
                P26: P26::steal(),
                P27: P27::steal(),
                P31: P31::steal(),
                #[cfg(feature = "hc32l110-full-pins")]
                P32: P32::steal(),
                #[cfg(feature = "hc32l110-full-pins")]
                P33: P33::steal(),
                #[cfg(feature = "hc32l110-full-pins")]
                P34: P34::steal(),
                P35: P35::steal(),
                P36: P36::steal(),
                #[cfg(all(bt_ddl_v1, peri_bt0))]
                BT0: crate::bt::Bt0::steal(),
                #[cfg(all(bt_ddl_v1, peri_bt1))]
                BT1: crate::bt::Bt1::steal(),
                #[cfg(all(bt_ddl_v1, peri_bt2))]
                BT2: crate::bt::Bt2::steal(),
                #[cfg(all(lptim_ddl_v1, peri_lptimer))]
                LPTIM: crate::lptim::LpTim::steal(),
                #[cfg(all(pca_ddl_v1, peri_pca))]
                PCA: crate::pca::PcaPeri::steal(),
                #[cfg(all(adt_ddl_v1, peri_adt0))]
                ADT0: crate::adt::Adt0::steal(),
                #[cfg(all(adt_ddl_v1, peri_adt1))]
                ADT1: crate::adt::Adt1::steal(),
                #[cfg(all(adt_ddl_v1, peri_adt2))]
                ADT2: crate::adt::Adt2::steal(),
                #[cfg(all(uart_ddl_v1, peri_uart0))]
                UART0: crate::uart::Uart0::steal(),
                #[cfg(all(uart_ddl_v1, peri_uart1))]
                UART1: crate::uart::Uart1::steal(),
                #[cfg(all(lpuart_ddl_v1, peri_lpuart))]
                LPUART: crate::uart::LpUartPeri::steal(),
            }
        }
    }
}
