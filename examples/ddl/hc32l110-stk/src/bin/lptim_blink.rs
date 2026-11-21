//! LPTIM 周期定时翻转 LED：验证低功耗定时器模式 2 与 WT_FLAG 写同步
//! 首验（差异台账待验证项：复位后 WT_FLAG 是否自行置 1）。
//!
//! 例程所需接线（HC32L110C6PA）：LED 接 P23，低电平点亮。
//!
//! LPTIM 无预分频，PCLK 4 MHz 时基下单次溢出最长约 16.4 ms（16 位计数、
//! ARR < 0xFFFC 合法区），无法直接产生 500 ms 周期。本例程取 62 Hz 溢出
//! （约 16.13 ms），软件累计 31 次溢出翻转一次 LED，得到约 1 Hz 闪烁。
//!
//! 覆盖路径：`LpTimer::new_periodic`（`period_to_reload`：ticks = 64516
//! → ARR = 1020，同时验证 ARR ≥ 0xFFFC 中断丢失区被拒绝，RM 10.2）、
//! `set_reload` 等待 WT_FLAG = 1 再写 ARR（RM 10.4.2，构造时执行）、
//! `start` 后 CNT 由 ARR 自动装载（RM 10.2）、轮询 TF、`clear_interrupt`
//! （RM 10.4.5）。
//!
//! 预期现象：
//! - LED 以约 1 Hz 翻转（500 ms 亮、500 ms 灭）；
//! - 上电后 LED 立即开始闪烁：说明复位后 WT_FLAG 会自行置 1，`set_reload`
//!   首次等待即通过（差异台账 HC32L110 LPTIM 条目首验项）；
//! - 若 LED 始终不亮且程序无输出：`set_reload` 超时 panic 停机，说明
//!   WT_FLAG 复位后不自行置 1，需改驱动方案并回填差异台账。

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use hc32_ddl_hal::gpio::{Drive, Level, Output};
use hc32_ddl_hal::lptim::{ClockSource, LpTimer};
use hc32_ddl_hal::{Config, Hertz};

/// LPTIM 溢出频率 62 Hz：PCLK 4 MHz / 62 → 64516 节拍，ARR = 1020。
const OVERFLOW_HZ: Hertz = Hertz(62);

/// 翻转一次 LED 所需的溢出次数：31 × 64516 / 4 MHz ≈ 500 ms。
const TOGGLE_EVERY: u32 = 31;

#[entry]
fn main() -> ! {
    // 保持复位默认时钟：RCH 4 MHz，HCLK = PCLK = 4 MHz。
    let p = hc32_ddl_hal::init(Config::default());

    let mut led = Output::new(p.P23, Level::High, Drive::High);
    // 构造内部执行首次 set_reload：等待 WT_FLAG = 1（RM 10.4.2 写同步），
    // 本调用即 WT_FLAG 复位初值的首验点。
    let mut timer = LpTimer::new_periodic(p.LPTIM, OVERFLOW_HZ, ClockSource::Pclk);
    timer.start();

    let mut overflows: u32 = 0;
    loop {
        while !timer.is_interrupt_pending() {
            cortex_m::asm::nop();
        }
        timer.clear_interrupt();
        overflows = overflows.wrapping_add(1);
        if overflows % TOGGLE_EVERY == 0 {
            led.toggle();
        }
    }
}
