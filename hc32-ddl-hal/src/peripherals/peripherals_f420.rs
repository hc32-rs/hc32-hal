//! HC32F420 引脚单例与外设集合。
//!
//! 结构与 [`super::peripherals_l13x`] 一致：单一声明源（宏的 `pins` 段）
//! 派生引脚单例、`SealedPin::pin_id`（`(port, bit)`）与 `Peripherals`
//! 字段，发放集合在编译期与逐端口期望位掩码比对（逐位取自 DS Rev1.20
//! 表 4-1 的 LQFP64 列），防止门控漂移。引脚表迁入 PAC 元数据后由
//! 元数据按封装派生（AGENTS.md「从具体型号选择驱动」）。
//!
//! 当前只有 HC32F420KATB（LQFP64）：全部 52 个 IO 引出（PA00~PA15、
//! PB00~PB15、PC00~PC15、PD00~PD03，DS 3.7 通用 IO 端口：最多 52 个
//! GPIO），无未引出脚处理，PD 端口仅实现位 0~3（RM 表 6-5 PD 寄存器
//! 复位值 0x0000_000F）。
//! 其余 F420 封装（FAUB QFN32、JAUB/JATB 48 脚）引脚子集不同，逐个
//! 核对引脚表后再接入。
//!
//! 当前 F420 适配了 GPIO（`gpio/ddl_v3_1`）与 SYSCTRL 可配置时钟树
//! （`sysctrl/ddl_v4`）；USART/GTIM 等实例的驱动后端未适配，`Peripherals`
//! 暂无外设字段，驱动逐个接入后按实例 cfg 增加。
//!
//! 专用脚使用条件（DS Rev1.20 表 4-1、RM 6.3.3）：
//!
//! - NRESET（封装脚 7）：复位输入，固定上拉，不作为 GPIO 发放；
//! - PA13/PA14（SWDIO/SWCLK）：复位默认为调试端口（数字输入 + 内部
//!   上拉）；作普通 GPIO 前需置 SYSCTRL_CR1.SWDIO = 1（RM 3.5.3 位 0，
//!   受 CR3 序列保护），开关待时钟模块接入后开放；
//! - PB11（BOOT0）：复位时采样决定 ISP 编程模式；仍按 GPIO 发放，
//!   用作输出前注意其复位采样态（数字输入 + 内部下拉）；
//! - PD00/PD01（XTHI/XTHO）：外部晶振脚。接晶振时两脚都须配置为模拟；
//!   外部时钟直接注入时 PD00 保持数字输入、PD01 禁作 GPIO 且须配置为
//!   模拟（RM 3.1.4 与 CR2.EXXTH 位注）。

// 引脚单例沿用手册引脚名（PA00 等）。
#![allow(non_snake_case)]

use crate::peripheral::{Peri, PeripheralType};

macro_rules! f420_peripherals {
    (
        pins {
            $(
                $(#[doc = $pin_doc:expr])*
                $pin:ident = ($port:literal, $bit:literal)
            ),* $(,)?
        }
    ) => {
        $(
            $(#[doc = $pin_doc])*
            #[allow(non_camel_case_types)]
            #[derive(Copy, Clone)]
            pub struct $pin { _private: () }

            $(#[doc = $pin_doc])*
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

            impl PeripheralType for $pin {}

            // 与 gpio 驱动模块使用同一选择结果：GPIO 后端未开放时引脚
            // 不实现 Pin（build.rs 的 GPIO_DRIVER_BACKENDS 已按实例核对
            // GPIOA~GPIOD 集合）。
            #[cfg(gpio_ddl_v3_1)]
            impl crate::gpio::SealedPin for $pin {
                fn pin_id(&self) -> crate::gpio::PinId {
                    crate::gpio::PinId::new($port, $bit)
                }
            }

            #[cfg(gpio_ddl_v3_1)]
            impl crate::gpio::Pin for $pin {}
        )*

        /// 外设集合：每字段是对应单例的独占引用。
        ///
        /// 引脚单例属于 GPIO 驱动（后端 `gpio_ddl_v3_1`，一份实现服务
        /// GPIOA~GPIOD 四个端口实例）；HC32F420 的其他外设驱动尚未
        /// 适配，暂无实例字段。
        pub struct Peripherals {
            $(
                $(#[doc = $pin_doc])*
                pub $pin: Peri<'static, $pin>,
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
                        $( $pin: $pin::steal(), )*
                    }
                }
            }
        }

        /// 当前 cfg 组合下各端口（PA/PB/PC/PD 顺序）已发放引脚单例的
        /// 位掩码（编译期按引脚声明派生），供文件尾部的封装集合校验
        /// 比对。
        const EMITTED_PIN_MASKS: [u16; 4] = {
            let mut masks = [0u16; 4];
            $(
                { masks[$port] |= 1u16 << $bit; }
            )*
            masks
        };
    };
}

// LQFP64（KATB）：52 个 GPIO 全部引出（DS Rev1.20 表 4-1 的 LQFP64 列）。
f420_peripherals! {
    pins {
        /// 端口 PA 引脚（AIN0/VCIN0 模拟复用，数字功能见 DS 表 4-1）。
        PA00 = (0, 0),
        /// 端口 PA 引脚（MCO 复用）。
        PA01 = (0, 1),
        /// 端口 PA 引脚。
        PA02 = (0, 2),
        /// 端口 PA 引脚。
        PA03 = (0, 3),
        /// 端口 PA 引脚（PCLK0_OUT 复用）。
        PA04 = (0, 4),
        /// 端口 PA 引脚。
        PA05 = (0, 5),
        /// 端口 PA 引脚。
        PA06 = (0, 6),
        /// 端口 PA 引脚。
        PA07 = (0, 7),
        /// 端口 PA 引脚。
        PA08 = (0, 8),
        /// 端口 PA 引脚。
        PA09 = (0, 9),
        /// 端口 PA 引脚。
        PA10 = (0, 10),
        /// 端口 PA 引脚。
        PA11 = (0, 11),
        /// 端口 PA 引脚。
        PA12 = (0, 12),
        /// 端口 PA 引脚，SWDIO 调试复用（复位默认调试端口，见模块文档）。
        PA13 = (0, 13),
        /// 端口 PA 引脚，SWCLK 调试复用（复位默认调试端口，见模块文档）。
        PA14 = (0, 14),
        /// 端口 PA 引脚。
        PA15 = (0, 15),
        /// 端口 PB 引脚。
        PB00 = (1, 0),
        /// 端口 PB 引脚（PCLK1_OUT 复用）。
        PB01 = (1, 1),
        /// 端口 PB 引脚。
        PB02 = (1, 2),
        /// 端口 PB 引脚。
        PB03 = (1, 3),
        /// 端口 PB 引脚（PCLK0_OUT 复用）。
        PB04 = (1, 4),
        /// 端口 PB 引脚。
        PB05 = (1, 5),
        /// 端口 PB 引脚。
        PB06 = (1, 6),
        /// 端口 PB 引脚（PCLK1_OUT 复用）。
        PB07 = (1, 7),
        /// 端口 PB 引脚。
        PB08 = (1, 8),
        /// 端口 PB 引脚。
        PB09 = (1, 9),
        /// 端口 PB 引脚。
        PB10 = (1, 10),
        /// 端口 PB 引脚，BOOT0 复用（复位采样选择 ISP 模式，见模块文档）。
        PB11 = (1, 11),
        /// 端口 PB 引脚。
        PB12 = (1, 12),
        /// 端口 PB 引脚。
        PB13 = (1, 13),
        /// 端口 PB 引脚。
        PB14 = (1, 14),
        /// 端口 PB 引脚（MCO 复用）。
        PB15 = (1, 15),
        /// 端口 PC 引脚（AIN10 模拟复用）。
        PC00 = (2, 0),
        /// 端口 PC 引脚（AIN11 模拟复用）。
        PC01 = (2, 1),
        /// 端口 PC 引脚（AIN12 模拟复用）。
        PC02 = (2, 2),
        /// 端口 PC 引脚（AIN13 模拟复用）。
        PC03 = (2, 3),
        /// 端口 PC 引脚（AIN14 模拟复用）。
        PC04 = (2, 4),
        /// 端口 PC 引脚（AIN15 模拟复用）。
        PC05 = (2, 5),
        /// 端口 PC 引脚。
        PC06 = (2, 6),
        /// 端口 PC 引脚。
        PC07 = (2, 7),
        /// 端口 PC 引脚。
        PC08 = (2, 8),
        /// 端口 PC 引脚。
        PC09 = (2, 9),
        /// 端口 PC 引脚。
        PC10 = (2, 10),
        /// 端口 PC 引脚。
        PC11 = (2, 11),
        /// 端口 PC 引脚。
        PC12 = (2, 12),
        /// 端口 PC 引脚（LVDINO 模拟复用）。
        PC13 = (2, 13),
        /// 端口 PC 引脚。
        PC14 = (2, 14),
        /// 端口 PC 引脚。
        PC15 = (2, 15),
        /// 端口 PD 引脚，XTHI 晶振复用（外部时钟注入脚，见模块文档）。
        PD00 = (3, 0),
        /// 端口 PD 引脚，XTHO 晶振复用。
        PD01 = (3, 1),
        /// 端口 PD 引脚。
        PD02 = (3, 2),
        /// 端口 PD 引脚。
        PD03 = (3, 3),
    }
}

// 封装集合校验：期望发放的位掩码（按 [PA, PB, PC, PD] 排列），逐位
// 取自 DS Rev1.20 表 4-1 的 LQFP64 列（列内有引脚名的位为 1）。掩码
// 是声明集合之外的独立校验源：增删引脚单例时必须同步更新，否则编译
// 报错。
const EXPECTED_PIN_MASKS: [u16; 4] = [0xFFFF, 0xFFFF, 0xFFFF, 0x000F];

const _: () = assert!(
    EMITTED_PIN_MASKS[0] == EXPECTED_PIN_MASKS[0],
    "hc32-ddl-hal: HC32F420 PA 引脚单例集合与 LQFP64 封装 DS Rev1.20 表 4-1 的引出集合不符，请核对 peripherals_f420.rs 的引脚声明",
);
const _: () = assert!(
    EMITTED_PIN_MASKS[1] == EXPECTED_PIN_MASKS[1],
    "hc32-ddl-hal: HC32F420 PB 引脚单例集合与 LQFP64 封装 DS Rev1.20 表 4-1 的引出集合不符，请核对 peripherals_f420.rs 的引脚声明",
);
const _: () = assert!(
    EMITTED_PIN_MASKS[2] == EXPECTED_PIN_MASKS[2],
    "hc32-ddl-hal: HC32F420 PC 引脚单例集合与 LQFP64 封装 DS Rev1.20 表 4-1 的引出集合不符，请核对 peripherals_f420.rs 的引脚声明",
);
const _: () = assert!(
    EMITTED_PIN_MASKS[3] == EXPECTED_PIN_MASKS[3],
    "hc32-ddl-hal: HC32F420 PD 引脚单例集合与 LQFP64 封装 DS Rev1.20 表 4-1 的引出集合不符，请核对 peripherals_f420.rs 的引脚声明",
);

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
