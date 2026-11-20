# hc32-hal

小华半导体 HC32 微控制器的 Rust HAL 工作区。两个 `no_std` crate 分别面向厂商 DDL 和 LL SDK 覆盖的芯片系列，各自维护版本和驱动。

| Crate | 数据来源 |
| --- | --- |
| [hc32-ddl-hal](hc32-ddl-hal/) | hc32-ddl-data |
| [hc32-ll-hal](hc32-ll-hal/) | hc32-ll-data |

当前为仓库骨架，尚未接入 PAC、芯片 feature 或外设驱动。在仓库根目录运行 `cargo check --workspace` 可检查两个 crate。开发约定见 [AGENTS.md](AGENTS.md)。

例程按 `examples/<ddl 或 ll>/<外设>/src/bin/<外设>_vN.rs` 组织，每个外设独立构建，各 `vN` 支持的 MCU 见对应 `<外设>.md`；例程日志输出推荐使用 SEGGER RTT。

各系列的驱动、例程和实测进度见[系列与外设支持表](docs/peripheral-support.md)。
