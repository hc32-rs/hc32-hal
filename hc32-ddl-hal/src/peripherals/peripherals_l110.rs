//! HC32L110 引脚单例与外设集合。
//!
//! 每个物理引脚一个零大小单例类型，由 [`crate::init`] 一次性发放。
//! 引脚声明集中为一份（[`l110_peripherals!`] 宏的 `pins` 段），单例类型、
//! `Peripherals` 字段、构造与端口位号都由它派生，未引出脚以封装 cfg
//! 门控；引脚的存在性以封装引脚表为准（DS 第 4 章，PAC 元数据暂缺引脚
//! 表）。端口位号以 `(port, bit)` 二元组表达，无歧义区分 PA15 与 PB07
//! 一类跨端口引脚（AGENTS.md「从具体型号选择驱动」）。
//!
//! 外设字段由 build.rs 从 PAC 元数据派生的实例 cfg（`peri_<实例>`）门控：
//! 芯片拥有该实例才发放对应单例。
//!
//! 逐个核对过的封装引脚差异（DS 2.2/第 4 章引脚表）：
//!
//! | 料号 | 封装 | 引脚数 | 未引出 |
//! | --- | --- | --- | --- |
//! | HC32L110B4PA/B6PA | TSSOP16 | 16 | P03/P32/P33/P34 |
//! | HC32L110B6YA | CSP16 | 16 | P03/P32/P33/P34 |
//! | HC32L110C4PA/C4UA/C6PA/C6UA | TSSOP20/QFN20 | 20 | — |

// 引脚单例沿用手册引脚名（P14 等）。
#![allow(non_snake_case)]

use crate::peripheral::{Peri, PeripheralType};

/// 生成 HC32L110 的引脚单例与外设集合。
///
/// 单一声明源（`pins` 段）派生四类产物：
///
/// - 引脚单例类型与 `steal`（按封装 cfg 门控）；
/// - `SealedPin::pin_id`（`(port, bit)` 声明直接写入，无编号换算）；
/// - `Peripherals` 的引脚字段；
/// - `Peripherals::steal` 的引脚构造。
///
/// `instances` 段的每个条目声明一个外设字段：类型、门控 cfg 与构造
/// 表达式，与引脚共用同一 `Peripherals` 定义。
macro_rules! l110_peripherals {
    (
        pins {
            $(
                $(#[doc = $pin_doc:expr])*
                $(#[cfg($($pin_cfg:tt)*)])?
                $pin:ident = ($port:literal, $bit:literal)
            ),* $(,)?
        }
        instances {
            $(
                $(#[doc = $inst_doc:expr])*
                $(#[cfg($($inst_cfg:tt)*)])?
                $inst:ident : $inst_ty:ty => $inst_steal:expr
            ),* $(,)?
        }
    ) => {
        $(
            $(#[doc = $pin_doc])*
            $(#[cfg($($pin_cfg)*)])?
            #[allow(non_camel_case_types)]
            #[derive(Copy, Clone)]
            pub struct $pin { _private: () }

            $(#[doc = $pin_doc])*
            $(#[cfg($($pin_cfg)*)])?
            impl $pin {
                /// 直接取得该引脚，绕过 [`crate::init`] 的独占检查。
                ///
                /// # Safety
                ///
                /// 调用方须保证同一引脚不被两处同时持有。
                pub const unsafe fn steal() -> Peri<'static, Self> {
                    unsafe { Peri::new_unchecked(Self { _private: () }) }
                }
            }

            $(#[cfg($($pin_cfg)*)])?
            impl PeripheralType for $pin {}

            $(#[cfg($($pin_cfg)*)])?
            impl crate::gpio::SealedPin for $pin {
                fn pin_id(&self) -> crate::gpio::PinId {
                    crate::gpio::PinId::new($port, $bit)
                }
            }

            $(#[cfg($($pin_cfg)*)])?
            impl crate::gpio::Pin for $pin {}
        )*

        /// 外设集合：每字段是对应单例的独占引用。
        ///
        /// 引脚单例属于 GPIO 驱动（后端 `gpio_ddl_v2`）；定时器、串口等
        /// 字段由实例 cfg `peri_<实例>`（build.rs 从 PAC 元数据派生）控制，
        /// 后续型号按其实例清单自动增减。
        pub struct Peripherals {
            $(
                $(#[doc = $pin_doc])*
                $(#[cfg($($pin_cfg)*)])?
                pub $pin: Peri<'static, $pin>,
            )*
            $(
                $(#[doc = $inst_doc])*
                $(#[cfg($($inst_cfg)*)])?
                pub $inst: $inst_ty,
            )*
        }

        impl Peripherals {
            /// 不经独占检查构造外设集合。
            ///
            /// # Safety
            ///
            /// 调用方须保证所有单例不被两处同时持有。
            pub unsafe fn steal() -> Self {
                unsafe {
                    Self {
                        $( $(#[cfg($($pin_cfg)*)])? $pin: $pin::steal(), )*
                        $( $(#[cfg($($inst_cfg)*)])? $inst: $inst_steal, )*
                    }
                }
            }
        }
    };
}

l110_peripherals! {
    pins {
        /// 端口 P0 引脚（P00/RESETB 除外，见模块文档末尾）。
        P01 = (0, 1),
        /// 端口 P0 引脚。
        P02 = (0, 2),
        /// 仅 20 引脚封装（TSSOP20/QFN20，C 系列）引出。
        #[cfg(feature = "hc32l110-full-pins")]
        P03 = (0, 3),
        /// 端口 P1 引脚。
        P14 = (1, 4),
        /// 端口 P1 引脚。
        P15 = (1, 5),
        /// 端口 P2 引脚。
        P23 = (2, 3),
        /// 端口 P2 引脚。
        P24 = (2, 4),
        /// 端口 P2 引脚。
        P25 = (2, 5),
        /// 端口 P2 引脚。
        P26 = (2, 6),
        /// 端口 P2 引脚。
        P27 = (2, 7),
        /// 端口 P3 引脚。
        P31 = (3, 1),
        /// 仅 20 引脚封装（C 系列）引出。
        #[cfg(feature = "hc32l110-full-pins")]
        P32 = (3, 2),
        /// 仅 20 引脚封装（C 系列）引出。
        #[cfg(feature = "hc32l110-full-pins")]
        P33 = (3, 3),
        /// 仅 20 引脚封装（C 系列）引出。
        #[cfg(feature = "hc32l110-full-pins")]
        P34 = (3, 4),
        /// 端口 P3 引脚。
        P35 = (3, 5),
        /// 端口 P3 引脚。
        P36 = (3, 6),
    }
    instances {
        /// 基本定时器 TIM0。
        #[cfg(all(bt_ddl_v1, peri_bt0))]
        BT0: Peri<'static, crate::bt::Bt0> => crate::bt::Bt0::steal(),
        /// 基本定时器 TIM1。
        #[cfg(all(bt_ddl_v1, peri_bt1))]
        BT1: Peri<'static, crate::bt::Bt1> => crate::bt::Bt1::steal(),
        /// 基本定时器 TIM2。
        #[cfg(all(bt_ddl_v1, peri_bt2))]
        BT2: Peri<'static, crate::bt::Bt2> => crate::bt::Bt2::steal(),
        /// 低功耗定时器。
        #[cfg(all(lptim_ddl_v1, peri_lptimer))]
        LPTIM: Peri<'static, crate::lptim::LpTim> => crate::lptim::LpTim::steal(),
        /// 可编程计数阵列。
        #[cfg(all(pca_ddl_v1, peri_pca))]
        PCA: Peri<'static, crate::pca::PcaPeri> => crate::pca::PcaPeri::steal(),
        /// 高级定时器 TIM4。
        #[cfg(all(adt_ddl_v1, peri_adt0))]
        ADT0: Peri<'static, crate::adt::Adt0> => crate::adt::Adt0::steal(),
        /// 高级定时器 TIM5。
        #[cfg(all(adt_ddl_v1, peri_adt1))]
        ADT1: Peri<'static, crate::adt::Adt1> => crate::adt::Adt1::steal(),
        /// 高级定时器 TIM6。
        #[cfg(all(adt_ddl_v1, peri_adt2))]
        ADT2: Peri<'static, crate::adt::Adt2> => crate::adt::Adt2::steal(),
        /// UART0。
        #[cfg(all(uart_ddl_v1, peri_uart0))]
        UART0: Peri<'static, crate::uart::Uart0> => crate::uart::Uart0::steal(),
        /// UART1。
        #[cfg(all(uart_ddl_v1, peri_uart1))]
        UART1: Peri<'static, crate::uart::Uart1> => crate::uart::Uart1::steal(),
        /// 低功耗 UART。
        #[cfg(all(lpuart_ddl_v1, peri_lpuart))]
        LPUART: Peri<'static, crate::uart::LpUartPeri> => crate::uart::LpUartPeri::steal(),
    }
}

/// P00/RESETB 为复位输入，方向与上拉等控制位保留，暂不作为 GPIO 暴露。

impl Peripherals {
    /// 以安全方式取得全部单例，第二次调用 panic。
    ///
    /// 独占检查与占用经 [`super::claim_peripherals`] 在 PRIMASK 临界区
    /// 内完成；[`crate::init`] 在改动任何硬件前先领取，重复初始化不会
    /// 先触碰外设。
    pub(crate) fn take() -> Self {
        super::claim_peripherals();
        // safety: claim_peripherals 保证这是唯一一次构造。
        unsafe { Self::steal() }
    }
}
