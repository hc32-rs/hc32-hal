//! HC32L110 时钟树切换流程（RM 3.2）。

use super::{
    Clocks, Config, HclkPrescaler, RchFreq, RclFreq, RclStartup, Sysclk, backend, store_clocks,
};
use crate::{Hertz, pac};

pub(super) const DEFAULT_RCH_HZ: u32 = 4_000_000;

/// 读当前系统时钟源。
fn current_source() -> Sysclk {
    Sysclk::decode(backend::read_sysctrl0().clk_sw4_sel()).unwrap_or(Sysclk::Rch)
}

/// 当前 HCLK 频率，用于软件延时估算。
///
/// XTH 为当前源时晶振频率未知，按 32 MHz 保守估计（延时只会长不会短）。
fn current_hclk_hz() -> u32 {
    let sys = match current_source() {
        Sysclk::Rch => {
            RchFreq::from_trim(backend::read_rch_cr().trim())
                .frequency()
                .0
        }
        Sysclk::Rcl => {
            let trim = backend::read_rcl_cr().trim();
            if trim & (1 << 9) != 0 {
                RclFreq::K38_4.frequency().0
            } else {
                RclFreq::K32_768.frequency().0
            }
        }
        Sysclk::Xth => 32_000_000,
        Sysclk::Xtl => 32_768,
    };
    let prs = backend::read_sysctrl0().hclk_prs();
    sys >> prs
}

/// 软件延时不少于 10 ms（RM 3.2.3 Step7：XTH 稳定后需延时 10ms 以上）。
///
/// 以当前 HCLK 估算迭代次数；M0+ 上该循环单次迭代约 3 周期起，按 3
/// 周期取下限保证时长不少于 10 ms。SDK 无此延时，差异见差异台账。
fn delay_10ms() {
    let n = (current_hclk_hz() / 300).max(1_000);
    let mut i = 0u32;
    while i < n {
        i = i.wrapping_add(1);
        core::hint::black_box(i);
    }
}

/// 临时开启 GPIO 门控执行晶振引脚配置，结束后恢复原门控值（SDK
/// `Clk_Enable` 同做法：保存 PERI_CLKEN → 置 GPIO 位 → 配引脚 → 恢复）。
fn with_gpio_gate(f: impl FnOnce()) {
    let saved = backend::read_peri_clken();
    if !saved.gpio() {
        super::enable_gpio();
        f();
        backend::write_peri_clken(saved);
    } else {
        f();
    }
}

/// 直接写 RCH 出厂校准值（RCH_CR 不在解锁范围，RM 3.5.4）。
fn write_rch_trim(freq: RchFreq) {
    backend::modify_rch_cr(|w| w.set_trim(freq.trim_value()));
}

/// RM 3.2.7 方案 1：RCH 为当前系统时钟时逐级换频。
///
/// 先把 HCLK 分频提到 /128，按 4M ↔ 8M ↔ 16M ↔ 24M/22.12M 逐级写 TRIM，
/// 分频在 [`init`] 末尾统一写回配置值。
fn rch_step_to(target: RchFreq) {
    backend::write_sysctrl0(|w| w.set_hclk_prs(HclkPrescaler::Div128 as u8));
    let mut level = RchFreq::from_trim(backend::read_rch_cr().trim()).step_level();
    let target_level = target.step_level();
    while level != target_level {
        level = if target_level > level {
            level + 1
        } else {
            level - 1
        };
        // 中间档写对应校准值；顶档直接写目标值（22.12M 或 24M）。
        let step = match level {
            0 => RchFreq::Mhz4,
            1 => RchFreq::Mhz8,
            2 => RchFreq::Mhz16,
            _ => target,
        };
        write_rch_trim(step);
    }
    write_rch_trim(target);
}

/// 按配置初始化时钟树。
///
/// 执行 RM 3.2.1 标准切换流程并写入频率快照（[`super::clocks`]）。可重复调
/// 用（运行中重新配置），旧时钟源为 XTH 时其频率未知，FLASH 等待周期
/// 按保守的 32 MHz 处理。
///
/// # Panics
///
/// `sys` 指定的源未在配置中启用（例如 `sys = Xth` 而 `xth = None`），
/// 或该源等待稳定超时。
pub(super) fn init(config: &Config) {
    let old = current_source();

    // ---- RCH 频率（RM 3.2.6/3.2.7）----
    if let Some(target) = config.rch {
        let current = RchFreq::from_trim(backend::read_rch_cr().trim());
        if current != target {
            if old == Sysclk::Rch {
                // RCH 正在驱动系统时钟：按 RM 3.2.7 方案 1 逐级换频。
                rch_step_to(target);
            } else {
                // RCH 非当前源：直接改写（RM 3.2.6 Step1）。
                write_rch_trim(target);
            }
        }
    }

    // ---- XTH（RM 3.2.3）----
    if let Some(xth) = config.xth {
        with_gpio_gate(|| crate::gpio::backend_setup_xth_pins(xth.bypass));
        // XTH_CR 不在解锁范围（RM 3.2.3 Step2 先于 Step4 解锁）。
        backend::modify_xth_cr(|w| {
            w.set_driver((xth.freq_range.encode() << 2) | xth.driver.encode());
            w.set_startup(xth.startup.encode());
        });
        if xth.bypass {
            backend::write_sysctrl1(|w| w.set_exth_en(true));
        }
        // 使能并等稳定（Step4~Step7）。
        backend::write_sysctrl0(|w| w.set_xth_en(true));
        let deadline = wait_stable(WaitSource::Xth);
        if !deadline {
            panic!("clock: XTH 等待稳定超时");
        }
        // RM 3.2.3 Step7：稳定后软件延时 10ms 以上再切换。
        delay_10ms();
    }

    // ---- RCL（RM 3.2.5 Step1~Step4）----
    if let Some(rcl) = config.rcl {
        backend::modify_rcl_cr(|w| {
            w.set_trim(rcl.trim_value());
            w.set_startup(RclStartup::Cycles256.encode());
        });
        backend::write_sysctrl0(|w| w.set_rcl_en(true));
        if !wait_stable(WaitSource::Rcl) {
            panic!("clock: RCL 等待稳定超时");
        }
    }

    // ---- XTL（RM 3.2.2）----
    if let Some(xtl) = config.xtl {
        with_gpio_gate(|| crate::gpio::backend_setup_xtl_pins(xtl.bypass));
        backend::modify_xtl_cr(|w| {
            w.set_driver((xtl.amplitude.encode() << 2) | xtl.driver.encode());
            w.set_startup(xtl.startup.encode());
        });
        if xtl.bypass {
            backend::write_sysctrl1(|w| w.set_extl_en(true));
        }
        backend::write_sysctrl0(|w| w.set_xtl_en(true));
        if !wait_stable(WaitSource::Xtl) {
            panic!("clock: XTL 等待稳定超时");
        }
    }

    // ---- 目标源频率 ----
    // sys = Xtl 必须提供 XTL 配置（保证振荡器已按 RM 3.2.2 使能并稳定），
    // 与 sys = Xth/Rcl 的守卫一致。
    if config.sys == Sysclk::Xtl && config.xtl.is_none() {
        panic!("clock: sys = Xtl 需要在 config.xtl 提供配置");
    }
    let sys_freq = match config.sys {
        Sysclk::Rch => match config.rch {
            Some(f) => f.frequency(),
            None => RchFreq::from_trim(backend::read_rch_cr().trim()).frequency(),
        },
        Sysclk::Xth => {
            config
                .xth
                .as_ref()
                .expect("clock: sys = Xth 需要在 config.xth 提供配置")
                .freq
        }
        Sysclk::Rcl => config
            .rcl
            .expect("clock: sys = Rcl 需要在 config.rcl 提供档位")
            .frequency(),
        Sysclk::Xtl => Hertz(32_768),
    };

    // ---- RCH 重新使能/稳定化（RM 3.2.6 Step3/Step4）----
    // 再入场景：目标源为 RCH 且当前系统时钟不在 RCH 上。无条件"先清后置
    // RCH_EN"重启振荡启动序列再等待 Stable：本次 init 可能刚直写过 TRIM
    // （RM 3.1.1"特定的更改时序"），TRIM 更改后 STABLE 是否重新清零手册
    // 未说明，重启使等待必然有效；RCH 未驱动系统时钟，清置窗口无手册禁
    // 忌，启动仅 4us（RM 3.1.1）代价可忽略。
    if config.sys == Sysclk::Rch && old != Sysclk::Rch {
        backend::write_sysctrl0(|w| w.set_rch_en(false));
        backend::write_sysctrl0(|w| w.set_rch_en(true));
        // RM 3.2.6 Step4："查询等待 RCH_CR.Stable 标志变为 1"。
        while !backend::read_rch_cr().stable() {}
    }

    // ---- FLASH 等待周期，前置（RM 3.2.1 Step4，按新旧源较高频率）----
    // 旧源为 XTH 时晶振频率未知，按 32 MHz 保守处理。
    let old_may_exceed_24m = match old {
        Sysclk::Xth => true,
        Sysclk::Rch | Sysclk::Rcl | Sysclk::Xtl => false,
    };
    if old_may_exceed_24m || sys_freq.0 > 24_000_000 {
        set_flash_wait(true);
    }

    // ---- 切换系统时钟源（RM 3.2.1 Step6）----
    if config.sys != old {
        backend::write_sysctrl0(|w| w.set_clk_sw4_sel(config.sys.encode()));
    }

    // ---- 分频（RM 3.5.1；RM 3.2.7 方案 1 的分频在此一并写回）----
    backend::write_sysctrl0(|w| {
        w.set_hclk_prs(config.hclk_pre as u8);
        w.set_pclk_prs(config.pclk_pre as u8);
    });

    // ---- FLASH 等待周期，按新源频率复配（RM 3.2.1 Step7）----
    set_flash_wait(sys_freq.0 > 24_000_000);

    // ---- 时钟故障检测（SYSCTRL1 位 4，SDK 定义 CLOCK_FT_EN）----
    // RM 3.5.2 位 4 标记 Reserved（差异台账已登记）；仅在用户显式开启
    // 时写 1，默认不触碰保留位。
    if config.clock_fault_detect {
        backend::write_sysctrl1(|w| w.set_clock_ft_en(true));
    }

    // ---- 关闭不再使用的旧源（RM 3.2.1 Step8）----
    // 仅处理本函数知情的能力：复位后仅 RCH 使能；XTH/RCL/XTL 若为旧源
    // 且未在配置中出现，说明是本函数之外的使能，不代为关闭。
    if old == Sysclk::Rch && config.rch.is_none() && config.sys != Sysclk::Rch {
        backend::write_sysctrl0(|w| w.set_rch_en(false));
    }

    // ---- 频率快照 ----
    let hclk = sys_freq / config.hclk_pre;
    let pclk = hclk / config.pclk_pre;
    store_clocks(Clocks {
        sys: sys_freq,
        hclk,
        pclk,
    });
}

/// 振荡稳定等待（`*_CR.STABLE`，RM 3.5.4~3.5.7）。
///
/// 带简单超时上限（STABLE 长期为 0 时避免死循环），硬件正常时远在超时
/// 前返回。手册未规定轮询超时值，上限按最长稳定时间的量级放宽取值。
fn wait_stable(source: WaitSource) -> bool {
    // 上限迭代数：按最慢时钟 XTL 2000ms 启动（DS 7.3.7）量级放宽。
    let deadline = 20_000_000u32;
    for _ in 0..deadline {
        let stable = match source {
            WaitSource::Xth => backend::read_xth_cr().stable(),
            WaitSource::Rcl => backend::read_rcl_cr().stable(),
            WaitSource::Xtl => backend::read_xtl_cr().stable(),
        };
        if stable {
            return true;
        }
    }
    false
}

enum WaitSource {
    Xth,
    Rcl,
    Xtl,
}

/// 设置 FLASH 等待周期（FLASH_CR.WAIT，RM 7.5）。
///
/// HCLK 0~24MHz 时 0 个等待周期、24~48MHz 时 1 个（DS 7.3.5）；改写前
/// 需向 FLASH_BYPASS 依次写 0x5A5A、0xA5A5 解锁（RM 7.7"寄存器写保护"：
/// 两写之间不可插入其他写操作、不可被中断打断；每次 Bypass 序列后只能
/// 修改一个寄存器）。写后按 RM 7.7 Step4 回读校验，不一致重试。
fn set_flash_wait(enable: bool) {
    // 先比对当前值，相同则无需触碰（RM 7.7 Step1）。
    if pac::FLASH.cr().read().wait() == enable {
        return;
    }
    // RM 7.7：Bypass 两写与目标写须关中断执行，结束后恢复原 PRIMASK。
    let was_masked: u32;
    unsafe {
        core::arch::asm!("mrs {0}, primask", out(reg) was_masked, options(nomem));
        core::arch::asm!("cpsid i", options(nomem, nostack));
    }
    for _ in 0..4 {
        pac::FLASH.bypass().write(|w| w.set_byseq(0x5A5A));
        pac::FLASH.bypass().write(|w| w.set_byseq(0xA5A5));
        pac::FLASH.cr().modify(|w| w.set_wait(enable));
        // RM 7.7 Step4：回读校验，不一致则重发序列。
        if pac::FLASH.cr().read().wait() == enable {
            break;
        }
    }
    if was_masked == 0 {
        unsafe { core::arch::asm!("cpsie i", options(nomem, nostack)) };
    }
    assert!(
        pac::FLASH.cr().read().wait() == enable,
        "clock: FLASH_CR.WAIT 写入校验失败"
    );
}
