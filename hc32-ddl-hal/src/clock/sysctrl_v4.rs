//! SYSCTRL 寄存器后端：DDL v4 布局（HC32F420 的 `SYSCTRL` 外设）。
//!
//! 基址 0x4000_B800。CR0~CR2 受 CR3 的 0x5A5A/0xA5A5 序列保护，每次
//! 写后自动恢复保护（RM 3.5.2~3.5.5）；振荡参数、TRIM 与门控寄存器可直接
//! 读写。FLASH_WAIT 使用独立的 FLASH_BYPASS 解锁序列（RM 10.6.9）。

use crate::pac;

fn unlock() {
    pac::SYSCTRL.cr3().write(|w| w.set_cr3(0x5A5A));
    pac::SYSCTRL.cr3().write(|w| w.set_cr3(0xA5A5));
}

pub(crate) fn modify_cr0(f: impl FnOnce(&mut pac::sysctrl_ddl_v4::regs::Cr0)) {
    cortex_m::interrupt::free(|_| {
        unlock();
        pac::SYSCTRL.cr0().modify(f);
    });
}

pub(crate) fn modify_cr2(f: impl FnOnce(&mut pac::sysctrl_ddl_v4::regs::Cr2)) {
    cortex_m::interrupt::free(|_| {
        unlock();
        pac::SYSCTRL.cr2().modify(f);
    });
}

pub(crate) fn read_cr0() -> pac::sysctrl_ddl_v4::regs::Cr0 {
    pac::SYSCTRL.cr0().read()
}

pub(crate) fn read_cr2() -> pac::sysctrl_ddl_v4::regs::Cr2 {
    pac::SYSCTRL.cr2().read()
}

pub(crate) fn read_state() -> pac::sysctrl_ddl_v4::regs::State {
    pac::SYSCTRL.state().read()
}

pub(crate) fn modify_rch(f: impl FnOnce(&mut pac::sysctrl_ddl_v4::regs::Rch)) {
    pac::SYSCTRL.rch().modify(f);
}

pub(crate) fn modify_rcl(f: impl FnOnce(&mut pac::sysctrl_ddl_v4::regs::Rcl)) {
    pac::SYSCTRL.rcl().modify(f);
}

pub(crate) fn modify_xth(f: impl FnOnce(&mut pac::sysctrl_ddl_v4::regs::Xth)) {
    pac::SYSCTRL.xth().modify(f);
}

pub(crate) fn modify_pll(f: impl FnOnce(&mut pac::sysctrl_ddl_v4::regs::Pll)) {
    pac::SYSCTRL.pll().modify(f);
}

/// SDK 的 PLL 例程在启动 PLL 前开启 ADC0 门控和 BGR，并等待 20 µs；
/// BGR 在 PLL 使用期间保持开启，延时由上层按当前 HCLK 完成。
pub(crate) fn enable_bgr() {
    pac::SYSCTRL.apb1en().modify(|w| w.set_adc0(true));
    pac::ADC0.bgr().modify(|w| w.set_bgren(true));
}

/// 设置 FLASH_WAIT.WAIT，并回读校验（RM 3.2、10.6.3/10.6.9）。
pub(crate) fn set_flash_wait(wait: u8) {
    debug_assert!(wait <= 3);
    if pac::FLASH.wait().read().wait() == wait {
        return;
    }
    cortex_m::interrupt::free(|_| {
        for _ in 0..4 {
            pac::FLASH.bypass().write(|w| w.set_byseq(0x5A5A));
            pac::FLASH.bypass().write(|w| w.set_byseq(0xA5A5));
            pac::FLASH.wait().modify(|w| w.set_wait(wait));
            if pac::FLASH.wait().read().wait() == wait {
                return;
            }
        }
        panic!("clock: FLASH_WAIT 写入校验失败");
    });
}

/// 默认初始化：对齐厂商 `SystemInit`，装载 RCL、RCH 出厂校准值。
///
/// SDK 整字读写会改保留位；这里按 RM 3.5.10/3.5.11 读取 16 位校准值并只
/// 修改 TRIM 字段。默认 RCH 24 MHz 经 DIV=6 得到 4 MHz 系统时钟。
pub(crate) fn init_default() {
    // safety: 芯片固定的已映射只读校准区，厂商保证有数据（RM 3.5.10/3.5.11）。
    let rcl_trim = unsafe { (0x0100_11E8 as *const u16).read_volatile() };
    let rch_trim = unsafe { (0x0100_11E4 as *const u16).read_volatile() };
    pac::SYSCTRL.rcltrim().modify(|w| w.set_trim(rcl_trim));
    pac::SYSCTRL.rchtrim().modify(|w| w.set_trim(rch_trim));
}

/// 打开 `port` 端口的 GPIO 时钟门控，幂等（AHB1EN 位 0~3，RM 3.5.16）。
pub(crate) fn enable_gpio(port: u8) {
    pac::SYSCTRL.ahb1en().modify(|w| match port {
        0 => w.set_gpioa(true),
        1 => w.set_gpiob(true),
        2 => w.set_gpioc(true),
        3 => w.set_gpiod(true),
        _ => unreachable!("SYSCTRL v4 后端无此 GPIO 端口"),
    });
}

pub(crate) const DEFAULT_SYSCLK_HZ: u32 = 4_000_000;
