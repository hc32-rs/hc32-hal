//! PCA 通道 0 输出 8 位 PWM：验证 PCA 时基与 PWM 模式（含 duty 反向语义
//! 与运行中改占空比的缓存装载）。
//!
//! 例程所需接线（HC32L110C6PA）：LED 接 P23（复用功能 4 = PCA_CH0），
//! 低电平点亮。引脚复用由例程直接写 P23_SEL = 4（GPIO 驱动暂未提供复用
//! 编号 API，RM 表 6-2：P23 Px_sel=4 → PCA_CH0）。
//!
//! 覆盖路径：`Pca::new`（时基 PCLK/4 = 1 MHz，`enable_peripheral` 门控）、
//! `enable_pwm`（先 CCAPxL 后 CCAPxH 写入，触发 ECOM 自动副作用，RM
//! 11.4.6/11.4.7）、`start`（CCON.CR）、`set_duty`（写 CCAPxH，硬件在
//! CNTL 溢出时装载到 CCAPxL，无毛刺，RM 11.2.3.4）。
//!
//! PWM 频率 = 时基 / 256 = 1 MHz / 256 ≈ 3.9 kHz（RM 11.2.3.4）。注意
//! `duty` 数值与高电平占比**反向**（RM 11.4.7）：duty = 0 输出恒高、
//! duty = 255 时高电平仅 1/256。本例程 LED 低电平点亮，故 LED 亮的占比
//! = duty / 256。
//!
//! 预期现象：LED 呼吸灯——亮的占比在 12.5%（duty = 32）至 87.5%
//! （duty = 224）间往返渐变，单程约 480 ms（24 级 × 20 ms）。

#![no_std]
#![no_main]

use cortex_m::{Peripherals, delay::Delay};
use cortex_m_rt::entry;
use panic_halt as _;

use hc32_ddl_hal::pac;
use hc32_ddl_hal::pca::{Channel, Pca, TimebaseClock};
use hc32_ddl_hal::{Config, clock};

/// 呼吸渐变步进周期。
const STEP_MS: u32 = 20;

/// duty 往返区间：LED 亮 12.5% ~ 87.5%。
const DUTY_MIN: u8 = 32;
const DUTY_MAX: u8 = 224;
const DUTY_STEP: u8 = 8;

#[entry]
fn main() -> ! {
    // 保持复位默认时钟：RCH 4 MHz，HCLK = PCLK = 4 MHz。
    let p = hc32_ddl_hal::init(Config::default());

    // P23 切到 PCA_CH0 复用输出：P23_SEL = 4（RM 表 6-2），方向输出。
    // 数字复用路径：ADS 清零 → SEL 写 4 → DIR 输出（RM 6.3.4/6.4.1）。
    pac::GPIO.p2ads().modify(|w| w.set_p23(false));
    pac::GPIO.p2dr().modify(|w| w.set_p23(false));
    pac::GPIO.p2od().modify(|w| w.set_p23(false));
    pac::GPIO.p2pu().modify(|w| w.set_p23(false));
    pac::GPIO.p2pd().modify(|w| w.set_p23(false));
    pac::GPIO.p2dir().modify(|w| w.set_p23(false));
    pac::GPIO.p23_sel().modify(|w| w.set_sel(4));

    // 时基 PCLK/4 = 1 MHz，PWM 频率 = 1 MHz / 256 ≈ 3.9 kHz。
    let mut pca = Pca::new(p.PCA, TimebaseClock::PclkDiv4);
    // 通道配置借用与 start 的借用不相容：先配好通道再启动时基。
    {
        let mut ch0 = pca.channel(Channel::C0);
        ch0.enable_pwm(DUTY_MIN);
    }
    pca.start();

    let mut delay = Delay::new(Peripherals::take().unwrap().SYST, clock::clocks().hclk.0);

    let mut duty: u8 = DUTY_MIN;
    let mut step: i16 = DUTY_STEP as i16;
    loop {
        delay.delay_ms(STEP_MS);
        // 写 CCAPxH：硬件在 CNTL 溢出（周期边界）装载到 CCAPxL，无毛刺。
        pca.channel(Channel::C0).set_duty(duty);

        let next = duty as i16 + step;
        if next > DUTY_MAX as i16 || next < DUTY_MIN as i16 {
            step = -step;
        }
        duty = (duty as i16 + step).clamp(DUTY_MIN as i16, DUTY_MAX as i16) as u8;
    }
}
