//! LPUART 普通模式（LPMODE=0）阻塞回环：串口调试器发什么，终端原样
//! 收回什么。
//!
//! 例程所需接线（HC32L110C6PA）：
//! - P26 = LPUART_TXD（复用功能 5），接 USB 转串口适配器的 RXD；
//! - P25 = LPUART_RXD（复用功能 5），接适配器的 TXD；
//! - 开发板与适配器共地。
//!
//! 时钟：RCH 校准档 22.12 MHz，SCLK = PCLK（SCON.SCLKSEL=00，复位
//! 默认）。Mode1 波特率由 TIM2 的翻转输出供给（RM 16.1）：驱动构造会
//! 占用 TIM2 并置 BT2.CR.TOG_EN=1（RM 16.4.3 注：LPMODE=0 必须使能
//! TIM2 的 TOG 输出，属正常行为），16 位自动重载装载值 TM = 65530
//!（= 65536 - 6），实际 115208.33（+0.01%，RM 15.4.2.1 表）。
//!
//! 预期现象：串口调试器设置 115200-8-N-1，发送任意字符或字符串，
//! 终端逐字节原样回显；不发送时程序阻塞在接收等待，无输出。

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use hc32_ddl_hal::clock::RchFreq;
use hc32_ddl_hal::uart::{Config as UartConfig, LpPrescaler, LpUart, LpUartConfig, Mode, Sclk};
use hc32_ddl_hal::{Config, Hertz, clock};

/// 本例程的 PCLK = SCLK：RCH 22.12 MHz，不分频。
const PCLK: Hertz = Hertz::hz(22_120_000);

#[entry]
fn main() -> ! {
    // RCH 22.12 MHz 校准档：RM 15.4.2.1 表中 115200 误差最小的档位。
    let p = hc32_ddl_hal::init(Config {
        clock: Some(clock::Config {
            rch: Some(RchFreq::Mhz22_12),
            ..Default::default()
        }),
    });
    debug_assert_eq!(clock::clocks().pclk, PCLK);

    let mut lp = LpUart::new(
        p.LPUART,
        p.P26,
        p.P25,
        LpUartConfig {
            sclk: Sclk::Pclk,
            low_power: false,
            prescaler: LpPrescaler::Div1,
            base: UartConfig {
                baudrate: 115_200,
                mode: Mode::Mode1,
                ..UartConfig::default()
            },
        },
    )
    .expect("lpuart_echo: LPUART 配置失败");

    loop {
        // 阻塞回环：忙等 RI → 读 SBUF → 清 RI，再清 TI → 写 SBUF → 忙等 TI。
        if let Ok(byte) = lp.blocking_read() {
            // 发送侧无错误来源（blocking_write 恒 Ok），忽略返回值。
            let _ = lp.blocking_write(byte);
        }
        // Frame：驱动已清 FE 并丢弃错误帧（RM 16.5），继续接收下一帧。
    }
}
