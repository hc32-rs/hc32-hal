//! SYSCTRL 寄存器后端：DDL v4 布局（HC32F420 的 `SYSCTRL` 外设）。
//!
//! 基址 0x4000_B800，包含 CR0~CR3、RCH/RCL/XTH/PLL 振荡控制、RCHTRIM/
//! RCLTRIM 校准值、STATE 状态与 APB0/APB1/AHB0/AHB1 门控寄存器
//! （RM 3.5.1~3.5.16）。本后端只实现默认启动路径需要的部分：装载
//! RCH/RCL 出厂校准 TRIM、按端口开关 GPIO 的 AHB1 门控（AGENTS.md
//! 「仅接入默认时钟的型号只开放已实现的时钟能力」）；可配置时钟树
//! （RM 3.2 切换流程、PLL）待后续接入时再扩展。
//!
//! CR0/CR1/CR2 受 SYSCTRL_CR3 序列保护（先写 0x5A5A 再写 0xA5A5，一次
//! 写入后保护自动恢复，RM 3.5.5），默认路径不触碰这三个寄存器，暂不
//! 提供解锁入口。RCHTRIM/RCLTRIM 与各门控寄存器不在保护范围内。
//!
//! 同基址的 GPIOAUX 块（RM 3.5.18~3.5.20 的 IOAUX0~2：PCLK0/PCLK1 输出
//! 分频、MCO 选择、定时器 ETR/CAP 复用路由）只被波形输出与定时器复用
//! 使用，普通数字 GPIO 不依赖，本驱动不触碰。

use crate::pac;

/// 默认初始化：对齐厂商启动代码 SystemInit 的默认时钟路径
/// （mcu/common/system_hc32f420.c：RCLTRIM → RCHTRIM 顺序装载）。
///
/// Rust 启动路径（cortex-m-rt `Reset`）只搬运数据段并调用 `main`，不会
/// 执行任何厂商启动代码。本函数补齐 SystemInit 对应用可见的动作：
///
/// 1. 装载 RCL 出厂校准 TRIM（flash 0x0100_11E8，32.768 kHz 档；
///    RM 3.5.11）。RCL 复位为关（CR2 复位值 0x1 仅使能 RCH），装载的
///    TRIM 供后续使能 RCL 时直接得到校准频率，不开源、不等待稳定。
/// 2. 装载 RCH 出厂校准 TRIM（flash 0x0100_11E4，24 MHz 档；RM 3.5.10）。
///    RM 说明上电/掉电与 NRESET 复位时硬件会重新装载同值，SystemInit
///    仍显式重写，本驱动沿用。
///
/// SDK SystemInit 按 `*((uint32_t*)地址)` 整字读出并整字写入校准寄存器，
/// 会连同 b31~b9（RCHTRIM）/b31~b10（RCLTRIM）的保留位一起写；RM 要求
/// 保留位保持复位值，本驱动按 RM 文档的 2 字节校准值读取并只写 TRIM
/// 字段（差异见 docs/sdk-doc-discrepancies/hc32f420.md）。
///
/// `SystemCoreClockUpdate` 配置 SysTick 属于 CMSIS 运行时，不搬运。
/// 系统时钟保持复位默认：RCH 24 MHz 经 RCHCR.DIV = 2 分频为 RCHCLKD
/// 4 MHz（RM 3.1.2/3.5.6 复位值），CR0 复位值 0x0000_0000 选择
/// RCHCLKD 为系统时钟且 HCLK/PCLK0/PCLK1 均不分频（RM 3.5.2），即
/// HCLK = PCLK0 = PCLK1 = 4 MHz，与 [`super::Clocks`] 默认快照一致。
pub(crate) fn init_default() {
    // safety: 芯片固定的已映射只读校准区，厂商保证有数据（RM 3.5.10/3.5.11）。
    let rcl_trim = unsafe { (0x0100_11E8 as *const u16).read_volatile() };
    let rch_trim = unsafe { (0x0100_11E4 as *const u16).read_volatile() };
    pac::SYSCTRL.rcltrim().modify(|w| w.set_trim(rcl_trim));
    pac::SYSCTRL.rchtrim().modify(|w| w.set_trim(rch_trim));
}

/// 打开 `port` 端口的 GPIO 时钟门控，幂等。
///
/// AHB1EN 位 0~3 依次对应 GPIOA~GPIOD（RM 3.5.16），复位值
/// 0x0000_0000：门控关闭时对该端口寄存器的访问不生效。SDK
/// `SYSCTRL_PeriphClkEnable` 对 GPIOA~GPIOD 的编码（128~131）清除
/// REG3 掩码后落位 0~3，同源。
pub(crate) fn enable_gpio(port: u8) {
    pac::SYSCTRL.ahb1en().modify(|w| match port {
        0 => w.set_gpioa(true),
        1 => w.set_gpiob(true),
        2 => w.set_gpioc(true),
        3 => w.set_gpiod(true),
        _ => unreachable!("SYSCTRL v4 后端无此 GPIO 端口"),
    });
}

/// 复位默认系统时钟频率：RCHCLKD 4 MHz。
///
/// 区分 RCH 与 RCHCLKD：RCH 本振 22~24 MHz（复位装载 24 MHz 出厂
/// TRIM），经 SYSCTRL_RCHCR.DIV 分频产生 RCHCLKD（RM 3.1.2）；复位
/// DIV = 2 对应 24 MHz/6 = 4 MHz（RM 3.5.6 编码表）。系统时钟、HCLK、
/// PCLK0 与 PCLK1 复位默认均为 4 MHz。
pub(crate) const DEFAULT_SYSCLK_HZ: u32 = 4_000_000;
