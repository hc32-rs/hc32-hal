//! 轮询按键控制 LED：验证 GPIO 输入（内部上拉）与输出联动。
//!
//! 例程所需接线（HC32L136K8TA，HC32L13X_STK）：
//! - LED：PD05，高电平点亮（输出 1 亮、0 灭）；
//! - 按键：PD04，按下接地，松开经内部上拉为高。
//!
//! 预期现象：按下按键 LED 亮，松开 LED 灭（10 ms 两次采样消抖）。

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use cortex_m::{Peripherals, delay::Delay};

use hc32_ddl_hal::gpio::{Drive, Input, Level, Output, Pull};
use hc32_ddl_hal::{Config, clock};

#[entry]
fn main() -> ! {
    // 保持复位默认时钟：RCH 4 MHz。
    let p = hc32_ddl_hal::init(Config::default());

    let mut delay = Delay::new(Peripherals::take().unwrap().SYST, clock::clocks().hclk.0);
    let mut led = Output::new(p.PD05, Level::Low, Drive::High);
    let button = Input::new(p.PD04, Pull::Up);

    loop {
        // 按下为低电平：两次采样确认，期间等待释放。
        if button.is_low() {
            delay.delay_ms(10);
            if button.is_low() {
                led.set_high();
                while button.is_low() {
                    delay.delay_ms(10);
                }
            }
        }
        led.set_low();
    }
}
