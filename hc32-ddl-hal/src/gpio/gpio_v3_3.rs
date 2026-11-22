//! GPIO 寄存器后端：DDL v3_3 端口阵列布局（HC32L13x）。
//!
//! 单外设多端口：PA–PD 的控制寄存器集中在一个寄存器块内（PADIR/PAIN/
//! PAOUT/PAADS/… 与 PB/PC/PD 同构，RM 6.5.1~6.5.4），复用选择为逐引脚
//! 的 `Pxy_SEL` 寄存器。方向位 PxDIR 1 = 输入、0 = 输出（RM 6.5.1.17），
//! 模拟位 PxADS 1 = 模拟（RM 6.5.1.20），与 L110 的 v2 布局语义一致，
//! 仅寄存器名按端口命名。
//!
//! 输出置位/清零走本 IP 特有的位置位/位清零寄存器（PxBSET/PxBCLR，
//! RM 6.5.1.21/22，W1 位：1 = 置位/清零，0 = 保持）：整字只写目标引脚
//! 的掩码，不动其他引脚的锁存，也不需要读-改-写（SDK `Gpio_SetIO`/
//! `Gpio_ClrIO` 同做法）。v2 布局没有这对寄存器，这是两个后端的主要
//! 写路径差异。
//!
//! 门控：GPIO 寄存器访问前须打开 PERI_CLKEN 位 28（复位为 0，RM 3.5.9；
//! 复位值 0x8080_0000），各入口幂等地先打开本门控。

use super::PinId;
use crate::pac;

/// 打开 GPIO 外设时钟门控，幂等。
///
/// PERI_CLKEN 位 28 复位为 0（RM 3.5.9）；门控关闭时对 GPIO 寄存器的
/// 写入不生效。公共 GPIO 初始化经由当前型号的系统控制器（clock 模块的
/// [`crate::clock::enable_gpio`]）打开本门控。
#[allow(dead_code)] // L13x 侧由后续接入的通信驱动（UART 等）调用
pub(crate) fn enable() {
    crate::clock::enable_gpio();
}

/// 方向位（PxDIR）：`true` = 输入，`false` = 输出（RM 6.5.1.17）。
///
/// 复位值 0xFFFF：全部输入（与 RM 6.3"芯片复位后端口为高阻输入"一致）。
#[inline(always)]
pub(crate) fn set_dir(n: PinId, input: bool) {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.padir().modify(|w| w.set_pa00(input)),
        (0, 1) => pac::GPIO.padir().modify(|w| w.set_pa01(input)),
        (0, 2) => pac::GPIO.padir().modify(|w| w.set_pa02(input)),
        (0, 3) => pac::GPIO.padir().modify(|w| w.set_pa03(input)),
        (0, 4) => pac::GPIO.padir().modify(|w| w.set_pa04(input)),
        (0, 5) => pac::GPIO.padir().modify(|w| w.set_pa05(input)),
        (0, 6) => pac::GPIO.padir().modify(|w| w.set_pa06(input)),
        (0, 7) => pac::GPIO.padir().modify(|w| w.set_pa07(input)),
        (0, 8) => pac::GPIO.padir().modify(|w| w.set_pa08(input)),
        (0, 9) => pac::GPIO.padir().modify(|w| w.set_pa09(input)),
        (0, 10) => pac::GPIO.padir().modify(|w| w.set_pa10(input)),
        (0, 11) => pac::GPIO.padir().modify(|w| w.set_pa11(input)),
        (0, 12) => pac::GPIO.padir().modify(|w| w.set_pa12(input)),
        (0, 13) => pac::GPIO.padir().modify(|w| w.set_pa13(input)),
        (0, 14) => pac::GPIO.padir().modify(|w| w.set_pa14(input)),
        (0, 15) => pac::GPIO.padir().modify(|w| w.set_pa15(input)),
        (1, 0) => pac::GPIO.pbdir().modify(|w| w.set_pb00(input)),
        (1, 1) => pac::GPIO.pbdir().modify(|w| w.set_pb01(input)),
        (1, 2) => pac::GPIO.pbdir().modify(|w| w.set_pb02(input)),
        (1, 3) => pac::GPIO.pbdir().modify(|w| w.set_pb03(input)),
        (1, 4) => pac::GPIO.pbdir().modify(|w| w.set_pb04(input)),
        (1, 5) => pac::GPIO.pbdir().modify(|w| w.set_pb05(input)),
        (1, 6) => pac::GPIO.pbdir().modify(|w| w.set_pb06(input)),
        (1, 7) => pac::GPIO.pbdir().modify(|w| w.set_pb07(input)),
        (1, 8) => pac::GPIO.pbdir().modify(|w| w.set_pb08(input)),
        (1, 9) => pac::GPIO.pbdir().modify(|w| w.set_pb09(input)),
        (1, 10) => pac::GPIO.pbdir().modify(|w| w.set_pb10(input)),
        (1, 11) => pac::GPIO.pbdir().modify(|w| w.set_pb11(input)),
        (1, 12) => pac::GPIO.pbdir().modify(|w| w.set_pb12(input)),
        (1, 13) => pac::GPIO.pbdir().modify(|w| w.set_pb13(input)),
        (1, 14) => pac::GPIO.pbdir().modify(|w| w.set_pb14(input)),
        (1, 15) => pac::GPIO.pbdir().modify(|w| w.set_pb15(input)),
        (2, 0) => pac::GPIO.pcdir().modify(|w| w.set_pc00(input)),
        (2, 1) => pac::GPIO.pcdir().modify(|w| w.set_pc01(input)),
        (2, 2) => pac::GPIO.pcdir().modify(|w| w.set_pc02(input)),
        (2, 3) => pac::GPIO.pcdir().modify(|w| w.set_pc03(input)),
        (2, 4) => pac::GPIO.pcdir().modify(|w| w.set_pc04(input)),
        (2, 5) => pac::GPIO.pcdir().modify(|w| w.set_pc05(input)),
        (2, 6) => pac::GPIO.pcdir().modify(|w| w.set_pc06(input)),
        (2, 7) => pac::GPIO.pcdir().modify(|w| w.set_pc07(input)),
        (2, 8) => pac::GPIO.pcdir().modify(|w| w.set_pc08(input)),
        (2, 9) => pac::GPIO.pcdir().modify(|w| w.set_pc09(input)),
        (2, 10) => pac::GPIO.pcdir().modify(|w| w.set_pc10(input)),
        (2, 11) => pac::GPIO.pcdir().modify(|w| w.set_pc11(input)),
        (2, 12) => pac::GPIO.pcdir().modify(|w| w.set_pc12(input)),
        (2, 13) => pac::GPIO.pcdir().modify(|w| w.set_pc13(input)),
        (2, 14) => pac::GPIO.pcdir().modify(|w| w.set_pc14(input)),
        (2, 15) => pac::GPIO.pcdir().modify(|w| w.set_pc15(input)),
        (3, 0) => pac::GPIO.pddir().modify(|w| w.set_pd00(input)),
        (3, 1) => pac::GPIO.pddir().modify(|w| w.set_pd01(input)),
        (3, 2) => pac::GPIO.pddir().modify(|w| w.set_pd02(input)),
        (3, 3) => pac::GPIO.pddir().modify(|w| w.set_pd03(input)),
        (3, 4) => pac::GPIO.pddir().modify(|w| w.set_pd04(input)),
        (3, 5) => pac::GPIO.pddir().modify(|w| w.set_pd05(input)),
        (3, 6) => pac::GPIO.pddir().modify(|w| w.set_pd06(input)),
        (3, 7) => pac::GPIO.pddir().modify(|w| w.set_pd07(input)),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 输出锁存置位（PxBSET 目标位写 1，其余位写 0 保持，RM 6.5.1.21）。
///
/// `write` 从全 0 起写：整字只含目标引脚的掩码，符合 W1 位语义
/// （1 = 置位，0 = 保持），不触碰同端口其他引脚的锁存。
#[inline(always)]
pub(crate) fn set_out_high(n: PinId) {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.pabset().write(|w| w.set_pa00(true)),
        (0, 1) => pac::GPIO.pabset().write(|w| w.set_pa01(true)),
        (0, 2) => pac::GPIO.pabset().write(|w| w.set_pa02(true)),
        (0, 3) => pac::GPIO.pabset().write(|w| w.set_pa03(true)),
        (0, 4) => pac::GPIO.pabset().write(|w| w.set_pa04(true)),
        (0, 5) => pac::GPIO.pabset().write(|w| w.set_pa05(true)),
        (0, 6) => pac::GPIO.pabset().write(|w| w.set_pa06(true)),
        (0, 7) => pac::GPIO.pabset().write(|w| w.set_pa07(true)),
        (0, 8) => pac::GPIO.pabset().write(|w| w.set_pa08(true)),
        (0, 9) => pac::GPIO.pabset().write(|w| w.set_pa09(true)),
        (0, 10) => pac::GPIO.pabset().write(|w| w.set_pa10(true)),
        (0, 11) => pac::GPIO.pabset().write(|w| w.set_pa11(true)),
        (0, 12) => pac::GPIO.pabset().write(|w| w.set_pa12(true)),
        (0, 13) => pac::GPIO.pabset().write(|w| w.set_pa13(true)),
        (0, 14) => pac::GPIO.pabset().write(|w| w.set_pa14(true)),
        (0, 15) => pac::GPIO.pabset().write(|w| w.set_pa15(true)),
        (1, 0) => pac::GPIO.pbbset().write(|w| w.set_pb00(true)),
        (1, 1) => pac::GPIO.pbbset().write(|w| w.set_pb01(true)),
        (1, 2) => pac::GPIO.pbbset().write(|w| w.set_pb02(true)),
        (1, 3) => pac::GPIO.pbbset().write(|w| w.set_pb03(true)),
        (1, 4) => pac::GPIO.pbbset().write(|w| w.set_pb04(true)),
        (1, 5) => pac::GPIO.pbbset().write(|w| w.set_pb05(true)),
        (1, 6) => pac::GPIO.pbbset().write(|w| w.set_pb06(true)),
        (1, 7) => pac::GPIO.pbbset().write(|w| w.set_pb07(true)),
        (1, 8) => pac::GPIO.pbbset().write(|w| w.set_pb08(true)),
        (1, 9) => pac::GPIO.pbbset().write(|w| w.set_pb09(true)),
        (1, 10) => pac::GPIO.pbbset().write(|w| w.set_pb10(true)),
        (1, 11) => pac::GPIO.pbbset().write(|w| w.set_pb11(true)),
        (1, 12) => pac::GPIO.pbbset().write(|w| w.set_pb12(true)),
        (1, 13) => pac::GPIO.pbbset().write(|w| w.set_pb13(true)),
        (1, 14) => pac::GPIO.pbbset().write(|w| w.set_pb14(true)),
        (1, 15) => pac::GPIO.pbbset().write(|w| w.set_pb15(true)),
        (2, 0) => pac::GPIO.pcbset().write(|w| w.set_pc00(true)),
        (2, 1) => pac::GPIO.pcbset().write(|w| w.set_pc01(true)),
        (2, 2) => pac::GPIO.pcbset().write(|w| w.set_pc02(true)),
        (2, 3) => pac::GPIO.pcbset().write(|w| w.set_pc03(true)),
        (2, 4) => pac::GPIO.pcbset().write(|w| w.set_pc04(true)),
        (2, 5) => pac::GPIO.pcbset().write(|w| w.set_pc05(true)),
        (2, 6) => pac::GPIO.pcbset().write(|w| w.set_pc06(true)),
        (2, 7) => pac::GPIO.pcbset().write(|w| w.set_pc07(true)),
        (2, 8) => pac::GPIO.pcbset().write(|w| w.set_pc08(true)),
        (2, 9) => pac::GPIO.pcbset().write(|w| w.set_pc09(true)),
        (2, 10) => pac::GPIO.pcbset().write(|w| w.set_pc10(true)),
        (2, 11) => pac::GPIO.pcbset().write(|w| w.set_pc11(true)),
        (2, 12) => pac::GPIO.pcbset().write(|w| w.set_pc12(true)),
        (2, 13) => pac::GPIO.pcbset().write(|w| w.set_pc13(true)),
        (2, 14) => pac::GPIO.pcbset().write(|w| w.set_pc14(true)),
        (2, 15) => pac::GPIO.pcbset().write(|w| w.set_pc15(true)),
        (3, 0) => pac::GPIO.pdbset().write(|w| w.set_pd00(true)),
        (3, 1) => pac::GPIO.pdbset().write(|w| w.set_pd01(true)),
        (3, 2) => pac::GPIO.pdbset().write(|w| w.set_pd02(true)),
        (3, 3) => pac::GPIO.pdbset().write(|w| w.set_pd03(true)),
        (3, 4) => pac::GPIO.pdbset().write(|w| w.set_pd04(true)),
        (3, 5) => pac::GPIO.pdbset().write(|w| w.set_pd05(true)),
        (3, 6) => pac::GPIO.pdbset().write(|w| w.set_pd06(true)),
        (3, 7) => pac::GPIO.pdbset().write(|w| w.set_pd07(true)),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 输出锁存清零（PxBCLR 目标位写 1，其余位写 0 保持，RM 6.5.1.22）。
#[inline(always)]
pub(crate) fn set_out_low(n: PinId) {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.pabclr().write(|w| w.set_pa00(true)),
        (0, 1) => pac::GPIO.pabclr().write(|w| w.set_pa01(true)),
        (0, 2) => pac::GPIO.pabclr().write(|w| w.set_pa02(true)),
        (0, 3) => pac::GPIO.pabclr().write(|w| w.set_pa03(true)),
        (0, 4) => pac::GPIO.pabclr().write(|w| w.set_pa04(true)),
        (0, 5) => pac::GPIO.pabclr().write(|w| w.set_pa05(true)),
        (0, 6) => pac::GPIO.pabclr().write(|w| w.set_pa06(true)),
        (0, 7) => pac::GPIO.pabclr().write(|w| w.set_pa07(true)),
        (0, 8) => pac::GPIO.pabclr().write(|w| w.set_pa08(true)),
        (0, 9) => pac::GPIO.pabclr().write(|w| w.set_pa09(true)),
        (0, 10) => pac::GPIO.pabclr().write(|w| w.set_pa10(true)),
        (0, 11) => pac::GPIO.pabclr().write(|w| w.set_pa11(true)),
        (0, 12) => pac::GPIO.pabclr().write(|w| w.set_pa12(true)),
        (0, 13) => pac::GPIO.pabclr().write(|w| w.set_pa13(true)),
        (0, 14) => pac::GPIO.pabclr().write(|w| w.set_pa14(true)),
        (0, 15) => pac::GPIO.pabclr().write(|w| w.set_pa15(true)),
        (1, 0) => pac::GPIO.pbbclr().write(|w| w.set_pb00(true)),
        (1, 1) => pac::GPIO.pbbclr().write(|w| w.set_pb01(true)),
        (1, 2) => pac::GPIO.pbbclr().write(|w| w.set_pb02(true)),
        (1, 3) => pac::GPIO.pbbclr().write(|w| w.set_pb03(true)),
        (1, 4) => pac::GPIO.pbbclr().write(|w| w.set_pb04(true)),
        (1, 5) => pac::GPIO.pbbclr().write(|w| w.set_pb05(true)),
        (1, 6) => pac::GPIO.pbbclr().write(|w| w.set_pb06(true)),
        (1, 7) => pac::GPIO.pbbclr().write(|w| w.set_pb07(true)),
        (1, 8) => pac::GPIO.pbbclr().write(|w| w.set_pb08(true)),
        (1, 9) => pac::GPIO.pbbclr().write(|w| w.set_pb09(true)),
        (1, 10) => pac::GPIO.pbbclr().write(|w| w.set_pb10(true)),
        (1, 11) => pac::GPIO.pbbclr().write(|w| w.set_pb11(true)),
        (1, 12) => pac::GPIO.pbbclr().write(|w| w.set_pb12(true)),
        (1, 13) => pac::GPIO.pbbclr().write(|w| w.set_pb13(true)),
        (1, 14) => pac::GPIO.pbbclr().write(|w| w.set_pb14(true)),
        (1, 15) => pac::GPIO.pbbclr().write(|w| w.set_pb15(true)),
        (2, 0) => pac::GPIO.pcbclr().write(|w| w.set_pc00(true)),
        (2, 1) => pac::GPIO.pcbclr().write(|w| w.set_pc01(true)),
        (2, 2) => pac::GPIO.pcbclr().write(|w| w.set_pc02(true)),
        (2, 3) => pac::GPIO.pcbclr().write(|w| w.set_pc03(true)),
        (2, 4) => pac::GPIO.pcbclr().write(|w| w.set_pc04(true)),
        (2, 5) => pac::GPIO.pcbclr().write(|w| w.set_pc05(true)),
        (2, 6) => pac::GPIO.pcbclr().write(|w| w.set_pc06(true)),
        (2, 7) => pac::GPIO.pcbclr().write(|w| w.set_pc07(true)),
        (2, 8) => pac::GPIO.pcbclr().write(|w| w.set_pc08(true)),
        (2, 9) => pac::GPIO.pcbclr().write(|w| w.set_pc09(true)),
        (2, 10) => pac::GPIO.pcbclr().write(|w| w.set_pc10(true)),
        (2, 11) => pac::GPIO.pcbclr().write(|w| w.set_pc11(true)),
        (2, 12) => pac::GPIO.pcbclr().write(|w| w.set_pc12(true)),
        (2, 13) => pac::GPIO.pcbclr().write(|w| w.set_pc13(true)),
        (2, 14) => pac::GPIO.pcbclr().write(|w| w.set_pc14(true)),
        (2, 15) => pac::GPIO.pcbclr().write(|w| w.set_pc15(true)),
        (3, 0) => pac::GPIO.pdbclr().write(|w| w.set_pd00(true)),
        (3, 1) => pac::GPIO.pdbclr().write(|w| w.set_pd01(true)),
        (3, 2) => pac::GPIO.pdbclr().write(|w| w.set_pd02(true)),
        (3, 3) => pac::GPIO.pdbclr().write(|w| w.set_pd03(true)),
        (3, 4) => pac::GPIO.pdbclr().write(|w| w.set_pd04(true)),
        (3, 5) => pac::GPIO.pdbclr().write(|w| w.set_pd05(true)),
        (3, 6) => pac::GPIO.pdbclr().write(|w| w.set_pd06(true)),
        (3, 7) => pac::GPIO.pdbclr().write(|w| w.set_pd07(true)),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 输出锁存位（PxOUT）：`true` = 高。
///
/// 单引脚置位/清零走 PxBSET/PxBCLR（见 [`set_out_high`]/[`set_out_low`]）。
/// BSET/BCLR 是 W1 触发映像，电平状态仍从 PxOUT 锁存读取（`read_out`）。
#[inline(always)]
pub(crate) fn set_out(n: PinId, high: bool) {
    if high {
        set_out_high(n);
    } else {
        set_out_low(n);
    }
}

/// 输出锁存位（PxOUT）读取：`true` = 高。
#[inline(always)]
pub(crate) fn read_out(n: PinId) -> bool {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.paout().read().pa00(),
        (0, 1) => pac::GPIO.paout().read().pa01(),
        (0, 2) => pac::GPIO.paout().read().pa02(),
        (0, 3) => pac::GPIO.paout().read().pa03(),
        (0, 4) => pac::GPIO.paout().read().pa04(),
        (0, 5) => pac::GPIO.paout().read().pa05(),
        (0, 6) => pac::GPIO.paout().read().pa06(),
        (0, 7) => pac::GPIO.paout().read().pa07(),
        (0, 8) => pac::GPIO.paout().read().pa08(),
        (0, 9) => pac::GPIO.paout().read().pa09(),
        (0, 10) => pac::GPIO.paout().read().pa10(),
        (0, 11) => pac::GPIO.paout().read().pa11(),
        (0, 12) => pac::GPIO.paout().read().pa12(),
        (0, 13) => pac::GPIO.paout().read().pa13(),
        (0, 14) => pac::GPIO.paout().read().pa14(),
        (0, 15) => pac::GPIO.paout().read().pa15(),
        (1, 0) => pac::GPIO.pbout().read().pb00(),
        (1, 1) => pac::GPIO.pbout().read().pb01(),
        (1, 2) => pac::GPIO.pbout().read().pb02(),
        (1, 3) => pac::GPIO.pbout().read().pb03(),
        (1, 4) => pac::GPIO.pbout().read().pb04(),
        (1, 5) => pac::GPIO.pbout().read().pb05(),
        (1, 6) => pac::GPIO.pbout().read().pb06(),
        (1, 7) => pac::GPIO.pbout().read().pb07(),
        (1, 8) => pac::GPIO.pbout().read().pb08(),
        (1, 9) => pac::GPIO.pbout().read().pb09(),
        (1, 10) => pac::GPIO.pbout().read().pb10(),
        (1, 11) => pac::GPIO.pbout().read().pb11(),
        (1, 12) => pac::GPIO.pbout().read().pb12(),
        (1, 13) => pac::GPIO.pbout().read().pb13(),
        (1, 14) => pac::GPIO.pbout().read().pb14(),
        (1, 15) => pac::GPIO.pbout().read().pb15(),
        (2, 0) => pac::GPIO.pcout().read().pc00(),
        (2, 1) => pac::GPIO.pcout().read().pc01(),
        (2, 2) => pac::GPIO.pcout().read().pc02(),
        (2, 3) => pac::GPIO.pcout().read().pc03(),
        (2, 4) => pac::GPIO.pcout().read().pc04(),
        (2, 5) => pac::GPIO.pcout().read().pc05(),
        (2, 6) => pac::GPIO.pcout().read().pc06(),
        (2, 7) => pac::GPIO.pcout().read().pc07(),
        (2, 8) => pac::GPIO.pcout().read().pc08(),
        (2, 9) => pac::GPIO.pcout().read().pc09(),
        (2, 10) => pac::GPIO.pcout().read().pc10(),
        (2, 11) => pac::GPIO.pcout().read().pc11(),
        (2, 12) => pac::GPIO.pcout().read().pc12(),
        (2, 13) => pac::GPIO.pcout().read().pc13(),
        (2, 14) => pac::GPIO.pcout().read().pc14(),
        (2, 15) => pac::GPIO.pcout().read().pc15(),
        (3, 0) => pac::GPIO.pdout().read().pd00(),
        (3, 1) => pac::GPIO.pdout().read().pd01(),
        (3, 2) => pac::GPIO.pdout().read().pd02(),
        (3, 3) => pac::GPIO.pdout().read().pd03(),
        (3, 4) => pac::GPIO.pdout().read().pd04(),
        (3, 5) => pac::GPIO.pdout().read().pd05(),
        (3, 6) => pac::GPIO.pdout().read().pd06(),
        (3, 7) => pac::GPIO.pdout().read().pd07(),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 输入电平位（PxIN）读取：`true` = 高。
#[inline(always)]
pub(crate) fn read_in(n: PinId) -> bool {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.pain().read().pa00(),
        (0, 1) => pac::GPIO.pain().read().pa01(),
        (0, 2) => pac::GPIO.pain().read().pa02(),
        (0, 3) => pac::GPIO.pain().read().pa03(),
        (0, 4) => pac::GPIO.pain().read().pa04(),
        (0, 5) => pac::GPIO.pain().read().pa05(),
        (0, 6) => pac::GPIO.pain().read().pa06(),
        (0, 7) => pac::GPIO.pain().read().pa07(),
        (0, 8) => pac::GPIO.pain().read().pa08(),
        (0, 9) => pac::GPIO.pain().read().pa09(),
        (0, 10) => pac::GPIO.pain().read().pa10(),
        (0, 11) => pac::GPIO.pain().read().pa11(),
        (0, 12) => pac::GPIO.pain().read().pa12(),
        (0, 13) => pac::GPIO.pain().read().pa13(),
        (0, 14) => pac::GPIO.pain().read().pa14(),
        (0, 15) => pac::GPIO.pain().read().pa15(),
        (1, 0) => pac::GPIO.pbin().read().pb00(),
        (1, 1) => pac::GPIO.pbin().read().pb01(),
        (1, 2) => pac::GPIO.pbin().read().pb02(),
        (1, 3) => pac::GPIO.pbin().read().pb03(),
        (1, 4) => pac::GPIO.pbin().read().pb04(),
        (1, 5) => pac::GPIO.pbin().read().pb05(),
        (1, 6) => pac::GPIO.pbin().read().pb06(),
        (1, 7) => pac::GPIO.pbin().read().pb07(),
        (1, 8) => pac::GPIO.pbin().read().pb08(),
        (1, 9) => pac::GPIO.pbin().read().pb09(),
        (1, 10) => pac::GPIO.pbin().read().pb10(),
        (1, 11) => pac::GPIO.pbin().read().pb11(),
        (1, 12) => pac::GPIO.pbin().read().pb12(),
        (1, 13) => pac::GPIO.pbin().read().pb13(),
        (1, 14) => pac::GPIO.pbin().read().pb14(),
        (1, 15) => pac::GPIO.pbin().read().pb15(),
        (2, 0) => pac::GPIO.pcin().read().pc00(),
        (2, 1) => pac::GPIO.pcin().read().pc01(),
        (2, 2) => pac::GPIO.pcin().read().pc02(),
        (2, 3) => pac::GPIO.pcin().read().pc03(),
        (2, 4) => pac::GPIO.pcin().read().pc04(),
        (2, 5) => pac::GPIO.pcin().read().pc05(),
        (2, 6) => pac::GPIO.pcin().read().pc06(),
        (2, 7) => pac::GPIO.pcin().read().pc07(),
        (2, 8) => pac::GPIO.pcin().read().pc08(),
        (2, 9) => pac::GPIO.pcin().read().pc09(),
        (2, 10) => pac::GPIO.pcin().read().pc10(),
        (2, 11) => pac::GPIO.pcin().read().pc11(),
        (2, 12) => pac::GPIO.pcin().read().pc12(),
        (2, 13) => pac::GPIO.pcin().read().pc13(),
        (2, 14) => pac::GPIO.pcin().read().pc14(),
        (2, 15) => pac::GPIO.pcin().read().pc15(),
        (3, 0) => pac::GPIO.pdin().read().pd00(),
        (3, 1) => pac::GPIO.pdin().read().pd01(),
        (3, 2) => pac::GPIO.pdin().read().pd02(),
        (3, 3) => pac::GPIO.pdin().read().pd03(),
        (3, 4) => pac::GPIO.pdin().read().pd04(),
        (3, 5) => pac::GPIO.pdin().read().pd05(),
        (3, 6) => pac::GPIO.pdin().read().pd06(),
        (3, 7) => pac::GPIO.pdin().read().pd07(),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 上拉使能位（PxPU）。
#[inline(always)]
pub(crate) fn set_pu(n: PinId, enable: bool) {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.papu().modify(|w| w.set_pa00(enable)),
        (0, 1) => pac::GPIO.papu().modify(|w| w.set_pa01(enable)),
        (0, 2) => pac::GPIO.papu().modify(|w| w.set_pa02(enable)),
        (0, 3) => pac::GPIO.papu().modify(|w| w.set_pa03(enable)),
        (0, 4) => pac::GPIO.papu().modify(|w| w.set_pa04(enable)),
        (0, 5) => pac::GPIO.papu().modify(|w| w.set_pa05(enable)),
        (0, 6) => pac::GPIO.papu().modify(|w| w.set_pa06(enable)),
        (0, 7) => pac::GPIO.papu().modify(|w| w.set_pa07(enable)),
        (0, 8) => pac::GPIO.papu().modify(|w| w.set_pa08(enable)),
        (0, 9) => pac::GPIO.papu().modify(|w| w.set_pa09(enable)),
        (0, 10) => pac::GPIO.papu().modify(|w| w.set_pa10(enable)),
        (0, 11) => pac::GPIO.papu().modify(|w| w.set_pa11(enable)),
        (0, 12) => pac::GPIO.papu().modify(|w| w.set_pa12(enable)),
        (0, 13) => pac::GPIO.papu().modify(|w| w.set_pa13(enable)),
        (0, 14) => pac::GPIO.papu().modify(|w| w.set_pa14(enable)),
        (0, 15) => pac::GPIO.papu().modify(|w| w.set_pa15(enable)),
        (1, 0) => pac::GPIO.pbpu().modify(|w| w.set_pb00(enable)),
        (1, 1) => pac::GPIO.pbpu().modify(|w| w.set_pb01(enable)),
        (1, 2) => pac::GPIO.pbpu().modify(|w| w.set_pb02(enable)),
        (1, 3) => pac::GPIO.pbpu().modify(|w| w.set_pb03(enable)),
        (1, 4) => pac::GPIO.pbpu().modify(|w| w.set_pb04(enable)),
        (1, 5) => pac::GPIO.pbpu().modify(|w| w.set_pb05(enable)),
        (1, 6) => pac::GPIO.pbpu().modify(|w| w.set_pb06(enable)),
        (1, 7) => pac::GPIO.pbpu().modify(|w| w.set_pb07(enable)),
        (1, 8) => pac::GPIO.pbpu().modify(|w| w.set_pb08(enable)),
        (1, 9) => pac::GPIO.pbpu().modify(|w| w.set_pb09(enable)),
        (1, 10) => pac::GPIO.pbpu().modify(|w| w.set_pb10(enable)),
        (1, 11) => pac::GPIO.pbpu().modify(|w| w.set_pb11(enable)),
        (1, 12) => pac::GPIO.pbpu().modify(|w| w.set_pb12(enable)),
        (1, 13) => pac::GPIO.pbpu().modify(|w| w.set_pb13(enable)),
        (1, 14) => pac::GPIO.pbpu().modify(|w| w.set_pb14(enable)),
        (1, 15) => pac::GPIO.pbpu().modify(|w| w.set_pb15(enable)),
        (2, 0) => pac::GPIO.pcpu().modify(|w| w.set_pc00(enable)),
        (2, 1) => pac::GPIO.pcpu().modify(|w| w.set_pc01(enable)),
        (2, 2) => pac::GPIO.pcpu().modify(|w| w.set_pc02(enable)),
        (2, 3) => pac::GPIO.pcpu().modify(|w| w.set_pc03(enable)),
        (2, 4) => pac::GPIO.pcpu().modify(|w| w.set_pc04(enable)),
        (2, 5) => pac::GPIO.pcpu().modify(|w| w.set_pc05(enable)),
        (2, 6) => pac::GPIO.pcpu().modify(|w| w.set_pc06(enable)),
        (2, 7) => pac::GPIO.pcpu().modify(|w| w.set_pc07(enable)),
        (2, 8) => pac::GPIO.pcpu().modify(|w| w.set_pc08(enable)),
        (2, 9) => pac::GPIO.pcpu().modify(|w| w.set_pc09(enable)),
        (2, 10) => pac::GPIO.pcpu().modify(|w| w.set_pc10(enable)),
        (2, 11) => pac::GPIO.pcpu().modify(|w| w.set_pc11(enable)),
        (2, 12) => pac::GPIO.pcpu().modify(|w| w.set_pc12(enable)),
        (2, 13) => pac::GPIO.pcpu().modify(|w| w.set_pc13(enable)),
        (2, 14) => pac::GPIO.pcpu().modify(|w| w.set_pc14(enable)),
        (2, 15) => pac::GPIO.pcpu().modify(|w| w.set_pc15(enable)),
        (3, 0) => pac::GPIO.pdpu().modify(|w| w.set_pd00(enable)),
        (3, 1) => pac::GPIO.pdpu().modify(|w| w.set_pd01(enable)),
        (3, 2) => pac::GPIO.pdpu().modify(|w| w.set_pd02(enable)),
        (3, 3) => pac::GPIO.pdpu().modify(|w| w.set_pd03(enable)),
        (3, 4) => pac::GPIO.pdpu().modify(|w| w.set_pd04(enable)),
        (3, 5) => pac::GPIO.pdpu().modify(|w| w.set_pd05(enable)),
        (3, 6) => pac::GPIO.pdpu().modify(|w| w.set_pd06(enable)),
        (3, 7) => pac::GPIO.pdpu().modify(|w| w.set_pd07(enable)),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 下拉使能位（PxPD）。
#[inline(always)]
pub(crate) fn set_pd(n: PinId, enable: bool) {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.papd().modify(|w| w.set_pa00(enable)),
        (0, 1) => pac::GPIO.papd().modify(|w| w.set_pa01(enable)),
        (0, 2) => pac::GPIO.papd().modify(|w| w.set_pa02(enable)),
        (0, 3) => pac::GPIO.papd().modify(|w| w.set_pa03(enable)),
        (0, 4) => pac::GPIO.papd().modify(|w| w.set_pa04(enable)),
        (0, 5) => pac::GPIO.papd().modify(|w| w.set_pa05(enable)),
        (0, 6) => pac::GPIO.papd().modify(|w| w.set_pa06(enable)),
        (0, 7) => pac::GPIO.papd().modify(|w| w.set_pa07(enable)),
        (0, 8) => pac::GPIO.papd().modify(|w| w.set_pa08(enable)),
        (0, 9) => pac::GPIO.papd().modify(|w| w.set_pa09(enable)),
        (0, 10) => pac::GPIO.papd().modify(|w| w.set_pa10(enable)),
        (0, 11) => pac::GPIO.papd().modify(|w| w.set_pa11(enable)),
        (0, 12) => pac::GPIO.papd().modify(|w| w.set_pa12(enable)),
        (0, 13) => pac::GPIO.papd().modify(|w| w.set_pa13(enable)),
        (0, 14) => pac::GPIO.papd().modify(|w| w.set_pa14(enable)),
        (0, 15) => pac::GPIO.papd().modify(|w| w.set_pa15(enable)),
        (1, 0) => pac::GPIO.pbpd().modify(|w| w.set_pb00(enable)),
        (1, 1) => pac::GPIO.pbpd().modify(|w| w.set_pb01(enable)),
        (1, 2) => pac::GPIO.pbpd().modify(|w| w.set_pb02(enable)),
        (1, 3) => pac::GPIO.pbpd().modify(|w| w.set_pb03(enable)),
        (1, 4) => pac::GPIO.pbpd().modify(|w| w.set_pb04(enable)),
        (1, 5) => pac::GPIO.pbpd().modify(|w| w.set_pb05(enable)),
        (1, 6) => pac::GPIO.pbpd().modify(|w| w.set_pb06(enable)),
        (1, 7) => pac::GPIO.pbpd().modify(|w| w.set_pb07(enable)),
        (1, 8) => pac::GPIO.pbpd().modify(|w| w.set_pb08(enable)),
        (1, 9) => pac::GPIO.pbpd().modify(|w| w.set_pb09(enable)),
        (1, 10) => pac::GPIO.pbpd().modify(|w| w.set_pb10(enable)),
        (1, 11) => pac::GPIO.pbpd().modify(|w| w.set_pb11(enable)),
        (1, 12) => pac::GPIO.pbpd().modify(|w| w.set_pb12(enable)),
        (1, 13) => pac::GPIO.pbpd().modify(|w| w.set_pb13(enable)),
        (1, 14) => pac::GPIO.pbpd().modify(|w| w.set_pb14(enable)),
        (1, 15) => pac::GPIO.pbpd().modify(|w| w.set_pb15(enable)),
        (2, 0) => pac::GPIO.pcpd().modify(|w| w.set_pc00(enable)),
        (2, 1) => pac::GPIO.pcpd().modify(|w| w.set_pc01(enable)),
        (2, 2) => pac::GPIO.pcpd().modify(|w| w.set_pc02(enable)),
        (2, 3) => pac::GPIO.pcpd().modify(|w| w.set_pc03(enable)),
        (2, 4) => pac::GPIO.pcpd().modify(|w| w.set_pc04(enable)),
        (2, 5) => pac::GPIO.pcpd().modify(|w| w.set_pc05(enable)),
        (2, 6) => pac::GPIO.pcpd().modify(|w| w.set_pc06(enable)),
        (2, 7) => pac::GPIO.pcpd().modify(|w| w.set_pc07(enable)),
        (2, 8) => pac::GPIO.pcpd().modify(|w| w.set_pc08(enable)),
        (2, 9) => pac::GPIO.pcpd().modify(|w| w.set_pc09(enable)),
        (2, 10) => pac::GPIO.pcpd().modify(|w| w.set_pc10(enable)),
        (2, 11) => pac::GPIO.pcpd().modify(|w| w.set_pc11(enable)),
        (2, 12) => pac::GPIO.pcpd().modify(|w| w.set_pc12(enable)),
        (2, 13) => pac::GPIO.pcpd().modify(|w| w.set_pc13(enable)),
        (2, 14) => pac::GPIO.pcpd().modify(|w| w.set_pc14(enable)),
        (2, 15) => pac::GPIO.pcpd().modify(|w| w.set_pc15(enable)),
        (3, 0) => pac::GPIO.pdpd().modify(|w| w.set_pd00(enable)),
        (3, 1) => pac::GPIO.pdpd().modify(|w| w.set_pd01(enable)),
        (3, 2) => pac::GPIO.pdpd().modify(|w| w.set_pd02(enable)),
        (3, 3) => pac::GPIO.pdpd().modify(|w| w.set_pd03(enable)),
        (3, 4) => pac::GPIO.pdpd().modify(|w| w.set_pd04(enable)),
        (3, 5) => pac::GPIO.pdpd().modify(|w| w.set_pd05(enable)),
        (3, 6) => pac::GPIO.pdpd().modify(|w| w.set_pd06(enable)),
        (3, 7) => pac::GPIO.pdpd().modify(|w| w.set_pd07(enable)),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 开漏使能位（PxOD）。
#[inline(always)]
pub(crate) fn set_od(n: PinId, enable: bool) {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.paod().modify(|w| w.set_pa00(enable)),
        (0, 1) => pac::GPIO.paod().modify(|w| w.set_pa01(enable)),
        (0, 2) => pac::GPIO.paod().modify(|w| w.set_pa02(enable)),
        (0, 3) => pac::GPIO.paod().modify(|w| w.set_pa03(enable)),
        (0, 4) => pac::GPIO.paod().modify(|w| w.set_pa04(enable)),
        (0, 5) => pac::GPIO.paod().modify(|w| w.set_pa05(enable)),
        (0, 6) => pac::GPIO.paod().modify(|w| w.set_pa06(enable)),
        (0, 7) => pac::GPIO.paod().modify(|w| w.set_pa07(enable)),
        (0, 8) => pac::GPIO.paod().modify(|w| w.set_pa08(enable)),
        (0, 9) => pac::GPIO.paod().modify(|w| w.set_pa09(enable)),
        (0, 10) => pac::GPIO.paod().modify(|w| w.set_pa10(enable)),
        (0, 11) => pac::GPIO.paod().modify(|w| w.set_pa11(enable)),
        (0, 12) => pac::GPIO.paod().modify(|w| w.set_pa12(enable)),
        (0, 13) => pac::GPIO.paod().modify(|w| w.set_pa13(enable)),
        (0, 14) => pac::GPIO.paod().modify(|w| w.set_pa14(enable)),
        (0, 15) => pac::GPIO.paod().modify(|w| w.set_pa15(enable)),
        (1, 0) => pac::GPIO.pbod().modify(|w| w.set_pb00(enable)),
        (1, 1) => pac::GPIO.pbod().modify(|w| w.set_pb01(enable)),
        (1, 2) => pac::GPIO.pbod().modify(|w| w.set_pb02(enable)),
        (1, 3) => pac::GPIO.pbod().modify(|w| w.set_pb03(enable)),
        (1, 4) => pac::GPIO.pbod().modify(|w| w.set_pb04(enable)),
        (1, 5) => pac::GPIO.pbod().modify(|w| w.set_pb05(enable)),
        (1, 6) => pac::GPIO.pbod().modify(|w| w.set_pb06(enable)),
        (1, 7) => pac::GPIO.pbod().modify(|w| w.set_pb07(enable)),
        (1, 8) => pac::GPIO.pbod().modify(|w| w.set_pb08(enable)),
        (1, 9) => pac::GPIO.pbod().modify(|w| w.set_pb09(enable)),
        (1, 10) => pac::GPIO.pbod().modify(|w| w.set_pb10(enable)),
        (1, 11) => pac::GPIO.pbod().modify(|w| w.set_pb11(enable)),
        (1, 12) => pac::GPIO.pbod().modify(|w| w.set_pb12(enable)),
        (1, 13) => pac::GPIO.pbod().modify(|w| w.set_pb13(enable)),
        (1, 14) => pac::GPIO.pbod().modify(|w| w.set_pb14(enable)),
        (1, 15) => pac::GPIO.pbod().modify(|w| w.set_pb15(enable)),
        (2, 0) => pac::GPIO.pcod().modify(|w| w.set_pc00(enable)),
        (2, 1) => pac::GPIO.pcod().modify(|w| w.set_pc01(enable)),
        (2, 2) => pac::GPIO.pcod().modify(|w| w.set_pc02(enable)),
        (2, 3) => pac::GPIO.pcod().modify(|w| w.set_pc03(enable)),
        (2, 4) => pac::GPIO.pcod().modify(|w| w.set_pc04(enable)),
        (2, 5) => pac::GPIO.pcod().modify(|w| w.set_pc05(enable)),
        (2, 6) => pac::GPIO.pcod().modify(|w| w.set_pc06(enable)),
        (2, 7) => pac::GPIO.pcod().modify(|w| w.set_pc07(enable)),
        (2, 8) => pac::GPIO.pcod().modify(|w| w.set_pc08(enable)),
        (2, 9) => pac::GPIO.pcod().modify(|w| w.set_pc09(enable)),
        (2, 10) => pac::GPIO.pcod().modify(|w| w.set_pc10(enable)),
        (2, 11) => pac::GPIO.pcod().modify(|w| w.set_pc11(enable)),
        (2, 12) => pac::GPIO.pcod().modify(|w| w.set_pc12(enable)),
        (2, 13) => pac::GPIO.pcod().modify(|w| w.set_pc13(enable)),
        (2, 14) => pac::GPIO.pcod().modify(|w| w.set_pc14(enable)),
        (2, 15) => pac::GPIO.pcod().modify(|w| w.set_pc15(enable)),
        (3, 0) => pac::GPIO.pdod().modify(|w| w.set_pd00(enable)),
        (3, 1) => pac::GPIO.pdod().modify(|w| w.set_pd01(enable)),
        (3, 2) => pac::GPIO.pdod().modify(|w| w.set_pd02(enable)),
        (3, 3) => pac::GPIO.pdod().modify(|w| w.set_pd03(enable)),
        (3, 4) => pac::GPIO.pdod().modify(|w| w.set_pd04(enable)),
        (3, 5) => pac::GPIO.pdod().modify(|w| w.set_pd05(enable)),
        (3, 6) => pac::GPIO.pdod().modify(|w| w.set_pd06(enable)),
        (3, 7) => pac::GPIO.pdod().modify(|w| w.set_pd07(enable)),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 驱动能力位（PxDR）：`true` = 低驱动。
#[inline(always)]
pub(crate) fn set_dr(n: PinId, low_drive: bool) {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.padr().modify(|w| w.set_pa00(low_drive)),
        (0, 1) => pac::GPIO.padr().modify(|w| w.set_pa01(low_drive)),
        (0, 2) => pac::GPIO.padr().modify(|w| w.set_pa02(low_drive)),
        (0, 3) => pac::GPIO.padr().modify(|w| w.set_pa03(low_drive)),
        (0, 4) => pac::GPIO.padr().modify(|w| w.set_pa04(low_drive)),
        (0, 5) => pac::GPIO.padr().modify(|w| w.set_pa05(low_drive)),
        (0, 6) => pac::GPIO.padr().modify(|w| w.set_pa06(low_drive)),
        (0, 7) => pac::GPIO.padr().modify(|w| w.set_pa07(low_drive)),
        (0, 8) => pac::GPIO.padr().modify(|w| w.set_pa08(low_drive)),
        (0, 9) => pac::GPIO.padr().modify(|w| w.set_pa09(low_drive)),
        (0, 10) => pac::GPIO.padr().modify(|w| w.set_pa10(low_drive)),
        (0, 11) => pac::GPIO.padr().modify(|w| w.set_pa11(low_drive)),
        (0, 12) => pac::GPIO.padr().modify(|w| w.set_pa12(low_drive)),
        (0, 13) => pac::GPIO.padr().modify(|w| w.set_pa13(low_drive)),
        (0, 14) => pac::GPIO.padr().modify(|w| w.set_pa14(low_drive)),
        (0, 15) => pac::GPIO.padr().modify(|w| w.set_pa15(low_drive)),
        (1, 0) => pac::GPIO.pbdr().modify(|w| w.set_pb00(low_drive)),
        (1, 1) => pac::GPIO.pbdr().modify(|w| w.set_pb01(low_drive)),
        (1, 2) => pac::GPIO.pbdr().modify(|w| w.set_pb02(low_drive)),
        (1, 3) => pac::GPIO.pbdr().modify(|w| w.set_pb03(low_drive)),
        (1, 4) => pac::GPIO.pbdr().modify(|w| w.set_pb04(low_drive)),
        (1, 5) => pac::GPIO.pbdr().modify(|w| w.set_pb05(low_drive)),
        (1, 6) => pac::GPIO.pbdr().modify(|w| w.set_pb06(low_drive)),
        (1, 7) => pac::GPIO.pbdr().modify(|w| w.set_pb07(low_drive)),
        (1, 8) => pac::GPIO.pbdr().modify(|w| w.set_pb08(low_drive)),
        (1, 9) => pac::GPIO.pbdr().modify(|w| w.set_pb09(low_drive)),
        (1, 10) => pac::GPIO.pbdr().modify(|w| w.set_pb10(low_drive)),
        (1, 11) => pac::GPIO.pbdr().modify(|w| w.set_pb11(low_drive)),
        (1, 12) => pac::GPIO.pbdr().modify(|w| w.set_pb12(low_drive)),
        (1, 13) => pac::GPIO.pbdr().modify(|w| w.set_pb13(low_drive)),
        (1, 14) => pac::GPIO.pbdr().modify(|w| w.set_pb14(low_drive)),
        (1, 15) => pac::GPIO.pbdr().modify(|w| w.set_pb15(low_drive)),
        (2, 0) => pac::GPIO.pcdr().modify(|w| w.set_pc00(low_drive)),
        (2, 1) => pac::GPIO.pcdr().modify(|w| w.set_pc01(low_drive)),
        (2, 2) => pac::GPIO.pcdr().modify(|w| w.set_pc02(low_drive)),
        (2, 3) => pac::GPIO.pcdr().modify(|w| w.set_pc03(low_drive)),
        (2, 4) => pac::GPIO.pcdr().modify(|w| w.set_pc04(low_drive)),
        (2, 5) => pac::GPIO.pcdr().modify(|w| w.set_pc05(low_drive)),
        (2, 6) => pac::GPIO.pcdr().modify(|w| w.set_pc06(low_drive)),
        (2, 7) => pac::GPIO.pcdr().modify(|w| w.set_pc07(low_drive)),
        (2, 8) => pac::GPIO.pcdr().modify(|w| w.set_pc08(low_drive)),
        (2, 9) => pac::GPIO.pcdr().modify(|w| w.set_pc09(low_drive)),
        (2, 10) => pac::GPIO.pcdr().modify(|w| w.set_pc10(low_drive)),
        (2, 11) => pac::GPIO.pcdr().modify(|w| w.set_pc11(low_drive)),
        (2, 12) => pac::GPIO.pcdr().modify(|w| w.set_pc12(low_drive)),
        (2, 13) => pac::GPIO.pcdr().modify(|w| w.set_pc13(low_drive)),
        (2, 14) => pac::GPIO.pcdr().modify(|w| w.set_pc14(low_drive)),
        (2, 15) => pac::GPIO.pcdr().modify(|w| w.set_pc15(low_drive)),
        (3, 0) => pac::GPIO.pddr().modify(|w| w.set_pd00(low_drive)),
        (3, 1) => pac::GPIO.pddr().modify(|w| w.set_pd01(low_drive)),
        (3, 2) => pac::GPIO.pddr().modify(|w| w.set_pd02(low_drive)),
        (3, 3) => pac::GPIO.pddr().modify(|w| w.set_pd03(low_drive)),
        (3, 4) => pac::GPIO.pddr().modify(|w| w.set_pd04(low_drive)),
        (3, 5) => pac::GPIO.pddr().modify(|w| w.set_pd05(low_drive)),
        (3, 6) => pac::GPIO.pddr().modify(|w| w.set_pd06(low_drive)),
        (3, 7) => pac::GPIO.pddr().modify(|w| w.set_pd07(low_drive)),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 模拟/数字选择位（PxADS）：`true` = 模拟（RM 6.5.1.20，复位值 0 全
/// 数字）。
#[inline(always)]
pub(crate) fn set_ads(n: PinId, analog: bool) {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.paads().modify(|w| w.set_pa00(analog)),
        (0, 1) => pac::GPIO.paads().modify(|w| w.set_pa01(analog)),
        (0, 2) => pac::GPIO.paads().modify(|w| w.set_pa02(analog)),
        (0, 3) => pac::GPIO.paads().modify(|w| w.set_pa03(analog)),
        (0, 4) => pac::GPIO.paads().modify(|w| w.set_pa04(analog)),
        (0, 5) => pac::GPIO.paads().modify(|w| w.set_pa05(analog)),
        (0, 6) => pac::GPIO.paads().modify(|w| w.set_pa06(analog)),
        (0, 7) => pac::GPIO.paads().modify(|w| w.set_pa07(analog)),
        (0, 8) => pac::GPIO.paads().modify(|w| w.set_pa08(analog)),
        (0, 9) => pac::GPIO.paads().modify(|w| w.set_pa09(analog)),
        (0, 10) => pac::GPIO.paads().modify(|w| w.set_pa10(analog)),
        (0, 11) => pac::GPIO.paads().modify(|w| w.set_pa11(analog)),
        (0, 12) => pac::GPIO.paads().modify(|w| w.set_pa12(analog)),
        (0, 13) => pac::GPIO.paads().modify(|w| w.set_pa13(analog)),
        (0, 14) => pac::GPIO.paads().modify(|w| w.set_pa14(analog)),
        (0, 15) => pac::GPIO.paads().modify(|w| w.set_pa15(analog)),
        (1, 0) => pac::GPIO.pbads().modify(|w| w.set_pb00(analog)),
        (1, 1) => pac::GPIO.pbads().modify(|w| w.set_pb01(analog)),
        (1, 2) => pac::GPIO.pbads().modify(|w| w.set_pb02(analog)),
        (1, 3) => pac::GPIO.pbads().modify(|w| w.set_pb03(analog)),
        (1, 4) => pac::GPIO.pbads().modify(|w| w.set_pb04(analog)),
        (1, 5) => pac::GPIO.pbads().modify(|w| w.set_pb05(analog)),
        (1, 6) => pac::GPIO.pbads().modify(|w| w.set_pb06(analog)),
        (1, 7) => pac::GPIO.pbads().modify(|w| w.set_pb07(analog)),
        (1, 8) => pac::GPIO.pbads().modify(|w| w.set_pb08(analog)),
        (1, 9) => pac::GPIO.pbads().modify(|w| w.set_pb09(analog)),
        (1, 10) => pac::GPIO.pbads().modify(|w| w.set_pb10(analog)),
        (1, 11) => pac::GPIO.pbads().modify(|w| w.set_pb11(analog)),
        (1, 12) => pac::GPIO.pbads().modify(|w| w.set_pb12(analog)),
        (1, 13) => pac::GPIO.pbads().modify(|w| w.set_pb13(analog)),
        (1, 14) => pac::GPIO.pbads().modify(|w| w.set_pb14(analog)),
        (1, 15) => pac::GPIO.pbads().modify(|w| w.set_pb15(analog)),
        (2, 0) => pac::GPIO.pcads().modify(|w| w.set_pc00(analog)),
        (2, 1) => pac::GPIO.pcads().modify(|w| w.set_pc01(analog)),
        (2, 2) => pac::GPIO.pcads().modify(|w| w.set_pc02(analog)),
        (2, 3) => pac::GPIO.pcads().modify(|w| w.set_pc03(analog)),
        (2, 4) => pac::GPIO.pcads().modify(|w| w.set_pc04(analog)),
        (2, 5) => pac::GPIO.pcads().modify(|w| w.set_pc05(analog)),
        (2, 6) => pac::GPIO.pcads().modify(|w| w.set_pc06(analog)),
        (2, 7) => pac::GPIO.pcads().modify(|w| w.set_pc07(analog)),
        (2, 8) => pac::GPIO.pcads().modify(|w| w.set_pc08(analog)),
        (2, 9) => pac::GPIO.pcads().modify(|w| w.set_pc09(analog)),
        (2, 10) => pac::GPIO.pcads().modify(|w| w.set_pc10(analog)),
        (2, 11) => pac::GPIO.pcads().modify(|w| w.set_pc11(analog)),
        (2, 12) => pac::GPIO.pcads().modify(|w| w.set_pc12(analog)),
        (2, 13) => pac::GPIO.pcads().modify(|w| w.set_pc13(analog)),
        (2, 14) => pac::GPIO.pcads().modify(|w| w.set_pc14(analog)),
        (2, 15) => pac::GPIO.pcads().modify(|w| w.set_pc15(analog)),
        (3, 0) => pac::GPIO.pdads().modify(|w| w.set_pd00(analog)),
        (3, 1) => pac::GPIO.pdads().modify(|w| w.set_pd01(analog)),
        (3, 2) => pac::GPIO.pdads().modify(|w| w.set_pd02(analog)),
        (3, 3) => pac::GPIO.pdads().modify(|w| w.set_pd03(analog)),
        (3, 4) => pac::GPIO.pdads().modify(|w| w.set_pd04(analog)),
        (3, 5) => pac::GPIO.pdads().modify(|w| w.set_pd05(analog)),
        (3, 6) => pac::GPIO.pdads().modify(|w| w.set_pd06(analog)),
        (3, 7) => pac::GPIO.pdads().modify(|w| w.set_pd07(analog)),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 复用功能选择（Pxy_SEL）：0 = GPIO 功能，1–7 为复用功能（RM 6.5.1.1）。
#[inline(always)]
pub(crate) fn set_sel(n: PinId, func: u8) {
    match (n.port, n.pin) {
        (0, 0) => pac::GPIO.pa00_sel().modify(|w| w.set_sel(func)),
        (0, 1) => pac::GPIO.pa01_sel().modify(|w| w.set_sel(func)),
        (0, 2) => pac::GPIO.pa02_sel().modify(|w| w.set_sel(func)),
        (0, 3) => pac::GPIO.pa03_sel().modify(|w| w.set_sel(func)),
        (0, 4) => pac::GPIO.pa04_sel().modify(|w| w.set_sel(func)),
        (0, 5) => pac::GPIO.pa05_sel().modify(|w| w.set_sel(func)),
        (0, 6) => pac::GPIO.pa06_sel().modify(|w| w.set_sel(func)),
        (0, 7) => pac::GPIO.pa07_sel().modify(|w| w.set_sel(func)),
        (0, 8) => pac::GPIO.pa08_sel().modify(|w| w.set_sel(func)),
        (0, 9) => pac::GPIO.pa09_sel().modify(|w| w.set_sel(func)),
        (0, 10) => pac::GPIO.pa10_sel().modify(|w| w.set_sel(func)),
        (0, 11) => pac::GPIO.pa11_sel().modify(|w| w.set_sel(func)),
        (0, 12) => pac::GPIO.pa12_sel().modify(|w| w.set_sel(func)),
        (0, 13) => pac::GPIO.pa13_sel().modify(|w| w.set_sel(func)),
        (0, 14) => pac::GPIO.pa14_sel().modify(|w| w.set_sel(func)),
        (0, 15) => pac::GPIO.pa15_sel().modify(|w| w.set_sel(func)),
        (1, 0) => pac::GPIO.pb00_sel().modify(|w| w.set_sel(func)),
        (1, 1) => pac::GPIO.pb01_sel().modify(|w| w.set_sel(func)),
        (1, 2) => pac::GPIO.pb02_sel().modify(|w| w.set_sel(func)),
        (1, 3) => pac::GPIO.pb03_sel().modify(|w| w.set_sel(func)),
        (1, 4) => pac::GPIO.pb04_sel().modify(|w| w.set_sel(func)),
        (1, 5) => pac::GPIO.pb05_sel().modify(|w| w.set_sel(func)),
        (1, 6) => pac::GPIO.pb06_sel().modify(|w| w.set_sel(func)),
        (1, 7) => pac::GPIO.pb07_sel().modify(|w| w.set_sel(func)),
        (1, 8) => pac::GPIO.pb08_sel().modify(|w| w.set_sel(func)),
        (1, 9) => pac::GPIO.pb09_sel().modify(|w| w.set_sel(func)),
        (1, 10) => pac::GPIO.pb10_sel().modify(|w| w.set_sel(func)),
        (1, 11) => pac::GPIO.pb11_sel().modify(|w| w.set_sel(func)),
        (1, 12) => pac::GPIO.pb12_sel().modify(|w| w.set_sel(func)),
        (1, 13) => pac::GPIO.pb13_sel().modify(|w| w.set_sel(func)),
        (1, 14) => pac::GPIO.pb14_sel().modify(|w| w.set_sel(func)),
        (1, 15) => pac::GPIO.pb15_sel().modify(|w| w.set_sel(func)),
        (2, 0) => pac::GPIO.pc00_sel().modify(|w| w.set_sel(func)),
        (2, 1) => pac::GPIO.pc01_sel().modify(|w| w.set_sel(func)),
        (2, 2) => pac::GPIO.pc02_sel().modify(|w| w.set_sel(func)),
        (2, 3) => pac::GPIO.pc03_sel().modify(|w| w.set_sel(func)),
        (2, 4) => pac::GPIO.pc04_sel().modify(|w| w.set_sel(func)),
        (2, 5) => pac::GPIO.pc05_sel().modify(|w| w.set_sel(func)),
        (2, 6) => pac::GPIO.pc06_sel().modify(|w| w.set_sel(func)),
        (2, 7) => pac::GPIO.pc07_sel().modify(|w| w.set_sel(func)),
        (2, 8) => pac::GPIO.pc08_sel().modify(|w| w.set_sel(func)),
        (2, 9) => pac::GPIO.pc09_sel().modify(|w| w.set_sel(func)),
        (2, 10) => pac::GPIO.pc10_sel().modify(|w| w.set_sel(func)),
        (2, 11) => pac::GPIO.pc11_sel().modify(|w| w.set_sel(func)),
        (2, 12) => pac::GPIO.pc12_sel().modify(|w| w.set_sel(func)),
        (2, 13) => pac::GPIO.pc13_sel().modify(|w| w.set_sel(func)),
        (2, 14) => pac::GPIO.pc14_sel().modify(|w| w.set_sel(func)),
        (2, 15) => pac::GPIO.pc15_sel().modify(|w| w.set_sel(func)),
        (3, 0) => pac::GPIO.pd00_sel().modify(|w| w.set_sel(func)),
        (3, 1) => pac::GPIO.pd01_sel().modify(|w| w.set_sel(func)),
        (3, 2) => pac::GPIO.pd02_sel().modify(|w| w.set_sel(func)),
        (3, 3) => pac::GPIO.pd03_sel().modify(|w| w.set_sel(func)),
        (3, 4) => pac::GPIO.pd04_sel().modify(|w| w.set_sel(func)),
        (3, 5) => pac::GPIO.pd05_sel().modify(|w| w.set_sel(func)),
        (3, 6) => pac::GPIO.pd06_sel().modify(|w| w.set_sel(func)),
        (3, 7) => pac::GPIO.pd07_sel().modify(|w| w.set_sel(func)),
        _ => unreachable!("GPIO v3_3 后端无此引脚"),
    }
}

/// 未引出引脚的上电处理（HC32L13x 各封装专用）。
///
/// DS 3.1 表下注与 RM 6.3 要求：应用中需将封装未引出的 IO 设为数字输入
/// 并使能上拉（芯片复位后端口为高阻输入，未引出脚悬空会漏电）。SDK
/// `SystemInit` 的 `_HidePinInit`（mcu/common/system_hc32l13x.c）按封装
/// 宏对未引出脚清 ADS、置 DIR、置 PU；方向寄存器复位值已为全输入
/// （PxDIR 复位 0xFFFF，RM 6.5.1.17），RM 未说明复位后 PxADS/PxPU 是否
/// 恒 0，SDK 仍显式清零 ADS，本驱动沿用其完整序列。
///
/// 掩码与 SDK 逐封装核对（SDK 行内注释"PC00~PC12""PD02/04/06"与掩码
/// 0x3FFF/0x00F4 不符处以掩码为准，掩码与 DS 表 3-2 一致）。
///
/// 门控已由调用方（[`crate::init`]）打开并恢复。各料号经 `hc32l13x-*pin`
/// 封装 feature 调用对应分支；64 引脚封装（K8TA）全部 IO 引出，SDK
/// `_HidePinInit` 的 64PIN 分支为空，三个分支均不编译。
#[cfg(feature = "hc32l13x")]
pub(crate) fn pull_up_unbonded() {
    // 掩码以 (寄存器位值) 表达：下述三组各封装互斥（feature 保证）。
    #[cfg(feature = "hc32l13x-28pin")]
    {
        // 28 脚（E8PA TSSOP28）：PA15、PB03~PB15、PC00~PC13、
        // PD02/PD04~PD07 未引出（DS 表 3-2 28 列空白处）。
        pac::GPIO.paads().modify(|w| w.set_pa15(false));
        pac::GPIO.pbads().modify(|w| {
            w.set_pb03(false);
            w.set_pb04(false);
            w.set_pb05(false);
            w.set_pb06(false);
            w.set_pb07(false);
            w.set_pb08(false);
            w.set_pb09(false);
            w.set_pb10(false);
            w.set_pb11(false);
            w.set_pb12(false);
            w.set_pb13(false);
            w.set_pb14(false);
            w.set_pb15(false);
        });
        pac::GPIO.pcads().modify(|w| {
            w.set_pc00(false);
            w.set_pc01(false);
            w.set_pc02(false);
            w.set_pc03(false);
            w.set_pc04(false);
            w.set_pc05(false);
            w.set_pc06(false);
            w.set_pc07(false);
            w.set_pc08(false);
            w.set_pc09(false);
            w.set_pc10(false);
            w.set_pc11(false);
            w.set_pc12(false);
            w.set_pc13(false);
        });
        pac::GPIO.pdads().modify(|w| {
            w.set_pd02(false);
            w.set_pd04(false);
            w.set_pd05(false);
            w.set_pd06(false);
            w.set_pd07(false);
        });
        pac::GPIO.papu().modify(|w| w.set_pa15(true));
        pac::GPIO.pbpu().modify(|w| {
            w.set_pb03(true);
            w.set_pb04(true);
            w.set_pb05(true);
            w.set_pb06(true);
            w.set_pb07(true);
            w.set_pb08(true);
            w.set_pb09(true);
            w.set_pb10(true);
            w.set_pb11(true);
            w.set_pb12(true);
            w.set_pb13(true);
            w.set_pb14(true);
            w.set_pb15(true);
        });
        pac::GPIO.pcpu().modify(|w| {
            w.set_pc00(true);
            w.set_pc01(true);
            w.set_pc02(true);
            w.set_pc03(true);
            w.set_pc04(true);
            w.set_pc05(true);
            w.set_pc06(true);
            w.set_pc07(true);
            w.set_pc08(true);
            w.set_pc09(true);
            w.set_pc10(true);
            w.set_pc11(true);
            w.set_pc12(true);
            w.set_pc13(true);
        });
        pac::GPIO.pdpu().modify(|w| {
            w.set_pd02(true);
            w.set_pd04(true);
            w.set_pd05(true);
            w.set_pd06(true);
            w.set_pd07(true);
        });
    }
    #[cfg(feature = "hc32l13x-32pin")]
    {
        // 32 脚（F8UA QFN32）：PA00/PA01/PA03、
        // PB02/PB08/PB09/PB10/PB12~PB15、PC00~PC13、PD02/PD04~PD07 未引出。
        pac::GPIO.paads().modify(|w| {
            w.set_pa00(false);
            w.set_pa01(false);
            w.set_pa03(false);
        });
        pac::GPIO.pbads().modify(|w| {
            w.set_pb02(false);
            w.set_pb08(false);
            w.set_pb09(false);
            w.set_pb10(false);
            w.set_pb12(false);
            w.set_pb13(false);
            w.set_pb14(false);
            w.set_pb15(false);
        });
        pac::GPIO.pcads().modify(|w| {
            w.set_pc00(false);
            w.set_pc01(false);
            w.set_pc02(false);
            w.set_pc03(false);
            w.set_pc04(false);
            w.set_pc05(false);
            w.set_pc06(false);
            w.set_pc07(false);
            w.set_pc08(false);
            w.set_pc09(false);
            w.set_pc10(false);
            w.set_pc11(false);
            w.set_pc12(false);
            w.set_pc13(false);
        });
        pac::GPIO.pdads().modify(|w| {
            w.set_pd02(false);
            w.set_pd04(false);
            w.set_pd05(false);
            w.set_pd06(false);
            w.set_pd07(false);
        });
        pac::GPIO.papu().modify(|w| {
            w.set_pa00(true);
            w.set_pa01(true);
            w.set_pa03(true);
        });
        pac::GPIO.pbpu().modify(|w| {
            w.set_pb02(true);
            w.set_pb08(true);
            w.set_pb09(true);
            w.set_pb10(true);
            w.set_pb12(true);
            w.set_pb13(true);
            w.set_pb14(true);
            w.set_pb15(true);
        });
        pac::GPIO.pcpu().modify(|w| {
            w.set_pc00(true);
            w.set_pc01(true);
            w.set_pc02(true);
            w.set_pc03(true);
            w.set_pc04(true);
            w.set_pc05(true);
            w.set_pc06(true);
            w.set_pc07(true);
            w.set_pc08(true);
            w.set_pc09(true);
            w.set_pc10(true);
            w.set_pc11(true);
            w.set_pc12(true);
            w.set_pc13(true);
        });
        pac::GPIO.pdpu().modify(|w| {
            w.set_pd02(true);
            w.set_pd04(true);
            w.set_pd05(true);
            w.set_pd06(true);
            w.set_pd07(true);
        });
    }
    #[cfg(feature = "hc32l13x-48pin")]
    {
        // 48 脚（L130 J8TA LQFP48 / J8UA QFN48、L136 J8TA LQFP48）：
        // PC00~PC12、PD02/PD04/PD05 未引出。
        pac::GPIO.pcads().modify(|w| {
            w.set_pc00(false);
            w.set_pc01(false);
            w.set_pc02(false);
            w.set_pc03(false);
            w.set_pc04(false);
            w.set_pc05(false);
            w.set_pc06(false);
            w.set_pc07(false);
            w.set_pc08(false);
            w.set_pc09(false);
            w.set_pc10(false);
            w.set_pc11(false);
            w.set_pc12(false);
        });
        pac::GPIO.pdads().modify(|w| {
            w.set_pd02(false);
            w.set_pd04(false);
            w.set_pd05(false);
        });
        pac::GPIO.pcpu().modify(|w| {
            w.set_pc00(true);
            w.set_pc01(true);
            w.set_pc02(true);
            w.set_pc03(true);
            w.set_pc04(true);
            w.set_pc05(true);
            w.set_pc06(true);
            w.set_pc07(true);
            w.set_pc08(true);
            w.set_pc09(true);
            w.set_pc10(true);
            w.set_pc11(true);
            w.set_pc12(true);
        });
        pac::GPIO.pdpu().modify(|w| {
            w.set_pd02(true);
            w.set_pd04(true);
            w.set_pd05(true);
        });
    }
}
