//! LED 闪烁：验证 GPIO 推挽输出与电平翻转。
//!
//! 例程所需接线（HC32L136K8TA，HC32L13X_STK）：板载 LED 接 PD05，
//! 高电平点亮（输出 1 亮、输出 0 灭）。
//!
//! 预期现象：LED 以约 1 Hz（500 ms 亮、500 ms 灭）闪烁。

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use cortex_m::{Peripherals, delay::Delay};

use hc32_ddl_hal::gpio::{Drive, Level, Output};
use hc32_ddl_hal::{Config, Hertz, clock};

/// 当前 HCLK 频率：复位默认 RCH 4 MHz、不分频。
const HCLK: Hertz = Hertz::mhz(4);

#[entry]
fn main() -> ! {
    // HC32L13x 只接入默认时钟路径：RCH 4 MHz，HCLK = PCLK = 4 MHz。
    let p = hc32_ddl_hal::init(Config::default());
    debug_assert_eq!(clock::clocks().hclk, HCLK);

    let mut delay = Delay::new(Peripherals::take().unwrap().SYST, clock::clocks().hclk.0);
    let mut led = Output::new(p.PD05, Level::Low, Drive::High);

    loop {
        delay.delay_ms(500);
        led.toggle();
    }
}
