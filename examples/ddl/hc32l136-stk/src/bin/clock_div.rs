//! 时钟分频与波形输出：RCH 4 MHz 经 HCLK/2、PCLK/4 分频，从 PA01/PA02 观测。
//!
//! 例程所需接线（HC32L136K8TA，HC32L13X_STK）：
//! - PA01：HCLK 波形输出（复用 6，GPIO CTRL1.HCLK_EN，RM Rev2.50 表 6-2、
//!   6.5.5.1）；
//! - PA02：PCLK 波形输出（复用 6，GPIO CTRL1.PCLK_EN）；
//! - 示波器/逻辑分析仪分别接 PA01、PA02 与 GND。
//!
//! 预期现象：PA01 输出 2 MHz 方波（RCH 4 MHz，HCLK = SYSCLK/2），PA02 输出
//! 500 kHz 方波（PCLK = HCLK/4）；LED（PD05）以约 1 Hz 闪烁表示程序在运行。
//! 对照 SDK 例程 `example/sysctrl/sysctrl_clk_div_output`。

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use cortex_m::{Peripherals, delay::Delay};

use hc32_ddl_hal::clock::{
    self, Config as ClkConfig, HclkPrescaler, OutputDiv, PclkPrescaler, RchFreq, Sysclk,
};
use hc32_ddl_hal::gpio::{Drive, Level, Output};
use hc32_ddl_hal::{Config, Hertz};

#[entry]
fn main() -> ! {
    // RCH 4 MHz → HCLK = SYSCLK/2 = 2 MHz → PCLK = HCLK/4 = 500 kHz。
    let p = hc32_ddl_hal::init(Config {
        clock: Some(ClkConfig {
            rch: Some(RchFreq::Mhz4),
            sys: Sysclk::Rch,
            hclk_pre: HclkPrescaler::Div2,
            pclk_pre: PclkPrescaler::Div4,
            ..Default::default()
        }),
    });
    let clocks = clock::clocks();
    assert_eq!(clocks.sys, Hertz::mhz(4));
    assert_eq!(clocks.hclk, Hertz::mhz(2));
    assert_eq!(clocks.pclk, Hertz(500_000));

    // PA01 输出 HCLK、PA02 输出 PCLK（CTRL1 输出分频均取 1，波形即总线频率）。
    clock::enable_hclk_output_pa01(OutputDiv::Div1);
    clock::enable_pclk_output_pa02(OutputDiv::Div1);

    let mut delay = Delay::new(Peripherals::take().unwrap().SYST, clocks.hclk.0);
    let mut led = Output::new(p.PD05, Level::Low, Drive::High);

    loop {
        delay.delay_ms(500);
        led.toggle();
    }
}
