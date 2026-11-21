//! ADT0（TIM4）锯齿波比较输出 PWM：验证高级定时器 PWM 时基与运行中
//! 缓存改占空比路径。
//!
//! 例程所需接线（HC32L110C6PA）：LED 接 P23（复用功能 3 = TIM4_CHA），
//! 低电平点亮。引脚复用由例程直接写 P23_SEL = 3（RM 表 6-2：P23
//! Px_sel=3 → TIM4_CHA）。
//!
//! 覆盖路径：`Adt::new_pwm`（`period_to_perar` 自动选档：1 kHz → Div1、
//! PERAR = 3999；`set_period` 直写 PERAR + PERBR/BENP 缓存，RM 12.3.2/
//! 12.3.3/12.3.9）、`enable_output`（直写 GCMAR + GCMCR/BENA 缓存，
//! CMPCA = SetHigh、PERCA = KeepLow，RM 12.3.8）、`start`（GCONR.START）、
//! `set_compare`（运行中写缓存 GCMCR，硬件在溢出点装载到 GCMAR，无毛刺，
//! RM 12.2.7.3）。
//!
//! PWM 关系（锯齿波递加，RM 12.2.1.1/12.2.1.2）：计数值达到比较值时
//! 端口置高、溢出点回低。LED 低电平点亮，亮的比例 = GCMA / (PERAR + 1)
//! = 比较值 / 4000。
//!
//! 预期现象：LED 呼吸灯——亮的比例在 10%（比较值 400）至 90%（3600）
//! 间往返渐变，单程约 220 ms（11 级 × 20 ms）。

#![no_std]
#![no_main]

use cortex_m::{Peripherals, delay::Delay};
use cortex_m_rt::entry;
use panic_halt as _;

use hc32_ddl_hal::adt::{Adt, Ch, PortAction};
use hc32_ddl_hal::pac;
use hc32_ddl_hal::{Config, Hertz, clock};

/// PWM 周期 1 ms（PCLK 4 MHz，PERAR = 3999）。
const PWM_HZ: Hertz = Hertz(1_000);

/// 呼吸渐变步进周期。
const STEP_MS: u32 = 20;

/// 比较值往返区间：LED 亮 10% ~ 90%。
const CMP_MIN: u16 = 400;
const CMP_MAX: u16 = 3600;
const CMP_STEP: u16 = 300;

#[entry]
fn main() -> ! {
    // 保持复位默认时钟：RCH 4 MHz，HCLK = PCLK = 4 MHz。
    let p = hc32_ddl_hal::init(Config::default());

    // P23 切到 TIM4_CHA 复用输出：P23_SEL = 3（RM 表 6-2），方向输出。
    // 数字复用路径：ADS 清零 → SEL 写 3 → DIR 输出（RM 6.3.4/6.4.1）。
    pac::GPIO.p2ads().modify(|w| w.set_p23(false));
    pac::GPIO.p2dr().modify(|w| w.set_p23(false));
    pac::GPIO.p2od().modify(|w| w.set_p23(false));
    pac::GPIO.p2pu().modify(|w| w.set_p23(false));
    pac::GPIO.p2pd().modify(|w| w.set_p23(false));
    pac::GPIO.p2dir().modify(|w| w.set_p23(false));
    pac::GPIO.p23_sel().modify(|w| w.set_sel(3));

    // 1 kHz PWM 时基；匹配输出高、周期点回低。
    let mut adt = Adt::new_pwm(p.ADT0, PWM_HZ);
    adt.enable_output(Ch::A, CMP_MIN, PortAction::SetHigh);
    adt.start();

    let mut delay = Delay::new(Peripherals::take().unwrap().SYST, clock::clocks().hclk.0);

    let mut cmp: u16 = CMP_MIN;
    let mut step: i32 = CMP_STEP as i32;
    loop {
        delay.delay_ms(STEP_MS);
        // 写缓存 GCMCR：硬件在溢出点装载到 GCMAR（RM 12.2.7.3）。
        adt.set_compare(Ch::A, cmp);

        let next = cmp as i32 + step;
        if next > CMP_MAX as i32 || next < CMP_MIN as i32 {
            step = -step;
        }
        cmp = (cmp as i32 + step).clamp(CMP_MIN as i32, CMP_MAX as i32) as u16;
    }
}
