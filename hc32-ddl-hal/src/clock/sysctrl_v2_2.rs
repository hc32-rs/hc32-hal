//! SYSCTRL 寄存器后端：DDL v2_2 布局（HC32L110 的 `CLOCK` 外设）。
//!
//! 基址 0x4000_2000，包含 SYSCTRL0/1/2、RCH/XTH/RCL/XTL 振荡控制、
//! PERI_CLKEN、SYSTICK_CR（RM 3.5.1~3.5.9）。晶振引脚与波形输出引脚属
//! GPIO 寄存器操作，经 gpio 后端原语完成，不在此重复。
//!
//! 改写 SYSCTRL0/SYSCTRL1 前必须先向 SYSCTRL2 依次写入 0x5A5A、0xA5A5
//! 解锁；一次写入完成后保护自动恢复（RM 3.5.3）。其余寄存器（RCH_CR 等
//! 振荡控制、PERI_CLKEN、SYSTICK_CR）不在保护范围内，直接读-改-写。

use crate::pac;

/// 向 SYSCTRL2 依次写入解锁序列（RM 3.5.3）。
///
/// 每次改写 SYSCTRL0/SYSCTRL1 前调用；一次写入后保护自动恢复。
pub(crate) fn unlock() {
    pac::CLOCK.sysctrl2().write(|w| w.set_sysctrl2(0x5A5A));
    pac::CLOCK.sysctrl2().write(|w| w.set_sysctrl2(0xA5A5));
}

/// 解锁后读-改-写 SYSCTRL0（写完保护自动恢复，RM 3.5.3）。
pub(crate) fn write_sysctrl0(f: impl FnOnce(&mut pac::sysctrl_ddl_v2_2::regs::Sysctrl0)) {
    unlock();
    pac::CLOCK.sysctrl0().modify(f);
}

/// 解锁后读-改-写 SYSCTRL1（写完保护自动恢复，RM 3.5.3）。
pub(crate) fn write_sysctrl1(f: impl FnOnce(&mut pac::sysctrl_ddl_v2_2::regs::Sysctrl1)) {
    unlock();
    pac::CLOCK.sysctrl1().modify(f);
}

/// 读取 SYSCTRL0（时钟源选择、分频）。
pub(crate) fn read_sysctrl0() -> pac::sysctrl_ddl_v2_2::regs::Sysctrl0 {
    pac::CLOCK.sysctrl0().read()
}

/// 读-改-写 RCH 控制寄存器（RCH_CR 不在解锁范围，RM 3.5.4）。
pub(crate) fn modify_rch_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v2_2::regs::RchCr)) {
    pac::CLOCK.rch_cr().modify(f);
}

/// 读取 RCH 控制寄存器（TRIM、STABLE）。
pub(crate) fn read_rch_cr() -> pac::sysctrl_ddl_v2_2::regs::RchCr {
    pac::CLOCK.rch_cr().read()
}

/// 读-改-写 XTH 控制寄存器（XTH_CR 不在解锁范围，RM 3.5.5）。
pub(crate) fn modify_xth_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v2_2::regs::XthCr)) {
    pac::CLOCK.xth_cr().modify(f);
}

/// 读取 XTH 控制寄存器（STABLE 标志）。
pub(crate) fn read_xth_cr() -> pac::sysctrl_ddl_v2_2::regs::XthCr {
    pac::CLOCK.xth_cr().read()
}

/// 读-改-写 RCL 控制寄存器（RCL_CR 不在解锁范围，RM 3.5.6）。
pub(crate) fn modify_rcl_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v2_2::regs::RclCr)) {
    pac::CLOCK.rcl_cr().modify(f);
}

/// 读取 RCL 控制寄存器（STABLE 标志）。
pub(crate) fn read_rcl_cr() -> pac::sysctrl_ddl_v2_2::regs::RclCr {
    pac::CLOCK.rcl_cr().read()
}

/// 读-改-写 XTL 控制寄存器（XTL_CR 不在解锁范围，RM 3.5.7）。
pub(crate) fn modify_xtl_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v2_2::regs::XtlCr)) {
    pac::CLOCK.xtl_cr().modify(f);
}

/// 读取 XTL 控制寄存器（STABLE 标志）。
pub(crate) fn read_xtl_cr() -> pac::sysctrl_ddl_v2_2::regs::XtlCr {
    pac::CLOCK.xtl_cr().read()
}

/// 读-改-写 PERI_CLKEN（不在解锁范围，RM 3.5.8）。
pub(crate) fn modify_peri_clken(f: impl FnOnce(&mut pac::sysctrl_ddl_v2_2::regs::PeriClken)) {
    pac::CLOCK.peri_clken().modify(f);
}

/// 读取 PERI_CLKEN（门控状态、保存恢复）。
pub(crate) fn read_peri_clken() -> pac::sysctrl_ddl_v2_2::regs::PeriClken {
    pac::CLOCK.peri_clken().read()
}

/// 整写 PERI_CLKEN（恢复保存的门控值）。
pub(crate) fn write_peri_clken(value: pac::sysctrl_ddl_v2_2::regs::PeriClken) {
    pac::CLOCK.peri_clken().write_value(value);
}

/// 读-改-写 SYSTICK_CR（RM 3.5.9）。
pub(crate) fn modify_systick_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v2_2::regs::SystickCr)) {
    pac::CLOCK.systick_cr().modify(f);
}

/// 读取 SYSTICK_CR（STCALIB）。
pub(crate) fn read_systick_cr() -> pac::sysctrl_ddl_v2_2::regs::SystickCr {
    pac::CLOCK.systick_cr().read()
}
