//! 时钟分频测量：P24/P31 输出 HCLK/PCLK 波形，用示波器或逻辑分析仪
//! 测量周期，验证时钟驱动的分频配置。
//!
//! 接线（HC32L110STK，HC32L110C6PA）：
//! - P24 → HCLK_OUT（复用功能 3）；
//! - P31 → PCLK_OUT（复用功能 3）。P31 默认为 SWCLK：例程置
//!   SYSCTRL1.SWD_USE_IO 释放为 GPIO；烧录后如需再次连接调试器，
//!   可使用复位下连接。
//!
//! 预期现象：
//! - 上电 5 秒内：HCLK = RCH 4 MHz / 2 = 2 MHz 方波（P24）、
//!   PCLK = 2 MHz / 4 = 500 kHz 方波（P31）；
//! - 5 秒后（clock::init 可重复调用）：HCLK/4 = 1 MHz、
//!   PCLK/1 = 1 MHz，波形持续输出。

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use cortex_m::{Peripherals, delay::Delay};

use hc32_ddl_hal::clock::{self, HclkPrescaler, PclkPrescaler};
use hc32_ddl_hal::{Config, Hertz};

/// 阶段 1 的 HCLK 频率（RCH 4 MHz / 2）。
const HCLK_PHASE1: Hertz = Hertz::mhz(2);

#[entry]
fn main() -> ! {
    // 阶段 1：RCH 4 MHz，HCLK/2 = 2 MHz，PCLK 再 /4 = 500 kHz。
    hc32_ddl_hal::init(Config {
        clock: Some(clock::Config {
            hclk_pre: HclkPrescaler::Div2,
            pclk_pre: PclkPrescaler::Div4,
            ..Default::default()
        }),
    });
    debug_assert_eq!(clock::clocks().hclk, HCLK_PHASE1);

    // P24 输出 HCLK/1 = 2 MHz，P31 输出 PCLK/1 = 500 kHz。
    // P31 配置会清 HCLK_EN，最后再打开 P24。
    clock::enable_pclk_output_p31(clock::OutputDiv::Div1);
    clock::enable_hclk_output_p24(clock::OutputDiv::Div1);

    let mut delay = Delay::new(Peripherals::take().unwrap().SYST, clock::clocks().hclk.0);
    delay.delay_ms(5_000);

    // 阶段 2：运行中重配分频 → HCLK/4 = 1 MHz、PCLK = 1 MHz。
    clock::init(&clock::Config {
        hclk_pre: HclkPrescaler::Div4,
        pclk_pre: PclkPrescaler::Div1,
        ..Default::default()
    });

    loop {
        // 波形持续输出，main 不返回。
        cortex_m::asm::nop();
    }
}
