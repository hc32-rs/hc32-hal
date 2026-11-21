# hc32-hal

小华半导体 HC32 微控制器的 Rust HAL 工作区。两个 `no_std` crate 分别面向厂商 DDL 和 LL SDK 覆盖的芯片系列，各自维护版本和驱动。

| Crate | 数据来源 |
| --- | --- |
| [hc32-ddl-hal](hc32-ddl-hal/) | hc32-ddl-data |
| [hc32-ll-hal](hc32-ll-hal/) | hc32-ll-data |

已接入 hc32-metapac（path 依赖）。`hc32-ddl-hal` 已接入两个芯片系列（一次恰好启用一个 feature）：

- **HC32L110**（feature：`hc32l110b4pa`/`b6pa`/`b6ya`/`c4pa`/`c4ua`/`c6pa`/`c6ua`）：GPIO（`Flex`/`Input`/`Output`/`OutputOpenDrain`，embedded-hal 1.0）与时钟系统（`clock`：RCH/XTH/RCL/XTL 四源切换、HCLK/PCLK 分频、外设门控、频率快照；`clk_trim`：时钟校准与晶振失效监测；本系列无 PLL），不配置时钟树时运行于复位默认内部 RCH。板级例程包 [examples/ddl/hc32l110-stk/](examples/ddl/hc32l110-stk/)（HC32L110C6PA）已建立：`blinky`/`button_led`/`clock_div` 已通过目标链接，待板测。
- **HC32L130**（feature：`hc32l130e8pa`/`f8ua`/`j8ta`/`j8ua`）：默认时钟（复位默认 RCH 4 MHz + 装载 RCH/RCL 出厂 TRIM，可配置时钟树未接入）下的数字 GPIO（`Flex`/`Input`/`Output`/`OutputOpenDrain`，embedded-hal 1.0；输出置位/清零走 PxBSET/PxBCLR 位操作寄存器）。四个料号均通过单料号 `cargo check` 与临时 no_std 程序的完整目标链接（init → 输入采样 → 输出构造 → 置位/清零 → 锁存读回），尚无板级例程包，板测状态见[系列与外设支持表](docs/peripheral-support.md)。

检查方式：`cargo check -p hc32-ddl-hal --target thumbv6m-none-eabi --features <芯片feature>`，无 feature 时会得到启用指引的编译错误。开发约定见 [AGENTS.md](AGENTS.md)。

例程按 `examples/<ddl 或 ll>/<开发板名>/src/bin/<例程名>.rs` 组织，每块开发板一个独立 Cargo 包（空 `[workspace]`，path 依赖 HAL 并固定板上芯片 feature）；例程日志输出推荐使用 SEGGER RTT。

各系列的驱动、例程和实测进度见[系列与外设支持表](docs/peripheral-support.md)。
