# hc32-hal 开发约定

## 实现边界

- 本仓库是 Cargo workspace。`hc32-ddl-hal/` 和 `hc32-ll-hal/` 是独立版本、独立发布的 `no_std` crate。按芯片所属的 SDK 系列选择 crate，再按实际 IP 版本实现驱动。
- 时钟、复位、引脚复用、DMA 请求和中断绑定由各 crate 按硬件语义实现。比如核对 DMA 路由时，DDL 查 TRI_SEL，LL 查 EVT_SRC，两者的请求编号都不能直接当作物理通道号。
- HAL 的 API 风格与代码组织遵循 Embassy STM32 HAL（模块划分、类型构造、命名等），参考路径见下文资料表；Embassy 做法与 HC32 硬件语义不符时，按硬件实际调整。
- 应用层有对应标准接口时，优先实现 embedded-hal 系列 trait。等两侧出现语义一致的实际实现后，再考虑提取共享代码。
- 新增芯片时，根据对应数据仓库的芯片和外设描述选择 PAC feature。按具体型号验证引脚、外设实例和中断；同名外设可能有多种布局。

## HAL 源码布局与命名

以下路径相对于各 HAL crate。文件和模块统一用小写 `snake_case`，DDL 和 LL 由 crate 名区分。源码随功能落地时创建。

| 放置位置 | 内容 |
| --- | --- |
| `src/lib.rs` | crate 入口，声明公开模块，提供顶层 `Config`、`init` 和 `Peripherals`。初始化入口调用各模块完成硬件配置。 |
| `src/<模块名>.rs` | 一个外设或子系统的公开入口。GPIO 用 `gpio.rs`，UART 用 `uart.rs`，系统时钟统一用 `clock.rs`。GPIO 通过时钟模块操作门控，UART 从时钟模块获取总线频率。 |
| `src/<模块名>/mod.rs` | 模块需要多个文件时，将原入口移到这里，保留原有公开路径。例如 `src/uart.rs` 拆分后，用户仍通过 `uart::` 访问驱动。 |
| `src/<模块名>/<功能>.rs` | 模块内可以独立描述的功能，如 `uart/buffered.rs`、`timer/pwm.rs`。 |
| `src/<模块名>/<kind>_<版本>.rs` | 确需不同寄存器操作时，放置私有 IP 实现。文件名取上游寄存器类型和版本，在 crate 内省略 `ddl_` 或 `ll_` 前缀，保留完整数字后缀。例如 HC32L110 的 `CLOCK` 映射为 `sysctrl/ddl_v2_2`，分拆时对应 `clock/sysctrl_v2_2.rs`。 |

首个 IP 可以用单文件。接入第二种寄存器布局时，把沿用的公开类型、构造函数和 embedded-hal trait 实现保留在模块入口，把有差异的寄存器操作移到私有 IP 文件。经核对寄存器和硬件语义一致的操作可以共用。按实例或工作模式选择版本，同一颗芯片也可能有多种布局，例如 HC32A448 的 ADC1 和 ADC2。

### 从具体型号选择驱动

- 应用只选择一个完整料号的 Cargo feature。构建时从该型号的 PAC 元数据读取已实现外设实例的 `registers.kind` 和完整 `registers.version`，据此选择后端。按已适配的实例及其依赖一起开放驱动模块和 `Peripherals` 字段。芯片拥有某个 IP 仅说明寄存器布局，驱动还可能依赖另一个定时器、时钟源或引脚路由。
- GPIO 与系统控制器分别选择后端。例如 HC32L110 使用 `gpio/ddl_v2` 和 `CLOCK → sysctrl/ddl_v2_2`，HC32L130 使用 `gpio/ddl_v3_3` 和 `SYSCTRL → sysctrl/ddl_v3_1`。公开的 `gpio::Input`、`gpio::Output` 等类型沿用现有路径。引脚身份要能无歧义地表示端口和位号，例如 L130 的 PA15 与 PB07。
- IP 后端负责寄存器访问，封装决定可发放的引脚和未引出脚的上电处理。按完整料号核对 DS 引脚表。PAC 缺少封装引脚数据时，可维护一份带手册出处的映射，由它派生引脚类型、集合字段和构造。每项封装初始化明确匹配适用型号，尤其要核对复位脚、调试脚与 GPIO 模拟模式的默认状态。
- 默认时钟路径按各型号的复位条件实现，区分时钟源和分频、出厂校准、GPIO 门控三件事，并核对厂商启动代码在 Rust 启动路径中是否执行。仅接入默认时钟的型号只开放已实现的时钟能力。扩展型号时保持既有型号的时钟配置 API 和板级行为。

芯片的实例、引脚复用和中断映射优先来自 PAC 元数据。确需手写的系列差异就近放在所属模块，例如时钟树特例放在 `clock/<系列名>.rs`。开发板上的 LED、按键、接线和晶振选型归入对应板级例程。

## Examples 放置

每块开发板一个独立 Cargo 包，例程路径为 `examples/<ddl 或 ll>/<开发板名>/src/bin/<例程名>.rs`。自制最小系统用具体芯片型号和接线定义目标。包内的 `Cargo.toml` 声明空的 `[workspace]`，通过 path 依赖对应 HAL 并固定板上芯片的 feature。同板例程共享编译目标、烧录配置和依赖。

Cortex-M 板级例程使用 `cortex-m-rt` 的 `#[entry]` 和标准 `link.x`。由 PAC 提供芯片的 `device.x`、中断向量和 `memory.x`，在板级 target 配置中传入 `-Tlink.x`。在板包目录执行 `cargo build --bins --release` 验证最终链接。

每个例程可独立运行，文件名描述用户能操作或观察到的行为。DDL 首批以 `examples/ddl/hc32l110-stk/` 为目标，依次适配 `clock_div.rs`（测量分频后的输出周期）、`blinky.rs`（LED 闪烁）和 `button_led.rs`（轮询按键控制 LED）。中断驱动就绪后可加入 `button_irq.rs`（按键边沿触发翻转）。其他板可沿用相同的例程名，按实际接线和验证需求添加例程。日志推荐使用 SEGGER RTT，可复用的 API 片段可放在 rustdoc 中。

板级 `README.md` 写明开发板名、芯片完整型号、接线和有效电平，为每个例程列出运行命令和预期现象。上板后按例程记录实测结果、外设实例、IP 版本及观察到的行为。驱动、例程或板测结果变化时，同步更新[系列与外设支持表](docs/peripheral-support.md)，将 ✅ 链到板级 README 的对应实测条目。例如，`button_led` 可记录 GPIO 输入、输出和有效电平的验证结果，时钟分频功能则由 `clock_div` 的周期测量单独佐证。

## 本机源码与手册位置

以下相对路径均以本仓库根目录 `/Volumes/aigo_1t/GitRepo/hc32-rs-workspace/hc32-hal/` 为起点。

| 资料 | 位置 |
| --- | --- |
| HC32 HAL 实现 | `hc32-ddl-hal/src/`、`hc32-ll-hal/src/` |
| Embassy STM32 HAL 架构参考 | `/Volumes/aigo_1t/Github/stm32-rs/embassy/embassy-stm32/` |
| 厂商 SDK（只读） | `../xh_ddl_sdk/<系列及版本>/`、`../xh_ll_sdk/<系列及版本>/` |
| 厂商 DS、UM/RM | 对应 SDK 包的 `documents/`，例如 `../xh_ddl_sdk/HC32L110_DDL_Rev1.3.2/documents/` 和 `../xh_ll_sdk/HC32F460_DDL_Rev3.3.0/documents/`。数据手册通常以 `DS_` 开头，寄存器和操作时序见用户手册或参考手册（本机常以 `RM_` 开头）。 |
| HC32 数据与 PAC | `../hc32-ddl-data/`、`../hc32-ll-data/`、`../hc32-metapac/`。两侧的外设版本表均位于对应数据仓库的 `docs/peripheral-versions.csv`。 |

DS 和 UM/RM 平时阅读优先使用同名 `.md`（MinerU 转换产物，与 PDF 同目录，图片在 `<文件名>-image/`）。`documents/` 里若只有 PDF 没有对应 md，使用 mineru 技能转换：`extract` 精确模式输出 md 到 PDF 同目录，图片文件夹与 md 引用的改名后处理由该技能自动完成。

## 双 agent 编写与审阅

每项 HAL 驱动由两个独立 agent 完成，配套例程由审阅 agent 在审阅通过后编写。

1. **实现 agent** 可以查阅 Embassy STM32 HAL、对应 HC32 SDK、DS 和 UM/RM，以及本项目的数据与 PAC。参考 Embassy 的 API 和组织方式时，按 HC32 资料落实寄存器语义。实现 agent 编写驱动代码、运行验证，并登记发现的手册与 SDK 差异。
2. **审阅 agent** 在独立上下文中工作。交接时只提供目标型号、待审代码及对应 DS、UM/RM、SDK 文件路径。它只阅读这些厂商手册、SDK和hal代码，也可查看差异台账，逐项核对引脚、寄存器位、访问副作用、初始化顺序和中断处理等相关行为。STM32 实现、生成数据和实现 agent 的推理记录都不作为审阅输入。
3. 审阅意见给出代码位置和手册版本、章节或表号。手册没有说明的行为标为“手册未说明”。实现 agent 核对意见、更新代码和差异记录；涉及硬件行为的修订交回审阅 agent 复核。
4. 审阅通过后，如果编写的芯片已存在于examples中，由审阅 agent 按“Examples 放置”的约定在对应板级包中编写或扩展覆盖该驱动功能的例程，并同步更新[系列与外设支持表](docs/peripheral-support.md)。例程仅以厂商手册/SDK及SDK例程和定稿驱动代码为依据，不参考 STM32 实现；编写中发现驱动问题时按第 3 条流程处理。编写的芯片还未创建examples开发板就跳过。

## 手册与 SDK 差异

实现 agent 发现 DS 或 UM/RM 与 SDK 的定义或操作顺序不符时，应在同一变更中更新 [手册与 SDK 差异记录](docs/sdk-doc-discrepancies.md)。逐项列出双方证据、驱动采用的行为和验证状态。缺少定论时标记“待验证”，后续获得勘误或实测结果时更新原条目。

发现上游数据问题时，在对应数据仓库按外层工作区 AGENTS.md 的流程修正并重新生成。HC32F175 的支持状态以 hc32-data-tools 的 support_scope.py 为准。

## 验证

在本仓库根目录检查格式，并以一个具体型号交叉编译 DDL HAL。例如：

```sh
cargo fmt --all -- --check
cargo check -p hc32-ddl-hal --target thumbv6m-none-eabi --no-default-features --features hc32l110c6pa
cargo check -p hc32-ll-hal --target thumbv7em-none-eabihf
```

修改型号选择、封装表或 IP 后端时，逐个编译所有受影响的完整料号，并验证零型号和同时选择两个型号都有清楚的诊断。在已有的受影响板包目录执行 `cargo build --bins --release` 验证最终链接。新系列还没有板包时，用临时最小程序验证默认初始化和公开 GPIO API 的目标链接；支持表分别记录编译、链接和板测证据。
