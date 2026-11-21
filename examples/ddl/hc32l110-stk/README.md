# HC32L110 STK 例程

本包选择 HC32L110C6PA（Cortex-M0+，32 KiB Flash，4 KiB SRAM）。在本目录运行 `cargo build --locked --release --bins`，将 `target/thumbv6m-none-eabi/release/` 下选定的同名 ELF 用支持该芯片的调试器烧录并复位。

| 程序 | 例程所需接线 | 预期现象 |
| --- | --- | --- |
| blinky | P23 接低电平点亮的 LED，外接 LED 时串联限流电阻 | 每 500 ms 翻转一次 |
| button_led | 同上，另接按下时将 P24 接地的按键，输入启用内部上拉 | 按下点亮，松开熄灭 |
| clock_div | 示波器与开发板共地，观察 P24、P31；运行时 P31 从 SWCLK 切为时钟输出 | 最初约 5 秒分别为 2 MHz、500 kHz，随后均为 1 MHz |

三个程序均已通过目标芯片的 debug 和 release 链接。上板前请按具体板卡核对接线，尤其是板载 LED 与按键的引脚。实测结果待记录。
