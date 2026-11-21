//! BT0 周期定时翻转 LED：验证基本定时器模式 2 的周期换算与启停/清标志。
//!
//! 例程所需接线（HC32L110C6PA）：LED 接 P23，低电平点亮。
//!
//! 覆盖路径：`Bt::new_periodic`（`period_to_reload` 自动选档：500 ms →
//! 预分频 /32、ARR = 3036）、`start`、轮询 IFR.TF、`clear_interrupt`
//! （ICLR 写 0 清除，RM 9.4.6）。
//!
//! 预期现象：LED 以约 1 Hz 翻转（500 ms 亮、500 ms 灭）。

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use hc32_ddl_hal::bt::Bt;
use hc32_ddl_hal::gpio::{Drive, Level, Output};
use hc32_ddl_hal::{Config, Hertz};

/// 溢出频率 2 Hz：每 500 ms 一次溢出。
const OVERFLOW_HZ: Hertz = Hertz(2);

#[entry]
fn main() -> ! {
    // 保持复位默认时钟：RCH 4 MHz，HCLK = PCLK = 4 MHz。
    let p = hc32_ddl_hal::init(Config::default());

    let mut led = Output::new(p.P23, Level::High, Drive::High);
    // PCLK 4 MHz、500 ms → 2_000_000 节拍 → Div32 档 62500 计数，
    // ARR = 65536 - 62500 = 3036。
    let mut timer = Bt::new_periodic(p.BT0, OVERFLOW_HZ);
    timer.start();

    loop {
        while !timer.is_interrupt_pending() {
            cortex_m::asm::nop();
        }
        timer.clear_interrupt();
        led.toggle();
    }
}
