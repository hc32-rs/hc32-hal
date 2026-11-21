# HC32L110 STK 例程

本包选择 HC32L110C6PA（Cortex-M0+，32 KiB Flash，4 KiB SRAM）。在本目录运行 `cargo build --locked --release --bins`，将 `target/thumbv6m-none-eabi/release/` 下选定的同名 ELF 用支持该芯片的调试器烧录并复位。

| 程序 | 例程所需接线 | 预期现象 |
| --- | --- | --- |
| blinky | P23 接低电平点亮的 LED，外接 LED 时串联限流电阻 | 每 500 ms 翻转一次 |
| button_led | 同上，另接按下时将 P24 接地的按键，输入启用内部上拉 | 按下点亮，松开熄灭 |
| clock_div | 示波器与开发板共地，观察 P24、P31；运行时 P31 从 SWCLK 切为时钟输出 | 最初约 5 秒分别为 2 MHz、500 kHz，随后均为 1 MHz |
| wdt_reset_loop | 同 blinky（P23 LED） | 约 14 s 一轮循环：启动 2 s 标记复位原因（WDT 复位常亮、上电熄灭）→ 10 s LED 1 Hz 闪烁（每 500 ms 喂狗，溢出档 1.64 s）→ 停止喂狗 LED 常亮，约 1.64 s 后 WDT 溢出复位芯片，重启再入标记段 |
| crc | 无需额外接线（使用板载 P23 LED） | 自检（T1~T4，见例程头注释）全部通过 LED 常亮；失败按"短闪 N 次 + 熄灭 900 ms"循环，N=1~5 依次为：位序不符、正确值校验未过、篡改校验未拒、三种位宽不一致、空序列初值异常 |
| timer_blinky | 同 blinky（P23 LED） | LED 以约 1 Hz 翻转：BT0 模式 2 每 500 ms 溢出（Div32、ARR = 3036），轮询 TF 清标志后翻转 |
| lptim_blink | 同 blinky（P23 LED） | LED 以约 1 Hz 翻转：LPTIM PCLK 时基 62 Hz 溢出（ARR = 1020）× 31 次软件分频。上电即闪 = 复位后 WT_FLAG 自行置 1（构造期写同步首验通过）；LED 常灭不闪 = set_reload 超时停机，回填差异台账 |
| pca_pwm | 同 blinky（P23 LED，复用功能 4 = PCA_CH0） | LED 呼吸灯：PCA CH0 8 位 PWM，约 3.9 kHz（时基 1 MHz / 256），亮占比 12.5%~87.5% 往返渐变，单程约 480 ms。注意 duty 数值与高电平占比反向（RM 11.4.7）：duty 越大高电平越少、LED 越亮 |
| adt_pwm | 同 blinky（P23 LED，复用功能 3 = TIM4_CHA） | LED 呼吸灯：ADT0 锯齿波 1 kHz PWM（PERAR = 3999），亮占比 10%~90%（比较值 400~3600）往返渐变，单程约 220 ms；运行中改占空比走 GCMCR 缓存，硬件在溢出点装载无毛刺 |
| uart1_echo | P35（UART1_TXD，SEL 1）接 USB 转串口适配器 RXD、P36（UART1_RXD，SEL 1）接适配器 TXD、共地 | 串口调试器 115200-8-N-1 发送任意字符，终端逐字节原样回显（RCH 22.12 MHz，实际波特率 115208.33，+0.01%）；不发送时无输出 |
| lpuart_echo | P26（LPUART_TXD，SEL 5）接适配器 RXD、P25（LPUART_RXD，SEL 5）接适配器 TXD、共地 | 同上：LPUART 普通模式（LPMODE=0）阻塞回环，实际波特率 115208.33；驱动构造占用 TIM2 并开 TOG_EN（RM 16.4.3 注要求），属正常行为 |

十一个程序均已通过目标芯片的 debug 和 release 链接。上板前请按具体板卡核对接线，尤其是板载 LED 与按键的引脚。uart1_echo 与 lpuart_echo 待板测：核对回显内容、字节完整性及长时间连续收发有无丢帧（接收侧仅 1 帧缓存，RM 15.8.1/16.8.1）。其余程序实测结果待记录。
