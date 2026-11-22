//! SYSCTRL 寄存器后端：DDL v3_1 布局（HC32L13x 的 `SYSCTRL` 外设）。
//!
//! 基址 0x4000_2000，包含 SYSCTRL0/1/2、RCH/XTH/RCL/XTL 振荡控制、
//! PERI_CLKEN 与 PLL_CR（RM 3.5.1~3.5.9）。PLL 还依赖 BGR，系统时钟
//! 切换还需同步配置 FLASH_CR.WAIT；这三处寄存器访问集中在当前后端。
//!
//! 改写 SYSCTRL0/SYSCTRL1 前必须先向 SYSCTRL2 依次写入 0x5A5A、0xA5A5
//! 解锁；一次写入完成后保护自动恢复（RM 3.5.3）。RCH_CR/RCL_CR 等振荡
//! 控制、PLL_CR 与 PERI_CLKEN 不在保护范围内，直接读-改-写。

use crate::pac;

/// 向 SYSCTRL2 依次写入解锁序列（RM 3.5.3）。
///
/// 每次改写 SYSCTRL0/SYSCTRL1 前调用；一次写入后保护自动恢复。
pub(crate) fn unlock() {
    pac::SYSCTRL.sysctrl2().write(|w| w.set_sysctrl2(0x5A5A));
    pac::SYSCTRL.sysctrl2().write(|w| w.set_sysctrl2(0xA5A5));
}

/// 解锁后读-改-写 SYSCTRL0（写完保护自动恢复，RM 3.5.3）。
pub(crate) fn write_sysctrl0(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::Sysctrl0)) {
    unlock();
    pac::SYSCTRL.sysctrl0().modify(f);
}

/// 解锁后读-改-写 SYSCTRL1（写完保护自动恢复，RM 3.5.3）。
pub(crate) fn write_sysctrl1(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::Sysctrl1)) {
    unlock();
    pac::SYSCTRL.sysctrl1().modify(f);
}

/// 读取 SYSCTRL0（时钟源选择、分频）。
pub(crate) fn read_sysctrl0() -> pac::sysctrl_ddl_v3_1::regs::Sysctrl0 {
    pac::SYSCTRL.sysctrl0().read()
}

/// 读-改-写 RCH 控制寄存器（RCH_CR 不在解锁范围，RM 3.5.4）。
pub(crate) fn modify_rch_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::RchCr)) {
    pac::SYSCTRL.rch_cr().modify(f);
}

/// 读取 RCH 控制寄存器（TRIM、STABLE）。
pub(crate) fn read_rch_cr() -> pac::sysctrl_ddl_v3_1::regs::RchCr {
    pac::SYSCTRL.rch_cr().read()
}

/// 读-改-写 XTH 控制寄存器（XTH_CR 不在解锁范围，RM 3.5.5）。
pub(crate) fn modify_xth_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::XthCr)) {
    pac::SYSCTRL.xth_cr().modify(f);
}

/// 读取 XTH 控制寄存器（STABLE 标志）。
pub(crate) fn read_xth_cr() -> pac::sysctrl_ddl_v3_1::regs::XthCr {
    pac::SYSCTRL.xth_cr().read()
}

/// 读-改-写 RCL 控制寄存器（RCL_CR 不在解锁范围，RM 3.5.6）。
pub(crate) fn modify_rcl_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::RclCr)) {
    pac::SYSCTRL.rcl_cr().modify(f);
}

/// 读取 RCL 控制寄存器（TRIM、STABLE 标志）。
pub(crate) fn read_rcl_cr() -> pac::sysctrl_ddl_v3_1::regs::RclCr {
    pac::SYSCTRL.rcl_cr().read()
}

/// 读-改-写 XTL 控制寄存器（XTL_CR 不在解锁范围，RM 3.5.7）。
pub(crate) fn modify_xtl_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::XtlCr)) {
    pac::SYSCTRL.xtl_cr().modify(f);
}

/// 读取 XTL 控制寄存器（STABLE 标志）。
pub(crate) fn read_xtl_cr() -> pac::sysctrl_ddl_v3_1::regs::XtlCr {
    pac::SYSCTRL.xtl_cr().read()
}

/// 读-改-写 PLL 控制寄存器（PLL_CR 不在解锁范围，RM 3.5.8）。
pub(crate) fn modify_pll_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::PllCr)) {
    pac::SYSCTRL.pll_cr().modify(f);
}

/// 读取 PLL 控制寄存器（REFSEL、DIVN、STABLE）。
pub(crate) fn read_pll_cr() -> pac::sysctrl_ddl_v3_1::regs::PllCr {
    pac::SYSCTRL.pll_cr().read()
}

/// 读-改-写 PERI_CLKEN（不在解锁范围，RM 3.5.9）。
///
/// 复位值 0x8080_0000：FLASH 与保留位 23 置位，HDIV（位 30）及其余
/// 门控（含位 28 GPIO）复位为关；门控关闭时对 GPIO 寄存器的写入不生效。
pub(crate) fn modify_peri_clken(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::PeriClken)) {
    pac::SYSCTRL.peri_clken().modify(f);
}

/// 读取 PERI_CLKEN（门控状态、保存恢复）。
pub(crate) fn read_peri_clken() -> pac::sysctrl_ddl_v3_1::regs::PeriClken {
    pac::SYSCTRL.peri_clken().read()
}

/// 整写 PERI_CLKEN（恢复保存的门控值）。
pub(crate) fn write_peri_clken(value: pac::sysctrl_ddl_v3_1::regs::PeriClken) {
    pac::SYSCTRL.peri_clken().write_value(value);
}

/// 设置 BGR 使能，临时打开其与 ADC 共用的 PERI_CLKEN.ADC 门控。
///
/// RM 31.5.1 要求操作 BGR_CR 前开启该门控。调用方负责在使能后等待
/// 20us；PLL 的启动顺序见 RM 3.2.7/3.2.8。PERI_CLKEN 保存-恢复非原子，
/// 与中断中的门控操作并发时需由调用方加临界区。
pub(crate) fn set_bgr(enable: bool) {
    let saved = read_peri_clken();
    modify_peri_clken(|w| w.set_adc(true));
    pac::BGR.cr().modify(|w| w.set_bgr_en(enable));
    write_peri_clken(saved);
}

/// 设置 FLASH 读取等待周期（0 或 1，RM 11.5、11.8.9）。
///
/// 临时打开 FLASH 门控，并按 RM 11.7 在关中断状态下写入 BYPASS 序列，
/// 写后回读校验，不一致时重试。FLASH 中的程序取指不受其控制器门控影响。
/// 门控的保存-恢复非原子，与中断中的门控操作并发时需由调用方加临界区。
pub(crate) fn set_flash_wait(wait: u8) {
    debug_assert!(wait <= 1);
    let saved = read_peri_clken();
    modify_peri_clken(|w| w.set_flash(true));
    if pac::FLASH.cr().read().wait() != wait {
        let was_masked: u32;
        unsafe {
            core::arch::asm!("mrs {0}, primask", out(reg) was_masked, options(nomem));
            core::arch::asm!("cpsid i", options(nomem, nostack));
        }
        for _ in 0..4 {
            pac::FLASH.bypass().write(|w| w.set_byseq(0x5A5A));
            pac::FLASH.bypass().write(|w| w.set_byseq(0xA5A5));
            pac::FLASH.cr().modify(|w| w.set_wait(wait));
            if pac::FLASH.cr().read().wait() == wait {
                break;
            }
        }
        if was_masked == 0 {
            unsafe { core::arch::asm!("cpsie i", options(nomem, nostack)) };
        }
    }
    assert!(
        pac::FLASH.cr().read().wait() == wait,
        "clock: FLASH_CR.WAIT 写入校验失败"
    );
    write_peri_clken(saved);
}

/// 默认初始化：对齐厂商启动代码 SystemInit 的默认时钟路径
/// （mcu/common/system_hc32l13x.c）。
///
/// Rust 启动路径（cortex-m-rt `Reset`）只搬运数据段并调用 `main`，不会
/// 执行任何厂商启动代码；`SystemInit` 中对应用可见的行为只有两项在
/// 复位态之上需要软件动作：
///
/// 1. 装载 RCH 出厂校准 TRIM（flash 0x00100C08，4 MHz 档）：复位值
///    RCH_CR = 0x126 是晶圆默认，SystemInit 重写为出厂校准值。复位默认
///    系统时钟源即 RCH（SYSCTRL0 复位 0x0000_0001，RM 3.5.1），此时写
///    TRIM 是否须走 RM 3.2.2 逐级序列手册未说明；SDK SystemInit 在
///    RCH_EN = 1 下直接写（4 MHz 是同一档的校准替换，非换频），本驱动
///    沿用该行为。
/// 2. 装载 RCL 出厂校准 TRIM（flash 0x00100C22，32.768 kHz 档）：RCL
///    复位为关（RCL_EN = 0），装载的 TRIM 供后续使能 RCL 时直接得到校
///    准频率，不开源、不等待稳定。
///
/// `SystemCoreClockUpdate` 配置 SysTick 属于 CMSIS 运行时，不搬运。
/// 未引出脚处理（`_HidePinInit`）与封装相关，由 [`crate::init`] 的
/// `init_pins` 按料号执行。HCLK/PCLK 分频复位值 SYSCTRL0.HCLK_PRS = 0、
/// PCLK_PRS = 0（不分频，RM 3.5.1），与 [`super::Clocks`] 默认快照一致。
pub(crate) fn init_default() {
    // safety: 芯片固定的已映射只读校准区，厂商保证有数据（RM 3.5.4/3.5.6）。
    let rch_trim = unsafe { (0x0010_0C08 as *const u16).read_volatile() };
    let rcl_trim = unsafe { (0x0010_0C22 as *const u16).read_volatile() };
    modify_rch_cr(|w| w.set_trim(rch_trim));
    modify_rcl_cr(|w| w.set_trim(rcl_trim));
}

/// 装载后 RCH 的标称输出频率（4 MHz 档，RM 3.1.1 出厂校准档之一）。
pub(crate) const DEFAULT_RCH_HZ: u32 = 4_000_000;
