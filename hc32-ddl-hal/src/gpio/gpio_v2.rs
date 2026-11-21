//! GPIO 寄存器后端：DDL v2 精简端口布局（HC32L110 等）。
//!
//! 单外设多端口：端口 P0–P3 的控制寄存器集中在一个寄存器块内，复用选择
//! 为逐引脚的 `Pxy_SEL` 寄存器。所有配置与输出寄存器都是普通的读-改-写
//! 位域，没有 BSRR 型置位/清零镜像寄存器。位域字段名即引脚名（P01…P36），
//! 与 PxDIR/PxOUT/PxIN/PxPU/PxPD/PxOD/PxDR/PxADS 的字段一一对应。
//!
//! 门控：GPIO 寄存器访问前须打开 PERI_CLKEN 位 28（复位为 0，RM 3.5.8；
//! 门控关闭时写入不生效），各入口幂等地先调用 [`enable`] 经当前型号的
//! 系统控制器打开门控。

use super::PinId;
use crate::pac;

/// 打开 GPIO 外设时钟门控，幂等。
///
/// PERI_CLKEN 位 28 复位为 0（RM 3.5.8）；门控关闭时对 GPIO 寄存器的
/// 写入不生效。公共 GPIO 初始化经由当前型号的系统控制器（clock 模块的
/// [`crate::clock::enable_gpio`]）打开本门控。
pub(crate) fn enable() {
    crate::clock::enable_gpio();
}

/// 方向位（PxDIR）：`true` = 输入，`false` = 输出。
#[inline(always)]
pub(crate) fn set_dir(n: PinId, input: bool) {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// 输出锁存位（PxOUT）：`true` = 高。
#[inline(always)]
pub(crate) fn set_out(n: PinId, high: bool) {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// 输出锁存位（PxOUT）读取：`true` = 高。
#[inline(always)]
pub(crate) fn read_out(n: PinId) -> bool {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// 输入电平位（PxIN）读取：`true` = 高。
#[inline(always)]
pub(crate) fn read_in(n: PinId) -> bool {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// 上拉使能位（PxPU）。
#[inline(always)]
pub(crate) fn set_pu(n: PinId, enable: bool) {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// 下拉使能位（PxPD）。
#[inline(always)]
pub(crate) fn set_pd(n: PinId, enable: bool) {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// 开漏使能位（PxOD）。
#[inline(always)]
pub(crate) fn set_od(n: PinId, enable: bool) {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// 驱动能力位（PxDR）：`true` = 低驱动。
#[inline(always)]
pub(crate) fn set_dr(n: PinId, low_drive: bool) {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// 模拟/数字选择位（PxADS）：`true` = 模拟。
#[inline(always)]
pub(crate) fn set_ads(n: PinId, analog: bool) {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// 复用功能选择（Pxy_SEL）：0 = GPIO 功能，1–7 为复用功能。
#[inline(always)]
pub(crate) fn set_sel(n: PinId, func: u8) {
    match (n.port, n.pin) {
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
        _ => unreachable!("GPIO v2 后端无此引脚"),
    }
}

/// XTH 晶振引脚置模拟（RM 3.2.3 Step1）。
///
/// 外部输入模式：P01 保持数字输入、P02 置模拟（RM 3.1 注意：外部输入
/// 时 P02 禁止作 GPIO）。
pub(crate) fn setup_xth_pins(bypass: bool) {
    if bypass {
        pac::GPIO.p0ads().modify(|w| {
            w.set_p01(false);
            w.set_p02(true);
        });
        pac::GPIO.p0dir().modify(|w| w.set_p01(true));
    } else {
        pac::GPIO.p0ads().modify(|w| {
            w.set_p01(true);
            w.set_p02(true);
        });
    }
}

/// XTL 晶振引脚置模拟（RM 3.2.2 Step1）。
///
/// 外部输入模式：P14 保持数字输入、P15 置模拟。
pub(crate) fn setup_xtl_pins(bypass: bool) {
    if bypass {
        pac::GPIO.p1ads().modify(|w| {
            w.set_p14(false);
            w.set_p15(true);
        });
        pac::GPIO.p1dir().modify(|w| w.set_p14(true));
    } else {
        pac::GPIO.p1ads().modify(|w| {
            w.set_p14(true);
            w.set_p15(true);
        });
    }
}

/// HCLK 波形输出到 P24（GPIO CTRL1，RM 6.5.5）。
///
/// SDK `Gpio_SetFunc_HCLKOUT_P24` 序列：ADS 清零、SEL=3、方向输出、
/// CTRL1.HCLK_SEL/HCLK_EN。
pub(crate) fn enable_hclk_output_p24(div: u8) {
    enable();
    pac::GPIO.p2ads().modify(|w| w.set_p24(false));
    pac::GPIO.p24_sel().modify(|w| w.set_sel(3));
    // PxDIR：true = 输入，false = 输出。
    pac::GPIO.p2dir().modify(|w| w.set_p24(false));
    pac::GPIO.ctrl1().modify(|w| {
        w.set_hclk_sel(div);
        w.set_hclk_en(true);
    });
}

/// 停止 P24 的 HCLK 输出（CTRL1.HCLK_EN = 0）。
pub(crate) fn disable_hclk_output_p24() {
    pac::GPIO.ctrl1().modify(|w| w.set_hclk_en(false));
}

/// PCLK 波形输出到 P31（GPIO CTRL1，RM 6.5.5）。
///
/// SDK `Gpio_SetFunc_PCLKOUT_P31` 序列：ADS 清零、SEL=3、方向输出、
/// CTRL1.PCLK_SEL/PCLK_EN；SDK 同时清 HCLK_EN——SDK 宏即如此，RM 未
/// 要求，无害保守。
pub(crate) fn enable_pclk_output_p31(div: u8) {
    enable();
    pac::GPIO.p3ads().modify(|w| w.set_p31(false));
    pac::GPIO.p31_sel().modify(|w| w.set_sel(3));
    pac::GPIO.p3dir().modify(|w| w.set_p31(false));
    pac::GPIO.ctrl1().modify(|w| {
        w.set_pclk_sel(div);
        w.set_pclk_en(true);
        w.set_hclk_en(false);
    });
}

/// 停止 P31 的 PCLK 输出（CTRL1.PCLK_EN = 0）。
pub(crate) fn disable_pclk_output_p31() {
    pac::GPIO.ctrl1().modify(|w| w.set_pclk_en(false));
}

/// 未引出引脚的上电处理（16 引脚封装专用）。
///
/// 16 引脚封装相对 TSSOP20 未引出的 IO 须设为输入并使能上拉（DS 第 4
/// 章表下注）；方向寄存器复位值已为全输入，这里只补上拉。门控已由调用
/// 方（[`crate::init`]）打开。仅由 `hc32l110-16pin` 料号带出编译。
#[cfg(feature = "hc32l110-16pin")]
pub(crate) fn pull_up_unbonded() {
    // P03 / P32 / P33 / P34（DS 第 4 章引脚表，16 引脚封装"注"位）。
    for (port, pin) in [(0, 3), (3, 2), (3, 3), (3, 4)] {
        set_pu(PinId::new(port, pin), true);
    }
}
