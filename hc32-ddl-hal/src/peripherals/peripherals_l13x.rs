//! HC32L13x（HC32L130/HC32L136）引脚单例与外设集合。
//!
//! 结构与 [`super::peripherals_l110`] 一致：单一声明源（宏的 `pins` 段）
//! 派生引脚单例、`SealedPin::pin_id`（`(port, bit)`）与 `Peripherals`
//! 字段，未引出脚按封装 cfg 门控。引脚存在性以 DS Rev2.20 表 3-2 与
//! 3.1 封装图为准（PAC 元数据暂缺引脚表），发放集合在编译期与逐端口
//! 期望位掩码（逐位取自表 3-2，见文件尾部）比对，防止门控漂移。
//! 未引出脚的初始化（数字输入 + 上拉）由 [`crate::init`] 按封装
//! feature 执行，掩码见 `gpio::gpio_v3_3`（SDK `_HidePinInit` 同源）。
//!
//! 当前只有 GPIO 驱动适配了 HC32L13x 的寄存器布局（`gpio_ddl_v3_3`），
//! UART/定时器等实例的驱动后端是 L110 的版本（如 `uart/ddl_v2`、
//! `adt/ddl_v2_1`），未适配前不发放对应单例，`Peripherals` 暂无外设
//! 字段；驱动逐个接入后按实例 cfg 增加（AGENTS.md「从具体型号选择
//! 驱动」）。
//!
//! 逐个核对过的封装引脚差异（DS Rev2.20 表 3-2，四列 64/48/32/28；
//! 订购信息表 I/O 计数互证）：
//!
//! | 料号 | 封装 | 引出 IO | 未引出 |
//! | --- | --- | --- | --- |
//! | HC32L130E8PA | TSSOP28 | 23：PA00~PA14、PB00~PB02、PC14/PC15、PD00/PD01/PD03 | PA15、PB03~PB15、PC00~PC13、PD02、PD04~PD07 |
//! | HC32L130F8UA | QFN32 | 26：PA02、PA04~PA15、PB00/PB01/PB03~PB07/PB11、PC14/PC15、PD00/PD01/PD03 | PA00/PA01/PA03、PB02/PB08~PB10、PB12~PB15、PC00~PC13、PD02、PD04~PD07 |
//! | HC32L130J8TA/J8UA、HC32L136J8TA | LQFP48/QFN48 | 40：PA00~PA15、PB00~PB15、PC13~PC15、PD00/PD01/PD03/PD06/PD07 | PC00~PC12、PD02/PD04/PD05 |
//! | HC32L136K8TA | LQFP64 | 56：全部 IO（PA00~PA15、PB00~PB15、PC00~PC15、PD00~PD07） | 无 |
//!
//! 48 脚封装的引出集合按 DS 表 3-2 共用一个「48」列，L136 与 L130
//! 同封装一致（L136 仅在相同引脚上多出 LCD/OPA 复用，第 9 章订购信息
//! 表）；64 脚封装独有引出 PC00~PC12、PD02/PD04/PD05。
//!
//! PD03 与 BOOT0 共用引脚（复位时采样决定 ISP 编程模式，DS 3.3
//! 「模块信号说明」），仍按 GPIO 发放；PA13/PA14 复位后默认为 SWDIO/
//! SWCLK 调试引脚，作普通 GPIO 使用前需置 SYSCTRL1.SWD_USE_IO（RM
//! 3.5.2 位 8），时钟模块接入后开放该开关。RESETB 为复位输入，不作为
//! GPIO 暴露；PD02/PD04/PD05 仅 64 引脚封装引出，按 `hc32l13x-64pin`
//! 发放。

// 引脚单例沿用手册引脚名（PA00 等）。
#![allow(non_snake_case)]

use crate::peripheral::{Peri, PeripheralType};

macro_rules! l13x_peripherals {
    (
        pins {
            $(
                $(#[doc = $pin_doc:expr])*
                $(#[cfg($($pin_cfg:tt)*)])?
                $pin:ident = ($port:literal, $bit:literal)
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
        /// 引脚单例属于 GPIO 驱动（后端 `gpio_ddl_v3_3`），按封装 cfg
        /// 发放；HC32L13x 的其他外设驱动尚未适配，暂无实例字段。
        pub struct Peripherals {
            $(
                $(#[doc = $pin_doc])*
                $(#[cfg($($pin_cfg)*)])?
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
                        $( $(#[cfg($($pin_cfg)*)])? $pin: $pin::steal(), )*
                    }
                }
            }
        }

        /// 当前 cfg 组合下各端口（PA/PB/PC/PD 顺序）已发放引脚单例的
        /// 位掩码（编译期按引脚声明的 cfg 派生），供文件尾部的逐封装
        /// 引脚集合校验比对。
        const EMITTED_PIN_MASKS: [u16; 4] = {
            let mut masks = [0u16; 4];
            $(
                $(#[cfg($($pin_cfg)*)])?
                { masks[$port] |= 1u16 << $bit; }
            )*
            masks
        };
    };
}

// 封装差异以 `hc32l13x-28pin/32pin/48pin/64pin` 表达：
// - TSSOP28（E8PA）：未引出 PA15、PB03~PB15、PC00~PC13、PD02/PD04~PD07；
// - QFN32（F8UA）：未引出 PA00/PA01/PA03、PB02/PB08~PB10/PB12~PB15、
//   PC00~PC13、PD02/PD04~PD07；
// - LQFP48/QFN48（L130 J8TA/J8UA、L136 J8TA）：未引出 PC00~PC12、
//   PD02/PD04/PD05；
// - LQFP64（L136 K8TA）：全部引出，PC00~PC12、PD02/PD04/PD05 仅由
//   本封装发放。
l13x_peripherals! {
    pins {
        /// 端口 PA 引脚（TSSOP28、LQFP48/QFN48 与 LQFP64 引出，QFN32 未引出）。
        #[cfg(not(feature = "hc32l13x-32pin"))]
        PA00 = (0, 0),
        /// 端口 PA 引脚（TSSOP28、LQFP48/QFN48 与 LQFP64 引出，QFN32 未引出）。
        #[cfg(not(feature = "hc32l13x-32pin"))]
        PA01 = (0, 1),
        /// 端口 PA 引脚（各封装均引出）。
        PA02 = (0, 2),
        /// 端口 PA 引脚（TSSOP28、LQFP48/QFN48 与 LQFP64 引出，QFN32 未引出）。
        #[cfg(not(feature = "hc32l13x-32pin"))]
        PA03 = (0, 3),
        /// 端口 PA 引脚（各封装均引出）。
        PA04 = (0, 4),
        /// 端口 PA 引脚（各封装均引出）。
        PA05 = (0, 5),
        /// 端口 PA 引脚（各封装均引出）。
        PA06 = (0, 6),
        /// 端口 PA 引脚（各封装均引出）。
        PA07 = (0, 7),
        /// 端口 PA 引脚（各封装均引出）。
        PA08 = (0, 8),
        /// 端口 PA 引脚（各封装均引出）。
        PA09 = (0, 9),
        /// 端口 PA 引脚（各封装均引出）。
        PA10 = (0, 10),
        /// 端口 PA 引脚（各封装均引出）。
        PA11 = (0, 11),
        /// 端口 PA 引脚（各封装均引出）。
        PA12 = (0, 12),
        /// 端口 PA 引脚，SWDIO 调试复用（各封装均引出）。
        PA13 = (0, 13),
        /// 端口 PA 引脚，SWCLK 调试复用（各封装均引出）。
        PA14 = (0, 14),
        /// 端口 PA 引脚（QFN32、LQFP48/QFN48 与 LQFP64 引出，TSSOP28 未引出）。
        #[cfg(not(feature = "hc32l13x-28pin"))]
        PA15 = (0, 15),
        /// 端口 PB 引脚（各封装均引出）。
        PB00 = (1, 0),
        /// 端口 PB 引脚（各封装均引出）。
        PB01 = (1, 1),
        /// 端口 PB 引脚（TSSOP28、LQFP48/QFN48 与 LQFP64 引出，QFN32 未引出）。
        #[cfg(not(feature = "hc32l13x-32pin"))]
        PB02 = (1, 2),
        /// 端口 PB 引脚（QFN32、LQFP48/QFN48 与 LQFP64 引出，TSSOP28 未引出）。
        #[cfg(not(feature = "hc32l13x-28pin"))]
        PB03 = (1, 3),
        /// 端口 PB 引脚（QFN32、LQFP48/QFN48 与 LQFP64 引出，TSSOP28 未引出）。
        #[cfg(not(feature = "hc32l13x-28pin"))]
        PB04 = (1, 4),
        /// 端口 PB 引脚（QFN32、LQFP48/QFN48 与 LQFP64 引出，TSSOP28 未引出）。
        #[cfg(not(feature = "hc32l13x-28pin"))]
        PB05 = (1, 5),
        /// 端口 PB 引脚（QFN32、LQFP48/QFN48 与 LQFP64 引出，TSSOP28 未引出）。
        #[cfg(not(feature = "hc32l13x-28pin"))]
        PB06 = (1, 6),
        /// 端口 PB 引脚（QFN32、LQFP48/QFN48 与 LQFP64 引出，TSSOP28 未引出）。
        #[cfg(not(feature = "hc32l13x-28pin"))]
        PB07 = (1, 7),
        /// 端口 PB 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PB08 = (1, 8),
        /// 端口 PB 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PB09 = (1, 9),
        /// 端口 PB 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PB10 = (1, 10),
        /// 端口 PB 引脚（QFN32、LQFP48/QFN48 与 LQFP64 引出，TSSOP28 未引出）。
        #[cfg(not(feature = "hc32l13x-28pin"))]
        PB11 = (1, 11),
        /// 端口 PB 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PB12 = (1, 12),
        /// 端口 PB 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PB13 = (1, 13),
        /// 端口 PB 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PB14 = (1, 14),
        /// 端口 PB 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PB15 = (1, 15),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC00 = (2, 0),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC01 = (2, 1),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC02 = (2, 2),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC03 = (2, 3),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC04 = (2, 4),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC05 = (2, 5),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC06 = (2, 6),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC07 = (2, 7),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC08 = (2, 8),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC09 = (2, 9),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC10 = (2, 10),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC11 = (2, 11),
        /// 端口 PC 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PC12 = (2, 12),
        /// 端口 PC 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PC13 = (2, 13),
        /// 端口 PC 引脚，XTLI 晶振复用（各封装均引出）。
        PC14 = (2, 14),
        /// 端口 PC 引脚，XTLO 晶振复用（各封装均引出）。
        PC15 = (2, 15),
        /// 端口 PD 引脚，XTHI 晶振复用（各封装均引出）。
        PD00 = (3, 0),
        /// 端口 PD 引脚，XTHO 晶振复用（各封装均引出）。
        PD01 = (3, 1),
        /// 端口 PD 引脚，BOOT0 复用（各封装均引出；复位时采样选择 ISP
        /// 编程模式，作普通 GPIO 前注意其复位态）。
        PD03 = (3, 3),
        /// 端口 PD 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PD02 = (3, 2),
        /// 端口 PD 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PD04 = (3, 4),
        /// 端口 PD 引脚（仅 LQFP64 引出）。
        #[cfg(feature = "hc32l13x-64pin")]
        PD05 = (3, 5),
        /// 端口 PD 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PD06 = (3, 6),
        /// 端口 PD 引脚（LQFP48/QFN48 与 LQFP64 引出）。
        #[cfg(any(feature = "hc32l13x-48pin", feature = "hc32l13x-64pin"))]
        PD07 = (3, 7),
    }
}

/// RESETB 为复位输入，不作为 GPIO 暴露；PD02/PD04/PD05 仅 64 引脚封装
/// 引出，见上表。

// 逐封装引脚集合校验：期望发放的位掩码（按 [PA, PB, PC, PD] 排列），
// 逐位取自 DS Rev2.20 表 3-2 对应封装列（列内有引脚名的位为 1，空白
// 列为 0；28 脚 PB 掩码 0x0007、32 脚 PA/PB 掩码 0xFFF4/0x08FB 与 SDK
// `_HidePinInit` 的 ADS 保留掩码一致互证）。掩码是声明集合之外的独立
// 校验源：增删引脚单例时必须同步更新，否则编译报错。
const EXPECTED_PIN_MASKS: [u16; 4] = if cfg!(feature = "hc32l13x-28pin") {
    // TSSOP28（E8PA）：PA00~PA14、PB00~PB02、PC14/PC15、PD00/PD01/PD03。
    [0x7FFF, 0x0007, 0xC000, 0x000B]
} else if cfg!(feature = "hc32l13x-32pin") {
    // QFN32（F8UA）：PA02/PA04~PA15、PB00/PB01/PB03~PB07/PB11、
    // PC14/PC15、PD00/PD01/PD03。
    [0xFFF4, 0x08FB, 0xC000, 0x000B]
} else if cfg!(feature = "hc32l13x-48pin") {
    // LQFP48/QFN48（L130 J8TA/J8UA、L136 J8TA）：PA00~PA15、
    // PB00~PB15、PC13~PC15、PD00/PD01/PD03/PD06/PD07。
    [0xFFFF, 0xFFFF, 0xE000, 0x00CB]
} else if cfg!(feature = "hc32l13x-64pin") {
    // LQFP64（L136 K8TA）：全部 56 个 IO（PA00~PA15、PB00~PB15、
    // PC00~PC15、PD00~PD07）。
    [0xFFFF, 0xFFFF, 0xFFFF, 0x00FF]
} else {
    // 未选择封装 feature：期望集合为空，与常驻引脚必然不符，由下方
    // 校验报错。
    [0; 4]
};

const _: () = assert!(
    EMITTED_PIN_MASKS[0] == EXPECTED_PIN_MASKS[0],
    "hc32-ddl-hal: HC32L13x PA 引脚单例集合与当前封装 DS Rev2.20 表 3-2 的引出集合不符，请核对 peripherals_l13x.rs 的 cfg 门控",
);
const _: () = assert!(
    EMITTED_PIN_MASKS[1] == EXPECTED_PIN_MASKS[1],
    "hc32-ddl-hal: HC32L13x PB 引脚单例集合与当前封装 DS Rev2.20 表 3-2 的引出集合不符，请核对 peripherals_l13x.rs 的 cfg 门控",
);
const _: () = assert!(
    EMITTED_PIN_MASKS[2] == EXPECTED_PIN_MASKS[2],
    "hc32-ddl-hal: HC32L13x PC 引脚单例集合与当前封装 DS Rev2.20 表 3-2 的引出集合不符，请核对 peripherals_l13x.rs 的 cfg 门控",
);
const _: () = assert!(
    EMITTED_PIN_MASKS[3] == EXPECTED_PIN_MASKS[3],
    "hc32-ddl-hal: HC32L13x PD 引脚单例集合与当前封装 DS Rev2.20 表 3-2 的引出集合不符，请核对 peripherals_l13x.rs 的 cfg 门控",
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
