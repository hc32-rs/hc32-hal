//! HC32F420 时钟树配置（RM 3.1/3.2）。

use core::sync::atomic::Ordering;

use super::{Clocks, backend};
use crate::Hertz;
use crate::time::Prescaler;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Sysclk {
    Rch,
    Xth,
    Rcl,
    Pll,
}

impl Sysclk {
    const fn encode(self) -> u8 {
        match self {
            Self::Rch => 0,
            Self::Xth => 1,
            Self::Rcl => 2,
            Self::Pll => 4,
        }
    }

    const fn decode(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Rch),
            1 => Some(Self::Xth),
            2 => Some(Self::Rcl),
            4 => Some(Self::Pll),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum RchFreq {
    /// 22.12 MHz，出厂校准值位于 0x0100_11E0。
    Mhz22_12,
    /// 24 MHz，出厂校准值位于 0x0100_11E4。
    #[default]
    Mhz24,
}

impl RchFreq {
    pub const fn frequency(self) -> Hertz {
        match self {
            Self::Mhz22_12 => Hertz(22_120_000),
            Self::Mhz24 => Hertz::mhz(24),
        }
    }

    fn trim(self) -> u16 {
        let address = match self {
            Self::Mhz22_12 => 0x0100_11E0,
            Self::Mhz24 => 0x0100_11E4,
        };
        // safety: 芯片固定的已映射只读校准区（RM 3.5.10）。
        unsafe { (address as *const u16).read_volatile() }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum RchPrescaler {
    Div2 = 0,
    Div4 = 1,
    #[default]
    Div6 = 2,
    Div8 = 3,
    Div10 = 4,
    Div12 = 5,
    Div14 = 6,
    Div16 = 7,
    Div1 = 8,
}

impl Prescaler for RchPrescaler {
    fn num(&self) -> u32 {
        1
    }

    fn denom(&self) -> u32 {
        match self {
            Self::Div1 => 1,
            Self::Div2 => 2,
            Self::Div4 => 4,
            Self::Div6 => 6,
            Self::Div8 => 8,
            Self::Div10 => 10,
            Self::Div12 => 12,
            Self::Div14 => 14,
            Self::Div16 => 16,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct RchConfig {
    pub freq: RchFreq,
    pub divider: RchPrescaler,
}

impl RchConfig {
    pub fn frequency(self) -> Hertz {
        self.freq.frequency() / self.divider
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RclFreq {
    K38_4,
    K32_768,
}

impl RclFreq {
    pub const fn frequency(self) -> Hertz {
        match self {
            Self::K38_4 => Hertz(38_400),
            Self::K32_768 => Hertz(32_768),
        }
    }

    fn trim(self) -> u16 {
        let address = match self {
            Self::K38_4 => 0x0100_11EC,
            Self::K32_768 => 0x0100_11E8,
        };
        // safety: 芯片固定的已映射只读校准区（RM 3.5.11）。
        unsafe { (address as *const u16).read_volatile() }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum XthFreqRange {
    Mhz8_12,
    Mhz8_16,
    Mhz16_24,
    Mhz20_32,
}

impl XthFreqRange {
    const fn encode(self) -> u8 {
        match self {
            Self::Mhz8_12 => 0,
            Self::Mhz8_16 => 1,
            Self::Mhz16_24 => 2,
            Self::Mhz20_32 => 3,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum XthDriver {
    Weakest,
    Weak,
    Default,
    Strongest,
}

impl XthDriver {
    const fn encode(self) -> u8 {
        match self {
            Self::Weakest => 0,
            Self::Weak => 1,
            Self::Default => 2,
            Self::Strongest => 3,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum XthStartup {
    Cycles8192,
    Cycles32768,
    Cycles131072,
    Cycles262144,
}

impl XthStartup {
    const fn encode(self) -> u8 {
        match self {
            Self::Cycles8192 => 0,
            Self::Cycles32768 => 1,
            Self::Cycles131072 => 2,
            Self::Cycles262144 => 3,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct XthConfig {
    pub freq_range: XthFreqRange,
    pub driver: XthDriver,
    pub startup: XthStartup,
    pub bypass: bool,
    pub freq: Hertz,
}

impl XthConfig {
    pub fn crystal(freq: Hertz) -> Self {
        assert!(
            (8_000_000..=32_000_000).contains(&freq.0),
            "XTH 晶振频率超出 8~32MHz"
        );
        let freq_range = match freq.0 {
            0..=12_000_000 => XthFreqRange::Mhz8_12,
            12_000_001..=16_000_000 => XthFreqRange::Mhz8_16,
            16_000_001..=24_000_000 => XthFreqRange::Mhz16_24,
            _ => XthFreqRange::Mhz20_32,
        };
        Self {
            freq_range,
            driver: XthDriver::Default,
            startup: XthStartup::Cycles262144,
            bypass: false,
            freq,
        }
    }

    pub fn bypass(freq: Hertz) -> Self {
        assert!(
            (8_000_000..=32_000_000).contains(&freq.0),
            "XTH 外部输入频率超出 8~32MHz"
        );
        Self {
            // RM 未说明旁路模式下 RANGE/DRIVER 是否有效，保留复位等价编码。
            freq_range: XthFreqRange::Mhz8_12,
            driver: XthDriver::Default,
            startup: XthStartup::Cycles262144,
            bypass: true,
            freq,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PllSource {
    Rch,
    Xth,
}

impl PllSource {
    const fn encode(self) -> u8 {
        match self {
            Self::Rch => 0,
            Self::Xth => 1,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PllOutputDiv {
    Div1 = 0,
    Div2 = 1,
    Div4 = 2,
    Div8 = 3,
}

impl Prescaler for PllOutputDiv {
    fn num(&self) -> u32 {
        1
    }

    fn denom(&self) -> u32 {
        1 << (*self as u32)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PllStartup {
    Cycles128 = 0,
    Cycles256 = 1,
    Cycles512 = 2,
    Cycles1024 = 3,
    Cycles2048 = 4,
    Cycles4096 = 5,
    Cycles8192 = 6,
    Cycles16384 = 7,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PllConfig {
    pub source: PllSource,
    pub mulm: u8,
    pub divn: u8,
    pub output_div: PllOutputDiv,
    pub startup: PllStartup,
}

impl PllConfig {
    pub const fn new(source: PllSource, mulm: u8, divn: u8, output_div: PllOutputDiv) -> Self {
        Self {
            source,
            mulm,
            divn,
            output_div,
            startup: PllStartup::Cycles16384,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum HclkPrescaler {
    #[default]
    Div1 = 0,
    Div2 = 1,
    Div4 = 2,
    Div8 = 3,
    Div16 = 4,
    Div32 = 5,
    Div64 = 6,
    Div128 = 7,
    Div256 = 8,
    Div512 = 9,
}

impl Prescaler for HclkPrescaler {
    fn num(&self) -> u32 {
        1
    }

    fn denom(&self) -> u32 {
        1 << (*self as u32)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum PclkPrescaler {
    #[default]
    Div1 = 0,
    Div2 = 1,
    Div4 = 2,
    Div8 = 3,
}

impl Prescaler for PclkPrescaler {
    fn num(&self) -> u32 {
        1
    }

    fn denom(&self) -> u32 {
        1 << (*self as u32)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Config {
    /// RCH 配置；`None` 表示目标时钟树不使用 RCH。
    pub rch: Option<RchConfig>,
    /// XTH 配置；`sys = Xth` 或 PLL 选择 XTH 输入时必须提供。
    pub xth: Option<XthConfig>,
    /// RCL 配置；`sys = Rcl` 时必须提供。
    pub rcl: Option<RclFreq>,
    /// PLL 配置；`sys = Pll` 时必须提供。
    pub pll: Option<PllConfig>,
    pub sys: Sysclk,
    pub hclk_pre: HclkPrescaler,
    pub pclk0_pre: PclkPrescaler,
    pub pclk1_pre: PclkPrescaler,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rch: Some(RchConfig::default()),
            xth: None,
            rcl: None,
            pll: None,
            sys: Sysclk::Rch,
            hclk_pre: HclkPrescaler::Div1,
            pclk0_pre: PclkPrescaler::Div1,
            pclk1_pre: PclkPrescaler::Div1,
        }
    }
}

fn pll_frequency(config: &Config, pll: &PllConfig) -> Hertz {
    assert!((2..=63).contains(&pll.mulm), "PLL MULM 必须在 2~63");
    assert!((1..=63).contains(&pll.divn), "PLL DIVN 必须在 1~63");
    let fin = match pll.source {
        PllSource::Rch => {
            let rch = config
                .rch
                .expect("clock: PLL 选择 RCH 输入时需要 config.rch");
            let frequency = rch.frequency().0;
            assert!(
                (4_000_000..=12_000_000).contains(&frequency),
                "clock: PLL 的 RCHCLKD 输入必须为 4~12MHz"
            );
            frequency
        }
        PllSource::Xth => {
            config
                .xth
                .as_ref()
                .expect("clock: PLL 选择 XTH 输入时需要 config.xth")
                .freq
                .0
        }
    };
    assert!(
        (4_000_000..=32_000_000).contains(&fin),
        "clock: PLL FIN 必须在 4~32MHz"
    );

    let divn = u64::from(pll.divn);
    let fin = u64::from(fin);
    let vco_numerator = fin * u64::from(pll.mulm);
    assert!(
        fin >= 4_000_000 * divn && fin <= 16_000_000 * divn,
        "clock: PLL FIN/DIVN 必须在 4~16MHz"
    );
    assert!(
        (120_000_000 * divn..=200_000_000 * divn).contains(&vco_numerator),
        "clock: PLL FIN*MULM/DIVN 必须在 120~200MHz"
    );
    let output_div = u64::from(pll.output_div.denom());
    let output_denominator = divn * output_div;
    assert!(
        vco_numerator >= 15_000_000 * output_denominator
            && vco_numerator <= 84_000_000 * output_denominator,
        "clock: PLL FOUT 必须在 15~84MHz"
    );
    Hertz((vco_numerator / output_denominator) as u32)
}

fn configured_frequencies(config: &Config) -> (Hertz, Hertz) {
    if let Some(xth) = config.xth {
        assert!(
            (8_000_000..=32_000_000).contains(&xth.freq.0),
            "clock: XTH 频率必须在 8~32MHz"
        );
    }
    let pll = config.pll.map(|pll| pll_frequency(config, &pll));
    let sys = match config.sys {
        Sysclk::Rch => config
            .rch
            .expect("clock: sys = Rch 需要 config.rch")
            .frequency(),
        Sysclk::Xth => config.xth.expect("clock: sys = Xth 需要 config.xth").freq,
        Sysclk::Rcl => config
            .rcl
            .expect("clock: sys = Rcl 需要 config.rcl")
            .frequency(),
        Sysclk::Pll => pll.expect("clock: sys = Pll 需要 config.pll"),
    };
    let hclk = sys / config.hclk_pre;
    assert!(hclk.0 <= 84_000_000, "clock: HCLK 不能超过 84MHz");
    (sys, hclk)
}

const fn flash_wait(hclk: u32) -> u8 {
    if hclk <= 24_000_000 {
        0
    } else if hclk <= 48_000_000 {
        1
    } else if hclk <= 72_000_000 {
        2
    } else {
        3
    }
}

const fn transition_prescaler(max_sysclk: u32) -> HclkPrescaler {
    if max_sysclk <= 24_000_000 {
        HclkPrescaler::Div1
    } else if max_sysclk <= 48_000_000 {
        HclkPrescaler::Div2
    } else {
        HclkPrescaler::Div4
    }
}

fn current_source() -> Sysclk {
    Sysclk::decode(backend::read_cr0().clksrc()).unwrap_or(Sysclk::Rch)
}

fn wait_stable(source: Sysclk) {
    for _ in 0..20_000_000 {
        let state = backend::read_state();
        let stable = match source {
            Sysclk::Rch => state.rch(),
            Sysclk::Xth => state.xth(),
            Sysclk::Rcl => state.rcl(),
            Sysclk::Pll => state.pll(),
        };
        if stable {
            return;
        }
    }
    panic!("clock: 等待时钟源稳定超时");
}

fn with_gpiod_gate(f: impl FnOnce()) {
    // 外部输入经 GPIO 数字输入路径进入 XTH，使用期间保持 GPIOD 门控。
    backend::enable_gpio(3);
    f();
}

fn delay_20us(hclk: u32) {
    const CSR: *mut u32 = 0xE000_E010 as *mut u32;
    const RVR: *mut u32 = 0xE000_E014 as *mut u32;
    const CVR: *mut u32 = 0xE000_E018 as *mut u32;
    cortex_m::interrupt::free(|_| {
        // safety: Cortex-M4 固定的 SysTick 寄存器。PLL 初始化期间暂时借用
        // SysTick，结束后恢复控制和重载值；当前计数值按硬件写语义清零。
        unsafe {
            let saved_csr = CSR.read_volatile();
            let saved_rvr = RVR.read_volatile();
            CSR.write_volatile(0);
            RVR.write_volatile((hclk / 50_000 - 1).min(0x00FF_FFFF));
            CVR.write_volatile(0);
            CSR.write_volatile((1 << 2) | 1);
            while CSR.read_volatile() & (1 << 16) == 0 {}
            CSR.write_volatile(0);
            RVR.write_volatile(saved_rvr);
            CVR.write_volatile(0);
            CSR.write_volatile(saved_csr);
        }
    });
}

/// 按 RM 3.2 配置 HC32F420 的四个系统时钟源、PLL 和三级总线分频。
///
/// 配置在写寄存器前完整校验。重配时先切到 RCH，并用临时 HCLK 分频保证
/// 切换过程不超过 24 MHz；FLASH_WAIT 在升频前、降频后更新。未使用的旧
/// 时钟源在最终切换完成后关闭。PLL 所需的 ADC0 BGR 按 SDK 例程开启并
/// 保持使能。
pub fn init(config: &Config) {
    let (sys, hclk) = configured_frequencies(config);
    let old = super::clocks();
    let old_source = current_source();
    let max_sysclk = if old_source == Sysclk::Rch {
        old.sys.0.max(sys.0)
    } else {
        old.sys.0.max(sys.0).max(24_000_000)
    };
    let transition_pre = transition_prescaler(max_sysclk);
    let transition_wait = flash_wait(old.hclk.0.max(hclk.0));

    backend::set_flash_wait(transition_wait);
    backend::modify_cr0(|w| w.set_hclkprs(transition_pre as u8));

    if old_source != Sysclk::Rch {
        if !backend::read_cr2().rch() {
            backend::modify_cr2(|w| w.set_rch(true));
        }
        wait_stable(Sysclk::Rch);
        backend::modify_cr0(|w| w.set_clksrc(Sysclk::Rch.encode()));
    }
    if old_source == Sysclk::Pll {
        backend::modify_cr2(|w| w.set_pll(false));
    }
    // 旧 PLL 可能使用 RCH 输入，必须先离开并关闭它再恢复出厂 TRIM。
    backend::init_default();

    if let Some(rch) = config.rch {
        crate::pac::SYSCTRL
            .rchtrim()
            .modify(|w| w.set_trim(rch.freq.trim()));
        backend::modify_rch(|w| w.set_div(rch.divider as u8));
        if !backend::read_cr2().rch() {
            backend::modify_cr2(|w| w.set_rch(true));
        }
        wait_stable(Sysclk::Rch);
    }

    if let Some(xth) = config.xth {
        with_gpiod_gate(|| crate::gpio::backend_setup_xth_pins(xth.bypass));
        backend::modify_xth(|w| {
            w.set_driver(xth.driver.encode());
            w.set_range(xth.freq_range.encode());
            w.set_waitcycle(xth.startup.encode());
        });
        backend::modify_cr2(|w| {
            w.set_exxth(xth.bypass);
            w.set_xth(true);
        });
        wait_stable(Sysclk::Xth);
    }

    if let Some(rcl) = config.rcl {
        crate::pac::SYSCTRL
            .rcltrim()
            .modify(|w| w.set_trim(rcl.trim()));
        backend::modify_rcl(|w| w.set_waitcycle(3));
        backend::modify_cr2(|w| w.set_rcl(true));
        wait_stable(Sysclk::Rcl);
    }

    if let Some(pll) = config.pll {
        if backend::read_cr2().pll() {
            backend::modify_cr2(|w| w.set_pll(false));
        }
        backend::enable_bgr();
        delay_20us(24_000_000 / transition_pre.denom());
        backend::modify_pll(|w| {
            w.set_src(pll.source.encode());
            w.set_od(pll.output_div as u8);
            w.set_waitcycle(pll.startup as u8);
            w.set_oen(true);
            w.set_divn(pll.divn);
            w.set_bypass(false);
            w.set_lken(false);
            w.set_mulm(pll.mulm);
        });
        backend::modify_cr2(|w| w.set_pll(true));
        wait_stable(Sysclk::Pll);
    }

    if config.sys != Sysclk::Rch {
        backend::modify_cr0(|w| w.set_clksrc(config.sys.encode()));
    }
    backend::modify_cr0(|w| {
        w.set_hclkprs(config.hclk_pre as u8);
        w.set_pclk0prs(config.pclk0_pre as u8);
        w.set_pclk1prs(config.pclk1_pre as u8);
    });
    backend::set_flash_wait(flash_wait(hclk.0));

    if config.rch.is_none() {
        backend::modify_cr2(|w| w.set_rch(false));
    }
    if config.xth.is_none() {
        backend::modify_cr2(|w| {
            w.set_xth(false);
            w.set_exxth(false);
        });
    }
    if config.rcl.is_none() {
        backend::modify_cr2(|w| w.set_rcl(false));
    }
    if config.pll.is_none() {
        backend::modify_cr2(|w| w.set_pll(false));
    }

    let pclk0 = hclk / config.pclk0_pre;
    let pclk1 = hclk / config.pclk1_pre;
    super::store_clocks(Clocks {
        sys,
        hclk,
        pclk: pclk0,
    });
    super::PCLK1_HZ.store(pclk1.0, Ordering::Relaxed);
}
