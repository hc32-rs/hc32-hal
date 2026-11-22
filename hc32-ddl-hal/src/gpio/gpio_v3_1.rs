//! GPIO 寄存器后端：DDL v3_1 按端口实例布局（HC32F420）。
//!
//! 与 L110（ddl_v2）/L13x（ddl_v3_3）的"单外设多端口"不同，本布局每个
//! 端口是独立的 GPIO 实例（GPIOA~GPIOD，基地址 0x4002_4000 起 每端口
//! +0x400，RM 表 6-4），各自占用 SYSCTRL_AHBEN1 的独立门控位（RM
//! 3.5.16 位 0~3）。一份实现服务全部端口：[`PinId::port`] 选择 PAC 实例
//! 与门控位，位号选择寄存器位。
//!
//! 寄存器语义（RM 6.4，与 v2/v3_3 同义、仅组织不同）：
//!
//! - GPIO_ADS.PIN：1 = 模拟（复位值除调试/BOOT 脚外全模拟，RM 表 6-5）；
//! - GPIO_DIR.PIN：1 = 输入、0 = 输出（复位值全输入）；
//! - GPIO_PUPD 低半字 PU、高半字 PD（RM 6.4.6；PU 与 PD 同时为 1 时
//!   上拉生效），一次 Pull 配置在同一寄存器内单次读-改-写完成；
//! - GPIO_OPENDRAIN.PIN：1 = 开漏；GPIO_DRIVER.PIN：1 = 低驱动
//!   （RM 6.4.4/6.4.5，复位均 0）；
//! - 复用走 GPIO_AFRL/AFRH，每脚 4 位 AFSEL（0000 = GPIO，0001~0101 =
//!   AF1~AF5，RM 6.4.11/6.4.12）；
//! - 写路径用位操作寄存器：置位写 GPIO_BSRR.BS（b15~b0）、清零写
//!   GPIO_BRR.PIN（W1 触发，RM 6.4.9/6.4.10；SDK `GPIO_PxNN_SET`/
//!   `GPIO_PxNN_RESET` 同做法），不对 GPIO_OUT 做读-改-写（RM 6.3.5
//!   注意：非原子）；
//! - 输入采样读 GPIO_IN.PIN（RM 6.4.7，数据经 2 级同步，Tpd 1~2 个
//!   系统时钟）；输出锁存读回 GPIO_OUT.PIN（RM 6.4.8）。
//!
//! PD 端口仅实现 PIN0~PIN3（DS 3.7 通用 IO 端口：最多 52 个 GPIO =
//! PA/PB/PC 各 16 + PD 4；RM 表 6-5 PD 寄存器复位值 0x0000_000F），
//! 引脚单例集合保证驱动不会访问 PD 的更高位。

use super::{PinId, Pull};
use crate::pac;
use pac::gpio_ddl_v3_1::Gpio;

/// `port` → GPIO 实例（0 = GPIOA … 3 = GPIOD，RM 表 6-4 基地址）。
#[inline(always)]
fn gpio(port: u8) -> Gpio {
    match port {
        0 => pac::GPIOA,
        1 => pac::GPIOB,
        2 => pac::GPIOC,
        3 => pac::GPIOD,
        _ => unreachable!("GPIO v3_1 后端无此端口"),
    }
}

/// 打开 `port` 端口 GPIO 实例的时钟门控，幂等。
///
/// 本布局按端口独立门控（AHB1EN 位 0~3 = GPIOA~GPIOD，复位为关），
/// 经当前型号的系统控制器打开（clock 模块转发）。
pub(crate) fn enable_port(port: u8) {
    crate::clock::enable_gpio(port);
}

/// 数模选择位（GPIO_ADS.PIN）：`true` = 模拟（RM 6.4.2）。
#[inline(always)]
pub(crate) fn set_ads(n: PinId, analog: bool) {
    let g = gpio(n.port);
    match n.pin {
        0 => g.ads().modify(|w| w.set_pin0(analog)),
        1 => g.ads().modify(|w| w.set_pin1(analog)),
        2 => g.ads().modify(|w| w.set_pin2(analog)),
        3 => g.ads().modify(|w| w.set_pin3(analog)),
        4 => g.ads().modify(|w| w.set_pin4(analog)),
        5 => g.ads().modify(|w| w.set_pin5(analog)),
        6 => g.ads().modify(|w| w.set_pin6(analog)),
        7 => g.ads().modify(|w| w.set_pin7(analog)),
        8 => g.ads().modify(|w| w.set_pin8(analog)),
        9 => g.ads().modify(|w| w.set_pin9(analog)),
        10 => g.ads().modify(|w| w.set_pin10(analog)),
        11 => g.ads().modify(|w| w.set_pin11(analog)),
        12 => g.ads().modify(|w| w.set_pin12(analog)),
        13 => g.ads().modify(|w| w.set_pin13(analog)),
        14 => g.ads().modify(|w| w.set_pin14(analog)),
        15 => g.ads().modify(|w| w.set_pin15(analog)),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}

/// 方向位（GPIO_DIR.PIN）：`true` = 输入、`false` = 输出（RM 6.4.3）。
///
/// 复位值全输入（RM 表 6-5）；输出使能前应先写好输出锁存（公共
/// `Output::new` 的 OUT → DIR 次序）。
#[inline(always)]
pub(crate) fn set_dir(n: PinId, input: bool) {
    let g = gpio(n.port);
    match n.pin {
        0 => g.dir().modify(|w| w.set_pin0(input)),
        1 => g.dir().modify(|w| w.set_pin1(input)),
        2 => g.dir().modify(|w| w.set_pin2(input)),
        3 => g.dir().modify(|w| w.set_pin3(input)),
        4 => g.dir().modify(|w| w.set_pin4(input)),
        5 => g.dir().modify(|w| w.set_pin5(input)),
        6 => g.dir().modify(|w| w.set_pin6(input)),
        7 => g.dir().modify(|w| w.set_pin7(input)),
        8 => g.dir().modify(|w| w.set_pin8(input)),
        9 => g.dir().modify(|w| w.set_pin9(input)),
        10 => g.dir().modify(|w| w.set_pin10(input)),
        11 => g.dir().modify(|w| w.set_pin11(input)),
        12 => g.dir().modify(|w| w.set_pin12(input)),
        13 => g.dir().modify(|w| w.set_pin13(input)),
        14 => g.dir().modify(|w| w.set_pin14(input)),
        15 => g.dir().modify(|w| w.set_pin15(input)),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}

/// 开漏使能位（GPIO_OPENDRAIN.PIN）：`true` = 开漏（RM 6.4.4）。
#[inline(always)]
pub(crate) fn set_od(n: PinId, enable: bool) {
    let g = gpio(n.port);
    match n.pin {
        0 => g.opendrain().modify(|w| w.set_pin0(enable)),
        1 => g.opendrain().modify(|w| w.set_pin1(enable)),
        2 => g.opendrain().modify(|w| w.set_pin2(enable)),
        3 => g.opendrain().modify(|w| w.set_pin3(enable)),
        4 => g.opendrain().modify(|w| w.set_pin4(enable)),
        5 => g.opendrain().modify(|w| w.set_pin5(enable)),
        6 => g.opendrain().modify(|w| w.set_pin6(enable)),
        7 => g.opendrain().modify(|w| w.set_pin7(enable)),
        8 => g.opendrain().modify(|w| w.set_pin8(enable)),
        9 => g.opendrain().modify(|w| w.set_pin9(enable)),
        10 => g.opendrain().modify(|w| w.set_pin10(enable)),
        11 => g.opendrain().modify(|w| w.set_pin11(enable)),
        12 => g.opendrain().modify(|w| w.set_pin12(enable)),
        13 => g.opendrain().modify(|w| w.set_pin13(enable)),
        14 => g.opendrain().modify(|w| w.set_pin14(enable)),
        15 => g.opendrain().modify(|w| w.set_pin15(enable)),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}

/// 驱动能力位（GPIO_DRIVER.PIN）：`true` = 低驱动（RM 6.4.5）。
#[inline(always)]
pub(crate) fn set_dr(n: PinId, low_drive: bool) {
    let g = gpio(n.port);
    match n.pin {
        0 => g.driver().modify(|w| w.set_pin0(low_drive)),
        1 => g.driver().modify(|w| w.set_pin1(low_drive)),
        2 => g.driver().modify(|w| w.set_pin2(low_drive)),
        3 => g.driver().modify(|w| w.set_pin3(low_drive)),
        4 => g.driver().modify(|w| w.set_pin4(low_drive)),
        5 => g.driver().modify(|w| w.set_pin5(low_drive)),
        6 => g.driver().modify(|w| w.set_pin6(low_drive)),
        7 => g.driver().modify(|w| w.set_pin7(low_drive)),
        8 => g.driver().modify(|w| w.set_pin8(low_drive)),
        9 => g.driver().modify(|w| w.set_pin9(low_drive)),
        10 => g.driver().modify(|w| w.set_pin10(low_drive)),
        11 => g.driver().modify(|w| w.set_pin11(low_drive)),
        12 => g.driver().modify(|w| w.set_pin12(low_drive)),
        13 => g.driver().modify(|w| w.set_pin13(low_drive)),
        14 => g.driver().modify(|w| w.set_pin14(low_drive)),
        15 => g.driver().modify(|w| w.set_pin15(low_drive)),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}

/// 上/下拉配置（GPIO_PUPD：低半字 PU、高半字 PD，RM 6.4.6），单次
/// 读-改-写写入目标态。
///
/// `Pull::None` 两位均清零；`Pull::Up`/`Pull::Down` 只置对应位并清
/// 另一位（RM 注：PU 与 PD 同时为 1 时端口为上拉，本驱动按 Pull 枚举
/// 互斥配置；SDK `GPIO_Init` 同做法）。上拉典型 80 kΩ、下拉典型
/// 40 kΩ（DS Rev1.20 表 6-19）。
#[inline(always)]
pub(crate) fn set_pull(n: PinId, pull: Pull) {
    let up = matches!(pull, Pull::Up);
    let down = matches!(pull, Pull::Down);
    let g = gpio(n.port);
    match n.pin {
        0 => g.pupd().modify(|w| {
            w.set_pupin0(up);
            w.set_pdpin0(down);
        }),
        1 => g.pupd().modify(|w| {
            w.set_pupin1(up);
            w.set_pdpin1(down);
        }),
        2 => g.pupd().modify(|w| {
            w.set_pupin2(up);
            w.set_pdpin2(down);
        }),
        3 => g.pupd().modify(|w| {
            w.set_pupin3(up);
            w.set_pdpin3(down);
        }),
        4 => g.pupd().modify(|w| {
            w.set_pupin4(up);
            w.set_pdpin4(down);
        }),
        5 => g.pupd().modify(|w| {
            w.set_pupin5(up);
            w.set_pdpin5(down);
        }),
        6 => g.pupd().modify(|w| {
            w.set_pupin6(up);
            w.set_pdpin6(down);
        }),
        7 => g.pupd().modify(|w| {
            w.set_pupin7(up);
            w.set_pdpin7(down);
        }),
        8 => g.pupd().modify(|w| {
            w.set_pupin8(up);
            w.set_pdpin8(down);
        }),
        9 => g.pupd().modify(|w| {
            w.set_pupin9(up);
            w.set_pdpin9(down);
        }),
        10 => g.pupd().modify(|w| {
            w.set_pupin10(up);
            w.set_pdpin10(down);
        }),
        11 => g.pupd().modify(|w| {
            w.set_pupin11(up);
            w.set_pdpin11(down);
        }),
        12 => g.pupd().modify(|w| {
            w.set_pupin12(up);
            w.set_pdpin12(down);
        }),
        13 => g.pupd().modify(|w| {
            w.set_pupin13(up);
            w.set_pdpin13(down);
        }),
        14 => g.pupd().modify(|w| {
            w.set_pupin14(up);
            w.set_pdpin14(down);
        }),
        15 => g.pupd().modify(|w| {
            w.set_pupin15(up);
            w.set_pdpin15(down);
        }),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}

/// 复用功能选择（GPIO_AFRL/AFRH.AFSELy，每脚 4 位，RM 6.4.11/6.4.12）。
///
/// `func` 为复用编号：0 = GPIO 功能，1~5 = AF1~AF5（6~15 保留）。位号
/// 0~7 落 AFRL，8~15 落 AFRH；两寄存器复位值 0（GPIO 功能）。
#[inline(always)]
pub(crate) fn set_sel(n: PinId, func: u8) {
    debug_assert!(func <= 5, "F420 复用编号 AF1~AF5（RM 6.4.11）");
    let g = gpio(n.port);
    match n.pin {
        0 => g.afrl().modify(|w| w.set_afsel0(func)),
        1 => g.afrl().modify(|w| w.set_afsel1(func)),
        2 => g.afrl().modify(|w| w.set_afsel2(func)),
        3 => g.afrl().modify(|w| w.set_afsel3(func)),
        4 => g.afrl().modify(|w| w.set_afsel4(func)),
        5 => g.afrl().modify(|w| w.set_afsel5(func)),
        6 => g.afrl().modify(|w| w.set_afsel6(func)),
        7 => g.afrl().modify(|w| w.set_afsel7(func)),
        8 => g.afrh().modify(|w| w.set_afsel8(func)),
        9 => g.afrh().modify(|w| w.set_afsel9(func)),
        10 => g.afrh().modify(|w| w.set_afsel10(func)),
        11 => g.afrh().modify(|w| w.set_afsel11(func)),
        12 => g.afrh().modify(|w| w.set_afsel12(func)),
        13 => g.afrh().modify(|w| w.set_afsel13(func)),
        14 => g.afrh().modify(|w| w.set_afsel14(func)),
        15 => g.afrh().modify(|w| w.set_afsel15(func)),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}

/// 输出锁存置位（GPIO_BSRR.BS 目标位写 1、其余位写 0 保持，RM 6.4.10）。
///
/// `write` 从全 0 起写：整字只含目标引脚掩码，符合 W1 位语义，不触碰
/// 同端口其他引脚的锁存。
#[inline(always)]
pub(crate) fn set_out_high(n: PinId) {
    let g = gpio(n.port);
    match n.pin {
        0 => g.bsrr().write(|w| w.set_bspin0(true)),
        1 => g.bsrr().write(|w| w.set_bspin1(true)),
        2 => g.bsrr().write(|w| w.set_bspin2(true)),
        3 => g.bsrr().write(|w| w.set_bspin3(true)),
        4 => g.bsrr().write(|w| w.set_bspin4(true)),
        5 => g.bsrr().write(|w| w.set_bspin5(true)),
        6 => g.bsrr().write(|w| w.set_bspin6(true)),
        7 => g.bsrr().write(|w| w.set_bspin7(true)),
        8 => g.bsrr().write(|w| w.set_bspin8(true)),
        9 => g.bsrr().write(|w| w.set_bspin9(true)),
        10 => g.bsrr().write(|w| w.set_bspin10(true)),
        11 => g.bsrr().write(|w| w.set_bspin11(true)),
        12 => g.bsrr().write(|w| w.set_bspin12(true)),
        13 => g.bsrr().write(|w| w.set_bspin13(true)),
        14 => g.bsrr().write(|w| w.set_bspin14(true)),
        15 => g.bsrr().write(|w| w.set_bspin15(true)),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}

/// 输出锁存清零（GPIO_BRR.PIN 目标位写 1，RM 6.4.9）。
///
/// SDK `GPIO_PxNN_RESET` 走 BRR（而非 BSRR 的 BR 半字），本驱动同做法。
#[inline(always)]
pub(crate) fn set_out_low(n: PinId) {
    let g = gpio(n.port);
    match n.pin {
        0 => g.brr().write(|w| w.set_pin0(true)),
        1 => g.brr().write(|w| w.set_pin1(true)),
        2 => g.brr().write(|w| w.set_pin2(true)),
        3 => g.brr().write(|w| w.set_pin3(true)),
        4 => g.brr().write(|w| w.set_pin4(true)),
        5 => g.brr().write(|w| w.set_pin5(true)),
        6 => g.brr().write(|w| w.set_pin6(true)),
        7 => g.brr().write(|w| w.set_pin7(true)),
        8 => g.brr().write(|w| w.set_pin8(true)),
        9 => g.brr().write(|w| w.set_pin9(true)),
        10 => g.brr().write(|w| w.set_pin10(true)),
        11 => g.brr().write(|w| w.set_pin11(true)),
        12 => g.brr().write(|w| w.set_pin12(true)),
        13 => g.brr().write(|w| w.set_pin13(true)),
        14 => g.brr().write(|w| w.set_pin14(true)),
        15 => g.brr().write(|w| w.set_pin15(true)),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}

/// 输出锁存位写入：`true` = 高。
#[inline(always)]
pub(crate) fn set_out(n: PinId, high: bool) {
    if high {
        set_out_high(n);
    } else {
        set_out_low(n);
    }
}

/// 输出锁存位（GPIO_OUT.PIN）读取：`true` = 高（RM 6.4.8）。
///
/// 单引脚置位/清零走 BSRR/BRR（见 [`set_out_high`]/[`set_out_low`]），
/// 电平状态仍从 GPIO_OUT 锁存读取。
#[inline(always)]
pub(crate) fn read_out(n: PinId) -> bool {
    let g = gpio(n.port);
    match n.pin {
        0 => g.out().read().pin0(),
        1 => g.out().read().pin1(),
        2 => g.out().read().pin2(),
        3 => g.out().read().pin3(),
        4 => g.out().read().pin4(),
        5 => g.out().read().pin5(),
        6 => g.out().read().pin6(),
        7 => g.out().read().pin7(),
        8 => g.out().read().pin8(),
        9 => g.out().read().pin9(),
        10 => g.out().read().pin10(),
        11 => g.out().read().pin11(),
        12 => g.out().read().pin12(),
        13 => g.out().read().pin13(),
        14 => g.out().read().pin14(),
        15 => g.out().read().pin15(),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}

/// 输入电平位（GPIO_IN.PIN）读取：`true` = 高（RM 6.4.7）。
#[inline(always)]
pub(crate) fn read_in(n: PinId) -> bool {
    let g = gpio(n.port);
    match n.pin {
        0 => g.in_().read().pin0(),
        1 => g.in_().read().pin1(),
        2 => g.in_().read().pin2(),
        3 => g.in_().read().pin3(),
        4 => g.in_().read().pin4(),
        5 => g.in_().read().pin5(),
        6 => g.in_().read().pin6(),
        7 => g.in_().read().pin7(),
        8 => g.in_().read().pin8(),
        9 => g.in_().read().pin9(),
        10 => g.in_().read().pin10(),
        11 => g.in_().read().pin11(),
        12 => g.in_().read().pin12(),
        13 => g.in_().read().pin13(),
        14 => g.in_().read().pin14(),
        15 => g.in_().read().pin15(),
        _ => unreachable!("GPIO v3_1 后端无此位号"),
    }
}
