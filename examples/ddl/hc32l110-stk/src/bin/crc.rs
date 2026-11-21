//! CRC 自检：按 RM 与定稿驱动验证 CRC 编码/校验模式、写入位宽与 FLAG
//! 行为（板测矩阵 T1~T4，见 hc32-hal/docs/sdk-doc-discrepancies.md 的
//! HC32L110 CRC 条目）。
//!
//! 例程所需接线（HC32L110C6PA）：LED 接 P23，低电平点亮（输出 0 亮、
//! 输出 1 灭）。无需其他接线。
//!
//! 预期现象：
//! - 全部通过：LED 常亮；
//! - 任一失败：LED 按"短闪 N 次（120 ms）+ 熄灭 900 ms"循环指示失败项：
//!   N=1 T1 位序（硬件编码结果与软件参考不符）；
//!   N=2 T2 正确值校验 FLAG 不为 1；
//!   N=3 T2 篡改数据后 FLAG 不为 0；
//!   N=4 T3 三种写入位宽结果不一致；
//!   N=5 T4 空序列 init 后 RESULT 不为 0xFFFF。
//!
//! 软件参考实现按 RM 20.3.2 多项式 0x1021 与 RM 20.4.1 第一步初值 0xFFFF
//! 逐位计算，每字节 MSB 在前（驱动模块文档的"不反转"解读）；对
//! `"123456789"` 应得 0x29B1。该解读是否与硬件位序一致正是 T1 要首验的
//! 内容（RM/DS 未说明输入输出是否反转）。
//!
//! T2 每次校验后读回的 RESULT（即追加校验值之后的残差）经 `black_box`
//! 保留，可用调试器观察局部变量：残差为 0x0000 提示"数据+校验值"恰好
//! 把引擎推回零残差（零残差判定下 FLAG 应为 1）；残差非 0 且 FLAG=1 提示
//! FLAG 按"与末次写入值比较"判定。判读依据见驱动模块文档 FLAG 一节。

#![no_std]
#![no_main]

use core::hint::black_box;

use cortex_m::{Peripherals, delay::Delay};
use cortex_m_rt::entry;
use panic_halt as _;

use hc32_ddl_hal::gpio::{Drive, Level, Output};
use hc32_ddl_hal::{Config, clock, crc::Crc};

/// T1/T2 用报文：软件参考实现下其 CRC-16 为 0x29B1（CCITT-FALSE 检验值）。
const MESSAGE: &[u8] = b"123456789";

/// T2 篡改报文：末字节 0x39（'9'）改为 0x30（'0'）。
const TAMPERED: &[u8] = b"123456780";

/// T3 用 RM 20.3.3 示例序列的三种位宽喂法（按小端序列化对应同一字节序列）。
const T3_BYTES: &[u8] = &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
const T3_HALFWORDS: &[u16] = &[0x1100, 0x3322, 0x5544, 0x7766];
const T3_WORDS: &[u32] = &[0x3322_1100, 0x7766_5544];

/// 软件参考 CRC：RM 20.3.2 多项式 0x1021，RM 20.4.1 初值 0xFFFF，每字节
/// MSB 在前（"不反转"解读）。
fn soft_crc(data: &[u8]) -> u16 {
    let mut reg: u16 = 0xFFFF;
    for &byte in data {
        reg ^= u16::from(byte) << 8;
        for _ in 0..8 {
            reg = if reg & 0x8000 != 0 {
                (reg << 1) ^ 0x1021
            } else {
                reg << 1
            };
        }
    }
    reg
}

/// 失败指示：短闪 `n` 次后熄灭 900 ms，循环（LED 低电平点亮）。
fn fail(led: &mut Output, delay: &mut Delay, n: u32) -> ! {
    loop {
        for _ in 0..n {
            led.set_low();
            delay.delay_ms(120);
            led.set_high();
            delay.delay_ms(120);
        }
        delay.delay_ms(900);
    }
}

#[entry]
fn main() -> ! {
    // 保持复位默认时钟：RCH 4 MHz，HCLK = PCLK = 4 MHz。
    let p = hc32_ddl_hal::init(Config::default());

    let mut delay = Delay::new(Peripherals::take().unwrap().SYST, clock::clocks().hclk.0);
    let mut led = Output::new(p.P23, Level::High, Drive::High);
    // 单例外设；本例程是全程序唯一使用者。
    let mut crc = unsafe { Crc::steal() };

    // T1 位序：硬件编码结果与软件参考比对。
    let message_crc = crc.compute_bytes(MESSAGE);
    if message_crc != soft_crc(MESSAGE) {
        fail(&mut led, &mut delay, 1);
    }

    // T2 FLAG 机制：正确值期待 FLAG=1；篡改数据期待 FLAG=0。
    let ok = crc.check_bytes(MESSAGE, message_crc);
    black_box(crc.result());
    if !ok {
        fail(&mut led, &mut delay, 2);
    }

    let tampered_flag = crc.check_bytes(TAMPERED, message_crc);
    black_box(crc.result());
    if tampered_flag {
        fail(&mut led, &mut delay, 3);
    }

    // T3 位宽一致（RM 20.3.3 示例序列）：编码与校验两种模式、三种位宽。
    let reference = soft_crc(T3_BYTES);
    let encode_ok = crc.compute_bytes(T3_BYTES) == reference
        && crc.compute_u16(T3_HALFWORDS) == reference
        && crc.compute_u32(T3_WORDS) == reference;
    let check_ok = crc.check_bytes(T3_BYTES, reference)
        && crc.check_u16(T3_HALFWORDS, reference)
        && crc.check_u32(T3_WORDS, reference);
    if !(encode_ok && check_ok) {
        fail(&mut led, &mut delay, 4);
    }

    // T4 空序列：init 后 RESULT 应回初值 0xFFFF。
    crc.init();
    if crc.result() != 0xFFFF {
        fail(&mut led, &mut delay, 5);
    }

    // 全部通过：常亮（低电平点亮）。
    led.set_low();
    loop {
        delay.delay_ms(1000);
    }
}
