//! 编译期驱动选择：从当前料号的 PAC 元数据派生 rustc cfg。
//!
//! 料号 feature 经 `dep:hc32-metapac` 同时激活普通依赖（寄存器访问）
//! 与本构建依赖（`metadata` feature，仅元数据），`hc32-metapac/<料号>`
//! 转发到两侧，保证两边读到同一颗芯片。普通依赖侧生成 PAC 时
//! metapac 构建脚本也会做同源的唯一性检查。
//!
//! 发出的 cfg 分两类：
//!
//! - 后端 cfg `<kind>_<version>`：外设实例的 `registers.kind` 与完整
//!   `registers.version` 拼接，如 GPIO → `gpio_ddl_v2`、CLOCK →
//!   `sysctrl_ddl_v2_2`。同名外设可能对应多种寄存器布局，驱动按此
//!   选择 IP 后端（AGENTS.md「从具体型号选择驱动」）。
//! - 实例 cfg `peri_<小写实例名>`：芯片拥有该外设实例。
//!
//! 已出现在 PAC 中但尚未适配的 IP 不发后端 cfg，保持私有。零料号时
//! 构建依赖不激活，本脚本不引用元数据，lib.rs 的 `compile_error!`
//! 负责诊断；两个料号时在读取元数据前拦下，给出 HAL 自己的中文提示。

use std::env;

// 构建依赖与普通依赖由同一料号 feature 激活（dep:hc32-metapac）；
// 零料号时两侧都不存在，元数据读取整体 cfg 掉。
#[cfg(feature = "_chip-selected")]
use hc32_metapac::metadata::METADATA;

/// HAL 已适配的驱动后端：`((kind, version), 组合依赖)`。
///
/// 组合依赖是驱动除自身寄存器外还需要的后端 cfg。按当前驱动代码核实：
/// - SYSCTRL（clock）：晶振引脚模拟模式经 GPIO 寄存器（RM 3.1），FLASH
///   等待周期经 FLASH 寄存器（RM 3.2.1）；
/// - BT/ADT/LPTIM/PCA：输出门控选择直接写 GPIO CTRLx（RM 9/11/12/13），
///   时钟门控与频率经 SYSCTRL；
/// - UART/LPUART：波特率定时器占用 BT（RM 15.1/16.1）、引脚复用经
///   GPIO、门控与频率经 SYSCTRL；
/// - CRC/WDT/CLK_TRIM：仅 SYSCTRL 门控。
///
/// 芯片拥有 IP 只说明寄存器布局，缺任一依赖的型号不开放该驱动。
const DRIVER_BACKENDS: &[((&str, &str), &[&str])] = &[
    (("gpio", "ddl_v2"), &["sysctrl_ddl_v2_2"]),
    (("sysctrl", "ddl_v2_2"), &["gpio_ddl_v2", "flash_ddl_v3"]),
    (("bt", "ddl_v1"), &["gpio_ddl_v2", "sysctrl_ddl_v2_2"]),
    (("adt", "ddl_v1"), &["gpio_ddl_v2", "sysctrl_ddl_v2_2"]),
    (("lptim", "ddl_v1"), &["gpio_ddl_v2", "sysctrl_ddl_v2_2"]),
    (("pca", "ddl_v1"), &["gpio_ddl_v2", "sysctrl_ddl_v2_2"]),
    (
        ("uart", "ddl_v1"),
        &["bt_ddl_v1", "gpio_ddl_v2", "sysctrl_ddl_v2_2"],
    ),
    (
        ("lpuart", "ddl_v1"),
        &["bt_ddl_v1", "gpio_ddl_v2", "sysctrl_ddl_v2_2"],
    ),
    (("crc", "ddl_v1"), &["sysctrl_ddl_v2_2"]),
    (("wdt", "ddl_v1"), &["sysctrl_ddl_v2_2"]),
    (("clk_trim", "ddl_v1"), &["sysctrl_ddl_v2_2"]),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // 无论是否选了料号，都先声明全部可能的 cfg 名，让零料号构建不产生
    // unexpected cfg 警告（Embassy 同款做法：declare all, enable some）。
    for ((kind, version), _) in DRIVER_BACKENDS {
        let cfg = format!("{kind}_{version}");
        println!("cargo:rustc-check-cfg=cfg({cfg})");
    }
    // 驱动源码引用的实例 cfg 固定为 L110 的实例名；后续型号接入时由元
    // 数据声明扩展（见 select_drivers）。
    for instance in [
        "peri_gpio",
        "peri_clock",
        "peri_flash",
        "peri_bt0",
        "peri_bt1",
        "peri_bt2",
        "peri_lptimer",
        "peri_pca",
        "peri_adt0",
        "peri_adt1",
        "peri_adt2",
        "peri_uart0",
        "peri_uart1",
        "peri_lpuart",
        "peri_crc",
        "peri_wdt",
        "peri_clk_trim",
    ] {
        println!("cargo:rustc-check-cfg=cfg({instance})");
    }

    // 与 metapac 构建脚本同源判断：HAL 料号 feature 以 CARGO_FEATURE_HC32
    // 开头。内部家族/封装 feature（hc32l110、hc32l110-full-pins）在
    // metapac 无同名芯片 feature，只选它们等于不选料号，需精确排除
    // （不能按前缀排除：hc32l110b4pa 等真料号同样以 hc32l110 开头）。
    const INTERNAL_FEATURES: &[&str] =
        &["CARGO_FEATURE_HC32L110", "CARGO_FEATURE_HC32L110_FULL_PINS"];
    let mut chip_features = env::vars().map(|(name, _)| name).filter(|name| {
        name.starts_with("CARGO_FEATURE_HC32") && !INTERNAL_FEATURES.contains(&name.as_str())
    });
    let chip_feature = chip_features.next();
    if chip_features.next().is_some() {
        panic!(
            "hc32-ddl-hal: 多个芯片 feature 同时启用；请只选择一个完整料号，\
             例如 hc32-ddl-hal = {{ features = [\"hc32l110c6ua\"] }}"
        );
    }

    // 零料号：构建依赖未激活，无法也不需要读元数据；lib.rs 的
    // compile_error! 负责诊断。
    let Some(chip_feature) = chip_feature else {
        return;
    };
    let feature_name = chip_feature
        .strip_prefix("CARGO_FEATURE_")
        .unwrap()
        .to_ascii_lowercase();

    #[cfg(feature = "_chip-selected")]
    select_drivers(feature_name, &METADATA);

    // 料号 feature 与构建依赖同时激活；hc32-metapac 缺失说明 Cargo.toml
    // 的料号 feature 表没有带 dep:hc32-metapac，构建期直接指出。
    #[cfg(not(feature = "_chip-selected"))]
    {
        let _ = feature_name;
        panic!(
            "hc32-ddl-hal: 料号 feature 未激活 hc32-metapac 构建依赖；请核对 Cargo.toml 料号 feature 表"
        );
    }
}

/// 按元数据发放驱动选择 cfg，见 [`main`] 的模块文档。
#[cfg(feature = "_chip-selected")]
fn select_drivers(feature_name: String, metadata: &hc32_metapac::metadata::Metadata) {
    // 元数据与选中 feature 必须指向同一颗芯片。metapac 侧同名 feature
    // 决定元数据，正常不会失配；失配说明 HAL 的料号 feature 表与
    // metapac 数据不同步，在这里比在驱动代码里出错更好定位。
    if !metadata.name.eq_ignore_ascii_case(&feature_name) {
        panic!(
            "hc32-ddl-hal: feature {feature_name} 对应 hc32-metapac 元数据中的 {}；\
             请核对 Cargo.toml 料号 feature 与 metapac 数据",
            metadata.name
        );
    }

    // 实例 cfg：芯片拥有该外设实例即开放，驱动按需引用。所有实例名
    // 先声明（零料号路径已在 main 声明过固定集合，这里覆盖元数据中的
    // 全部实例）。
    // MODE 拆分定时器的 views 共用物理实例的门控，不在此展开。
    for peripheral in metadata.peripherals {
        let instance_cfg = format!("peri_{}", peripheral.name.to_ascii_lowercase());
        println!("cargo:rustc-check-cfg=cfg({instance_cfg})");
        println!("cargo:rustc-cfg={instance_cfg}");
    }

    // 后端 cfg：只有驱动已适配的 kind_version 才开放，其余 IP（L110
    // 的 adc/flash/i2c/spi 等）保持私有。芯片拥有多个同布局实例时
    // 只发一次。
    //
    // 组合依赖是对芯片外设清单的要求（如 sysctrl 驱动写晶振引脚的
    // GPIO 寄存器、FLASH 等待周期寄存器），按元数据核对；被依赖的
    // IP 本身未适配驱动时不发 cfg，驱动代码经 PAC 常量直接访问其
    // 寄存器，由所在模块的门控保证只在依赖齐全时编译。
    let mut emitted = Vec::new();
    for peripheral in metadata.peripherals {
        let Some(registers) = peripheral.registers.as_ref() else {
            continue;
        };
        let backend_cfg = format!("{}_{}", registers.kind, registers.version);
        if emitted.contains(&backend_cfg) {
            continue;
        }
        let Some((_, deps)) = DRIVER_BACKENDS
            .iter()
            .find(|((kind, version), _)| *kind == registers.kind && *version == registers.version)
        else {
            continue;
        };

        let missing: Vec<&str> = deps
            .iter()
            .copied()
            .filter(|dep| {
                !metadata.peripherals.iter().any(|p| {
                    p.registers
                        .as_ref()
                        .is_some_and(|r| format!("{}_{}", r.kind, r.version) == *dep)
                })
            })
            .collect();
        if missing.is_empty() {
            println!("cargo:rustc-check-cfg=cfg({backend_cfg})");
            println!("cargo:rustc-cfg={backend_cfg}");
            emitted.push(backend_cfg);
        } else {
            println!(
                "cargo:warning=hc32-ddl-hal: {} 的驱动 {} 需要的后端 {} 缺失，\
                 该驱动在本型号上不开放",
                metadata.name,
                backend_cfg,
                missing.join(", ")
            );
        }
    }
}
