//! WDT 复位循环：喂狗阶段 LED 按喂狗节拍闪烁，停止喂狗后 WDT 溢出复位
//! 芯片，重启后以 LED 常亮标记 `RESET_FLAG.WDT` 复位原因，如此循环。
//!
//! 例程所需接线（HC32L110C6PA）：LED 接 P23，低电平点亮（输出 0 亮、
//! 输出 1 灭），同 `blinky`。
//!
//! 预期现象（约 14 s 一轮，循环往复）：
//! 1. 启动标记 2 s：上次复位为 WDT 溢出 → LED 常亮；上电等其他复位 →
//!    LED 熄灭（读出标志后顺带写 0 清除，RM 4.2.1 位 3：需软件清除，
//!    上电状态不定）；
//! 2. 喂狗 10 s：LED 每 500 ms 翻转（1 Hz 闪烁），每次翻转前喂狗，
//!    间隔小于 1.64 s 溢出档的一半 820 ms（RM 14.2.2 注：建议在计数
//!    到达溢出值一半之前清零），芯片不复位；
//! 3. 停止喂狗：LED 常亮等待，约 1.64 s 后 WDT 溢出复位芯片（灯灭即
//!    复位时刻，可计时核对溢出档标称值）；
//! 4. 复位后回到 1：LED 常亮 2 s，证明 `RESET_FLAG.WDT` 被正确读出。
//!
//! 上电首轮通常显示“熄灭 2 s”；若上电时标志恰为 1（RM 4.2.1 标注上电
//! 状态不定），或重新烧录前已发生过 WDT 复位（烧录复位不清除该标志），
//! 首轮也会显示常亮标记。若停喂后 LED 常亮超过约 5 s 才恢复闪烁，说明
//! WDT 未按标称溢出复位，需核对溢出档位与驱动。

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use cortex_m::{Peripherals, delay::Delay};

use hc32_ddl_hal::gpio::{Drive, Level, Output};
use hc32_ddl_hal::wdt::{self, Period, Wdt};
use hc32_ddl_hal::{Config, clock};

/// WDT 溢出档位：1.64 s（RM 14.3.2 WOV=1010）。
const PERIOD: Period = Period::S1_64;
/// 喂狗与 LED 翻转间隔：500 ms，小于溢出标称值一半（820 ms，
/// RM 14.2.2 注），为 RC 振荡器低精度留余量。
const FEED_MS: u32 = 500;
/// 喂狗轮数：20 轮 × 500 ms = 10 s。
const FEED_ROUNDS: u32 = 20;
/// 启动时复位原因标记时长。
const MARK_MS: u32 = 2_000;
/// 停止喂狗后的静默时长：5 s，远超 1.64 s 标称溢出，确保复位发生。
const STARVE_MS: u32 = 5_000;

#[entry]
fn main() -> ! {
    // 保持复位默认时钟：RCH 4 MHz。
    let p = hc32_ddl_hal::init(Config::default());

    let mut delay = Delay::new(Peripherals::take().unwrap().SYST, clock::clocks().hclk.0);
    let mut led = Output::new(p.P23, Level::High, Drive::High);

    // 读出并清除 WDT 复位标志（RM 4.2.1 位 3）：WDT 溢出复位启动 →
    // LED 常亮；上电等其他复位启动 → LED 熄灭。
    if wdt::reset_caused_by_wdt(true) {
        led.set_low();
    } else {
        led.set_high();
    }
    delay.delay_ms(MARK_MS);

    // 复位模式启动 WDT（RM 14.2.2 Step1~3：配置 WOV、WINT_EN=0、写
    // 启动序列 0x1E、0xE1）。
    let mut wdt = Wdt::new(wdt::Config::reset(PERIOD));
    wdt.start();
    debug_assert!(Wdt::is_running());

    // 喂狗阶段：LED 闪烁证明程序存活，WDT 从不溢出。
    for _ in 0..FEED_ROUNDS {
        delay.delay_ms(FEED_MS);
        wdt.feed();
        led.toggle();
    }

    // 停止喂狗：LED 常亮等待，约 1.64 s 后 WDT 溢出复位芯片。
    led.set_low();
    delay.delay_ms(STARVE_MS);

    // 不应到达：若 WDT 意外未复位（RC 精度极端偏差），继续喂狗闪烁，
    // 保持现象可观察、可重复。
    loop {
        delay.delay_ms(FEED_MS);
        wdt.feed();
        led.toggle();
    }
}
