# hc32-hal

小华半导体 HC32 微控制器的 Rust HAL 工作区。两个 `no_std` crate 分别面向厂商 DDL 和 LL SDK 覆盖的芯片系列，各自维护版本和驱动。

| Crate | 数据来源 |
| --- | --- |
| [hc32-ddl-hal](hc32-ddl-hal/) | hc32-ddl-data |
| [hc32-ll-hal](hc32-ll-hal/) | hc32-ll-data |

当前为仓库骨架，尚未接入 PAC、芯片 feature 或外设驱动。在仓库根目录运行 `cargo check --workspace` 可检查两个 crate。开发约定见 [AGENTS.md](AGENTS.md)。

板级例程按 `examples/<ddl 或 ll>/<板名>/src/bin/<例程名>.rs` 组织，每块板独立构建。

各系列的驱动、例程和板测进度见[系列与外设支持表](docs/peripheral-support.md)。
