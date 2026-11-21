//! SYSCTRL 寄存器后端：DDL v3_1 布局（HC32L13x 的 `SYSCTRL` 外设）。
//!
//! 基址 0x4000_2000，包含 SYSCTRL0/1/2、RCH/XTH/RCL/XTL 振荡控制、
//! PERI_CLKEN 与 PLL_CR（RM 3.5.1~3.5.9）。本后端只实现当前型号需要的
//! 默认初始化路径：装载出厂 TRIM、GPIO 门控与频率快照默认值（AGENTS.md
//! 「仅接入默认时钟的型号只开放已实现的时钟能力」）；可配置时钟树
//! （RM 3.2 切换流程、PLL）待后续按 3.2 章流程接入时再扩展。
//!
//! 改写 SYSCTRL0/SYSCTRL1 前必须先向 SYSCTRL2 依次写入 0x5A5A、0xA5A5
//! 解锁；一次写入完成后保护自动恢复（RM 3.5.3）。RCH_CR/RCL_CR 等振荡
//! 控制与 PERI_CLKEN 不在保护范围内，直接读-改-写。

use crate::pac;

/// 向 SYSCTRL2 依次写入解锁序列（RM 3.5.3）。
///
/// 每次改写 SYSCTRL0/SYSCTRL1 前调用；一次写入后保护自动恢复。
pub(crate) fn unlock() {
    pac::SYSCTRL.sysctrl2().write(|w| w.set_sysctrl2(0x5A5A));
    pac::SYSCTRL.sysctrl2().write(|w| w.set_sysctrl2(0xA5A5));
}

/// 解锁后读-改-写 SYSCTRL0（写完保护自动恢复，RM 3.5.3）。
#[allow(dead_code)]
pub(crate) fn write_sysctrl0(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::Sysctrl0)) {
    unlock();
    pac::SYSCTRL.sysctrl0().modify(f);
}

/// 读取 SYSCTRL0（时钟源选择、分频）。
#[allow(dead_code)] // 可配置时钟树接入后使用（L110 侧同名列已按此标记）
pub(crate) fn read_sysctrl0() -> pac::sysctrl_ddl_v3_1::regs::Sysctrl0 {
    pac::SYSCTRL.sysctrl0().read()
}

/// 读-改-写 RCH 控制寄存器（RCH_CR 不在解锁范围，RM 3.5.4）。
///
/// 复位值 0x126 的 TRIM 段是晶圆默认值；厂商启动代码 SystemInit 会
/// 重写为出厂校准值（flash 0x00100C08，4 MHz 档）。RM 3.2.2/SDK
/// `Sysctrl_SetRCHTrim` 在 RCH 驱动系统时钟时按逐级序列写 TRIM；直接
/// 写仅在非当前源时安全。
pub(crate) fn modify_rch_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::RchCr)) {
    pac::SYSCTRL.rch_cr().modify(f);
}

/// 读取 RCH 控制寄存器（TRIM、STABLE）。
#[allow(dead_code)] // 可配置时钟树接入后使用
pub(crate) fn read_rch_cr() -> pac::sysctrl_ddl_v3_1::regs::RchCr {
    pac::SYSCTRL.rch_cr().read()
}

/// 读-改-写 RCL 控制寄存器（RCL_CR 不在解锁范围，RM 3.5.6）。
pub(crate) fn modify_rcl_cr(f: impl FnOnce(&mut pac::sysctrl_ddl_v3_1::regs::RclCr)) {
    pac::SYSCTRL.rcl_cr().modify(f);
}

/// 读-改-写 PERI_CLKEN（不在解锁范围，RM 3.5.9）。
///
/// 复位值 0x8080_0000：FLASH 与 HDIV 使能，其余门控（含位 28 GPIO）
/// 复位为关；门控关闭时对 GPIO 寄存器的写入不生效。
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
