# hc32-hal 开发约定

## 实现边界

- 本仓库是 Cargo workspace。`hc32-ddl-hal/` 和 `hc32-ll-hal/` 是独立版本、独立发布的 `no_std` crate。按芯片所属的 SDK 系列选择 crate，再按实际 IP 版本实现驱动。
- 时钟、复位、引脚复用、DMA 请求和中断绑定由各 crate 按硬件语义实现。比如核对 DMA 路由时，DDL 查 TRI_SEL，LL 查 EVT_SRC，两者的请求编号都不能直接当作物理通道号。
- 应用层有对应标准接口时，优先实现 embedded-hal 系列 trait。等两侧出现语义一致的实际实现后，再考虑提取共享代码。
- 新增芯片时，根据对应数据仓库的芯片和外设描述选择 PAC feature。按具体型号验证引脚、外设实例和中断；同名外设可能有多种布局。

## Examples 放置

板级例程统一放在仓库根目录的 `examples/<ddl 或 ll>/<板名>/`。每块板一个独立 Cargo 包，在自己的 Cargo.toml 中声明空的 `[workspace]`，通过 path 依赖对应 HAL 并选定具体芯片 feature。同一块板的例程各占一个 `src/bin/<例程名>.rs`，共享该板的 target、链接配置和依赖。板级 README 写明开发板、芯片完整型号、接线、运行命令及可观察结果。可复用的源码片段可以直接放在 HAL API 的 rustdoc 中。

驱动、板级例程或板测结果发生变化时，同步更新[系列与外设支持表](docs/peripheral-support.md)。标记已测试时链接到对应板卡的实测记录。

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

每项 HAL 驱动及配套例程由两个独立 agent 完成。

1. **实现 agent** 可以查阅 Embassy STM32 HAL、对应 HC32 SDK、DS 和 UM/RM，以及本项目的数据与 PAC。参考 Embassy 的 API 和组织方式时，按 HC32 资料落实寄存器语义。实现 agent 编写代码、运行验证，并登记发现的手册与 SDK 差异。
2. **审阅 agent** 在独立上下文中工作。交接时只提供目标型号、待审代码及对应 DS、UM/RM 文件路径。它只阅读这些厂商手册和本次写出的代码，逐项核对引脚、寄存器位、访问副作用、初始化顺序和中断处理等相关行为。SDK 源码或示例、STM32 实现、PAC/SVD、生成数据、差异台账和实现 agent 的推理记录都不作为审阅输入。
3. 审阅意见给出代码位置和手册版本、章节或表号。手册没有说明的行为标为“手册未说明”。实现 agent 核对意见、更新代码和差异记录；涉及硬件行为的修订交回审阅 agent 复核。

## 手册与 SDK 差异

实现 agent 发现 DS 或 UM/RM 与 SDK 的定义或操作顺序不符时，应在同一变更中更新 [手册与 SDK 差异记录](docs/sdk-doc-discrepancies.md)。逐项列出双方证据、驱动采用的行为和验证状态。缺少定论时标记“待验证”，后续获得勘误或实测结果时更新原条目。

发现上游数据问题时，在对应数据仓库按外层工作区 AGENTS.md 的流程修正并重新生成。HC32F175 的支持状态以 hc32-data-tools 的 support_scope.py 为准。

## 验证

在本仓库根目录运行：

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo check -p hc32-ddl-hal --target thumbv6m-none-eabi
cargo check -p hc32-ll-hal --target thumbv7em-none-eabihf
```

上述两个嵌入式目标用于检查初始骨架。引入芯片 feature 后，按受影响型号选择匹配的 Rust target 和单个芯片 feature，分别检查对应 crate。
