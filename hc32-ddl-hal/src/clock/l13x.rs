//! HC32L13x 时钟树切换流程（RM 3.2）。

use super::{
    Config, HclkPrescaler, PllConfig, PllSource, RchFreq, RclFreq, RclStartup, Sysclk, backend,
    store_clocks,
};
use crate::Hertz;
use crate::clock::Clocks;

pub(super) fn init(config: &Config) {
    let sys_freq = source_frequency(config.sys, config);
    if let Some(pll) = config.pll {
        pll_frequency(pll, config);
    }

    let old = current_source();
    let mut current = old;

    // PLL 在参考源为 RCH 时禁止更改 RCH_TRIM，且 PLL 不能直接切到
    // RCL/XTL。重入时先经 RCH 离开 PLL，再执行常规配置。
    if current == Sysclk::Pll {
        leave_pll();
        current = Sysclk::Rch;
    }

    if let Some(target) = config.rch {
        let current_rch = RchFreq::from_trim(backend::read_rch_cr().trim());
        if current_rch != target {
            if current == Sysclk::Rch {
                rch_step_to(target);
            } else {
                write_rch_trim(target);
            }
        }
    }

    if let Some(xth) = config.xth {
        with_gpio_gate(|| crate::gpio::backend_setup_xth_pins(xth.bypass));
        backend::modify_xth_cr(|w| {
            w.set_driver(xth.driver.encode());
            w.set_xth_fsel(xth.freq_range.encode());
            w.set_startup(xth.startup.encode());
        });
        backend::write_sysctrl1(|w| w.set_exth_en(xth.bypass));
        backend::write_sysctrl0(|w| w.set_xth_en(true));
        assert!(wait_stable(WaitSource::Xth), "clock: XTH 等待稳定超时");
        delay_at_least(10_000);
    }

    if let Some(rcl) = config.rcl {
        backend::modify_rcl_cr(|w| {
            w.set_trim(rcl.trim_value());
            w.set_startup(RclStartup::Cycles256.encode());
        });
        backend::write_sysctrl0(|w| w.set_rcl_en(true));
        assert!(wait_stable(WaitSource::Rcl), "clock: RCL 等待稳定超时");
    }

    if let Some(xtl) = config.xtl {
        with_gpio_gate(|| crate::gpio::backend_setup_xtl_pins(xtl.bypass));
        backend::modify_xtl_cr(|w| {
            w.set_driver(xtl.driver.encode());
            w.set_amp_sel(xtl.amplitude.encode());
            w.set_startup(xtl.startup.encode());
        });
        backend::write_sysctrl1(|w| w.set_extl_en(xtl.bypass));
        backend::write_sysctrl0(|w| w.set_xtl_en(true));
        assert!(wait_stable(WaitSource::Xtl), "clock: XTL 等待稳定超时");
    }

    if let Some(pll) = config.pll {
        let reference = match pll.source {
            PllSource::Rch => Sysclk::Rch,
            PllSource::Xth => Sysclk::Xth,
        };
        if current != reference {
            if reference == Sysclk::Rch {
                enable_rch();
            }
            switch_to(reference, config);
            current = reference;
        }

        backend::set_bgr(true);
        delay_at_least(20);
        if backend::read_sysctrl0().pll_en() {
            backend::write_sysctrl0(|w| w.set_pll_en(false));
        }
        let input = pll_input_frequency(pll, config);
        let output = pll_frequency(pll, config);
        backend::modify_pll_cr(|w| {
            w.set_refsel(match pll.source {
                PllSource::Rch => 3,
                PllSource::Xth if config.xth.unwrap().bypass => 1,
                PllSource::Xth => 0,
            });
            w.set_frsel(match input.0 {
                4_000_000..=6_000_000 => 0,
                6_000_001..=12_000_000 => 1,
                12_000_001..=20_000_000 => 2,
                _ => 3,
            });
            w.set_divn(pll.multiplier);
            w.set_fosc(match output.0 {
                8_000_000..=12_000_000 => 0,
                12_000_001..=18_000_000 => 1,
                18_000_001..=24_000_000 => 2,
                24_000_001..=36_000_000 => 3,
                _ => 4,
            });
            w.set_startup(7);
        });
        backend::write_sysctrl0(|w| w.set_pll_en(true));
        backend::set_flash_wait(u8::from(output.0 > 24_000_000));
        assert!(wait_stable(WaitSource::Pll), "clock: PLL 等待稳定超时");
    }

    if config.sys == Sysclk::Rch && current != Sysclk::Rch {
        enable_rch();
    }
    if config.sys != current {
        switch_to(config.sys, config);
    }

    backend::write_sysctrl0(|w| {
        w.set_hclk_prs(config.hclk_pre as u8);
        w.set_pclk_prs(config.pclk_pre as u8);
    });
    backend::set_flash_wait(u8::from(sys_freq.0 > 24_000_000));

    let pll_uses_rch = config.pll.is_some_and(|pll| pll.source == PllSource::Rch);
    if old == Sysclk::Rch && config.rch.is_none() && config.sys != Sysclk::Rch && !pll_uses_rch {
        backend::write_sysctrl0(|w| w.set_rch_en(false));
    }
    // 上次可作为非系统源显式开启 PLL；本次未配置时连同其 BGR 关闭，
    // 避免重入后遗留数十微安的深度休眠功耗（RM 3.1.5、3.2.1 Step8）。
    if config.pll.is_none() && backend::read_sysctrl0().pll_en() {
        backend::write_sysctrl0(|w| w.set_pll_en(false));
        backend::set_bgr(false);
    }

    store_clocks(Clocks {
        sys: sys_freq,
        hclk: sys_freq / config.hclk_pre,
        pclk: sys_freq / config.hclk_pre / config.pclk_pre,
    });
}

fn current_source() -> Sysclk {
    Sysclk::decode(backend::read_sysctrl0().clksw()).unwrap_or(Sysclk::Rch)
}

fn source_frequency(source: Sysclk, config: &Config) -> Hertz {
    match source {
        Sysclk::Rch => match config.rch {
            Some(freq) => freq.frequency(),
            None => RchFreq::from_trim(backend::read_rch_cr().trim()).frequency(),
        },
        Sysclk::Xth => {
            config
                .xth
                .as_ref()
                .expect("clock: sys/PLL 使用 XTH 时需要在 config.xth 提供配置")
                .freq
        }
        Sysclk::Rcl => config
            .rcl
            .expect("clock: sys = Rcl 需要在 config.rcl 提供档位")
            .frequency(),
        Sysclk::Xtl => {
            assert!(
                config.xtl.is_some(),
                "clock: sys = Xtl 需要在 config.xtl 提供配置"
            );
            Hertz(32_768)
        }
        Sysclk::Pll => pll_frequency(
            config
                .pll
                .expect("clock: sys = Pll 需要在 config.pll 提供配置"),
            config,
        ),
    }
}

fn pll_input_frequency(pll: PllConfig, config: &Config) -> Hertz {
    let input = match pll.source {
        PllSource::Rch => source_frequency(Sysclk::Rch, config),
        PllSource::Xth => source_frequency(Sysclk::Xth, config),
    };
    assert!(
        (4_000_000..=24_000_000).contains(&input.0),
        "clock: PLL 输入频率超出 4~24MHz"
    );
    input
}

fn pll_frequency(pll: PllConfig, config: &Config) -> Hertz {
    assert!(
        (2..=12).contains(&pll.multiplier),
        "clock: PLL 倍频系数超出 2~12"
    );
    let output = pll_input_frequency(pll, config).0 * pll.multiplier as u32;
    assert!(
        (8_000_000..=48_000_000).contains(&output),
        "clock: PLL 输出频率超出 8~48MHz"
    );
    Hertz(output)
}

fn current_source_frequency() -> Hertz {
    match current_source() {
        Sysclk::Rch => RchFreq::from_trim(backend::read_rch_cr().trim()).frequency(),
        Sysclk::Xth => Hertz::mhz(32),
        Sysclk::Rcl => {
            let trim = backend::read_rcl_cr().trim();
            if trim == RclFreq::K38_4.trim_value() {
                RclFreq::K38_4.frequency()
            } else {
                RclFreq::K32_768.frequency()
            }
        }
        Sysclk::Xtl => Hertz(32_768),
        Sysclk::Pll => {
            let pll = backend::read_pll_cr();
            let input = if pll.refsel() == 3 {
                RchFreq::from_trim(backend::read_rch_cr().trim())
                    .frequency()
                    .0
            } else {
                32_000_000
            };
            Hertz(input.saturating_mul(pll.divn() as u32))
        }
    }
}

fn switch_to(target: Sysclk, config: &Config) {
    let old = current_source_frequency();
    let new = source_frequency(target, config);
    backend::set_flash_wait(u8::from(old.0.max(new.0) > 24_000_000));
    backend::write_sysctrl0(|w| w.set_clksw(target.encode()));
    backend::set_flash_wait(u8::from(new.0 > 24_000_000));
}

fn enable_rch() {
    backend::write_sysctrl0(|w| w.set_rch_en(true));
    assert!(wait_stable(WaitSource::Rch), "clock: RCH 等待稳定超时");
}

fn leave_pll() {
    enable_rch();
    backend::set_flash_wait(1);
    backend::write_sysctrl0(|w| w.set_clksw(Sysclk::Rch.encode()));
    backend::set_flash_wait(0);
    backend::write_sysctrl0(|w| w.set_pll_en(false));
    backend::set_bgr(false);
}

fn write_rch_trim(freq: RchFreq) {
    backend::modify_rch_cr(|w| w.set_trim(freq.trim_value()));
}

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

fn delay_at_least(us: u32) {
    let hclk = current_source_frequency().0 >> backend::read_sysctrl0().hclk_prs();
    let cycles = (hclk.min(48_000_000) as u64) * us as u64;
    let iterations = ((cycles + 2_999_999) / 3_000_000).max(100) as u32;
    for i in 0..iterations {
        core::hint::black_box(i);
    }
}

fn wait_stable(source: WaitSource) -> bool {
    for _ in 0..20_000_000 {
        let stable = match source {
            WaitSource::Rch => backend::read_rch_cr().stable(),
            WaitSource::Xth => backend::read_xth_cr().stable(),
            WaitSource::Rcl => backend::read_rcl_cr().stable(),
            WaitSource::Xtl => backend::read_xtl_cr().stable(),
            WaitSource::Pll => backend::read_pll_cr().stable(),
        };
        if stable {
            return true;
        }
    }
    false
}

enum WaitSource {
    Rch,
    Xth,
    Rcl,
    Xtl,
    Pll,
}
