//! PLL 输出 48 MHz：RCH 4 MHz ×12 倍频，从 PA01/PA02 观测（无需外部晶振）。
//!
//! 例程所需接线（HC32L136K8TA，HC32L13X_STK）：
//! - PA01：HCLK 波形输出（复用 6，GPIO CTRL1.HCLK_EN，RM Rev2.50 表 6-2、
//!   6.5.5.1）；
//! - PA02：PCLK 波形输出（复用 6，GPIO CTRL1.PCLK_EN）；
//! - 示波器/逻辑分析仪分别接 PA01、PA02 与 GND。
//!
//! 预期现象：PA01/PA02 均输出 48 MHz 方波（PLL 48 MHz，HCLK/PCLK 不分频）；
//! HCLK 大于 24 MHz，驱动已按 RM 3.2/11.5 自动插入 1 个 FLASH 等待周期；
//! LED（PD05）以约 1 Hz 闪烁表示程序在运行。对照 SDK 例程
//! `example/sysctrl/sysctrl_pll_output`。

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use cortex_m::{Peripherals, delay::Delay};

use hc32_ddl_hal::clock::{
    self, Config as ClkConfig, HclkPrescaler, OutputDiv, PclkPrescaler, PllConfig, PllSource,
    RchFreq, Sysclk,
};
use hc32_ddl_hal::gpio::{Drive, Level, Output};
use hc32_ddl_hal::{Config, Hertz};

#[entry]
fn main() -> ! {
    // RCH 4 MHz × 12 = 48 MHz：PLL 参考源取 RCH（RM 3.2.7），无需外部晶振。
    let p = hc32_ddl_hal::init(Config {
        clock: Some(ClkConfig {
            rch: Some(RchFreq::Mhz4),
            pll: Some(PllConfig::new(PllSource::Rch, 12)),
            sys: Sysclk::Pll,
            hclk_pre: HclkPrescaler::Div1,
            pclk_pre: PclkPrescaler::Div1,
            ..Default::default()
        }),
    });
    let clocks = clock::clocks();
    assert_eq!(clocks.sys, Hertz::mhz(48));
    assert_eq!(clocks.hclk, Hertz::mhz(48));
    assert_eq!(clocks.pclk, Hertz::mhz(48));

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
