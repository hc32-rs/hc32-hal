# HC32L136 STK 例程

本包选择 HC32L136K8TA（Cortex-M0+，64 KiB Flash，8 KiB SRAM，LQFP64）。在本目录运行 `cargo build --locked --release --bins`，将 `target/thumbv6m-none-eabi/release/` 下选定的同名 ELF 用支持该芯片的调试器烧录并复位。

| 程序 | 例程所需接线 | 预期现象 |
| --- | --- | --- |
| blinky | PD05 接高电平点亮的 LED，外接 LED 时串联限流电阻 | 每 500 ms 翻转一次 |
| button_led | 同上，另接按下时将 PD04 接地的按键，输入启用内部上拉 | 按下点亮，松开熄灭 |
| clock_div | 示波器/逻辑分析仪接 PA01、PA02 与 GND；LED 同 blinky | PA01 输出 2 MHz 方波（RCH 4 MHz、HCLK/2），PA02 输出 500 kHz 方波（PCLK 再 /4），LED 约 1 Hz 闪烁 |
| pll_output | 同上 | PA01/PA02 均输出 48 MHz 方波（RCH 4 MHz ×12，HCLK/PCLK 不分频，无需外部晶振），LED 约 1 Hz 闪烁 |

板载 LED 与按键取自厂商 `board_stkhc32l13x.h`（`STK_LED_PORT`/`STK_LED_PIN` = PD05，`STK_USER_PORT`/`STK_USER_PIN` = PD04）：LED 高电平点亮（SDK `Gpio_SetIO` 注释为“LED点亮”），按键按下为低（SDK 例程以 `Gpio_GetInputIO == FALSE` 判断按下）。这两个引脚只有 LQFP64 封装引出，因此本包固定 `hc32l136k8ta`（48 引脚的 `hc32l136j8ta` 没有这两个脚）。

`hc32-ddl-hal` 在 HC32L13x 上已适配 GPIO 与 SYSCTRL 时钟（默认路径为复位 RCH 4 MHz + 出厂 TRIM 装载；`Config::clock` 可配置 RCH/XTH/RCL/XTL/PLL 五源切换、HCLK/PCLK 分频与 PA01/PA02 时钟波形输出，经独立审阅）。L136 的 `crc_ddl_v2_1`、`wdt_ddl_v1_1`、`clk_trim_ddl_v1_2` 与 HAL 已适配的 v1 后端版本不一致，`uart`/`adt`/`lptim`/`pca` 驱动仍门控在 HC32L110。`blinky` 用 `clock::clocks()` 断言默认 HCLK 为 4 MHz，同时覆盖 `init` 的默认时钟路径；`clock_div` 与 `pll_output` 分别覆盖时钟分频和 RCH→PLL 48 MHz 切换，频率断言见各例程源码。

四个程序均已通过目标芯片的 release 链接（`cargo build --locked --release --bins`）。板测状态：未板测；实测后按例程记录引脚、外设实例、IP 版本与观察到的行为。