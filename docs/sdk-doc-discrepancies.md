# HC32 手册与 SDK 差异记录

每项差异使用“芯片系列 · 外设 · 差异主题”作标题，按以下字段登记。

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL 或 LL、芯片型号、涉及的 IP 版本（如已知） |
| 手册依据 | DS、UM 或 RM 的文件名、修订版、章节或表号，以及手册描述的行为 |
| SDK 依据 | SDK 包及版本、文件路径、函数或宏名，以及代码实际执行的行为 |
| HAL 处理 | 受影响的驱动和代码位置、采用的行为、选择依据及影响 |
| 验证状态 | 待验证或已确认；记录勘误出处或实测板卡、方法、结果和日期 |

## HC32L110 · GPIO · SDK 设备头文件缺少 CTRL0 寄存器

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号（B4PA/B6PA/B6YA/C4PA/C4UA/C6PA/C6UA），GPIO `ddl_v2` |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50.pdf`（本机暂无同名 md，章节号待核对：预期在 GPIO 寄存器描述章节列出 CTRL0 @偏移 0x300，字段 IESEL，控制端口中断在深度睡眠/活动与睡眠模式的生效范围）。HC32L110.svd 中 `CTRL0`（偏移 0x300，复位值 0x1）与该描述一致 |
| SDK 依据 | `xh_ddl_sdk/HC32L110_DDL_Rev1.3.2/mcu/common/hc32l110.h` 的 `M0P_GPIO_TypeDef` 从 CTRL1 起定义，无 CTRL0 成员；`driver/src/gpio.c` 的 `Gpio_SetCtrlExt` 也未实现 `GpioCtrlIESEL`（`driver/inc/gpio.h` 中的枚举存在但无对应实现） |
| HAL 处理 | `hc32-ddl-hal/src/gpio.rs` 本变更未涉及 CTRL0（端口中断功能未实现）。后续实现中断时以 RM/SVD 为准补 CTRL0 操作，不以设备头为准 |
| 验证状态 | 待验证。待 RM 的 GPIO 寄存器章节核对偏移、复位值与字段语义后更新本条 |

## HC32L110 · GPIO · SVD 将中断使能与清除寄存器标注为只读

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，GPIO `ddl_v2`；上游数据问题，经 hc32-ddl-data 影响生成的 PAC |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50.pdf`（章节号待核对：预期 PxHIE/PxLIE/PxRIE/PxFIE 与 PxICLR 应为可写寄存器——中断使能需软件写入，标志清除需软件写 PxICLR） |
| SDK 依据 | `xh_ddl_sdk/HC32L110_DDL_Rev1.3.2/mcu/common/hc32l110.h` 将 PxHIE/LIE/RIE/FIE/ICLR 声明为 `__IO`；`driver/src/gpio.c` 的 `Gpio_EnableIrq`/`Gpio_ClearIrq` 实际写入这些寄存器（ICLR 写 0 清除对应 STAT 标志，非写 1 清除）；示例 `example/gpio/gpio_irq` 亦直接写 PxICLR。而 SDK 自带 `mcu/GCC/HC32L110.svd` 将这些寄存器 access 标为 read-only，与设备头/驱动行为矛盾 |
| HAL 处理 | 本变更未使用这些寄存器（端口中断未实现），不阻塞。后续实现前需按外层工作区 AGENTS.md 流程在 hc32-ddl-data 修正 `data/registers/gpio_ddl_v2.yaml`（将 HIE/LIE/RIE/FIE/ICLR 改为 RW）并重新生成 PAC |
| 验证状态 | 待验证。以 RM 寄存器章节的访问属性为准；修复上游数据后更新本条 |

## HC32L110 · SYSCTRL · SVD 的 PERI_CLKEN 复位值与手册不符

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`；上游数据问题，影响 PAC 生成的复位值信息 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50.pdf` 3.5.8：PERI_CLKEN 偏移 0x020，复位值 **0xC080_0000**（bit 28 GPIO = 0，即复位后 GPIO 门控为关；Flash 位与个别保留位为 1） |
| SDK 依据 | `xh_ddl_sdk/HC32L110_DDL_Rev1.3.2/mcu/GCC/HC32L110.svd` 将 PERI_CLKEN resetValue 标注为 0x0，与手册不符（镜像于 `hc32-ddl-data/svd/HC32L110.svd`） |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` 的 `enable_gpio` 用读-改-写只动 bit 28，不依赖复位值假设，行为不受影响；门控"复位后为关"的结论两侧一致。后续按外层工作区 AGENTS.md 流程在 hc32-ddl-data 修正 sysctrl_ddl_v2_2 的 PERI_CLKEN 复位值并重新生成 PAC |
| 验证状态 | 已确认（审阅 agent 对 RM 3.5.8 原文核对，2026-11-21）；上游数据修复后更新本条 |

## HC32L110 · SYSCTRL · RM 解锁键“0x5A5”笔误

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2` |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50.pdf`（本机同名 md 行 1279/1295，3.5.1/3.5.2 注意栏）：“先对 SYSCTRL2 依次写入 **0x5A5**、0xA5A5”，键值少一位；同手册 3.5.3 SYSCTRL2 位描述与 3.2.2 Step3 及各切换示例均为 0x5A5A、0xA5A5 |
| SDK 依据 | `xh_ddl_sdk/HC32L110_DDL_Rev1.3.2/driver/src/clk.c` `ClkUnlock()`（120-124 行）：写 `0x5A5A; 0xA5A5`，且 3.2.7 示例代码（md 行 1114~1118）用 0X5A5A/0XA5A5 可正常解锁 |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` `unlock()` 采用 0x5A5A/0xA5A5（3.5.3 位描述与全部操作示例为准，注意栏视为 OCR/排版笔误） |
| 验证状态 | 待验证（上板解锁写 SYSCTRL0 后回读确认）；发现手册勘误时更新本条 |

## HC32L110 · SYSCTRL · XTH 晶振频率范围 DS 与 RM 不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，`XthConfig::crystal` 边界归属 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50`（md 行 931，3.1.4）：XTH 需外接 **4~32MHz** 高速晶振；`DS_HC32L110系列数据手册_Rev2.80`（md 行 226/684，时钟系统与 7.3.7 XTH 电气特性）：**8~32MHz** 晶体/陶瓷谐振器；RM 3.5.5 DRIVER[3:2] 分段为 4M~8M/8M~16M/16M~24M/24M~32M，最低段含 4M |
| SDK 依据 | `driver/inc/clk.h` `CLK_XTH_VAL` 默认 32MHz，`en_clk_xth_freq_t`（268-274 行）最低档 `SysctrlXthFreq4_8MHz` 含 4M |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` `XthConfig::crystal` 接受 4~32 MHz（RM 3.1.4 与 DRIVER 分段为准）；8M 晶振按 RM 分段落 4~8M/8~16M 边界，归 8~16M 段（DS 7.3.7 推荐 8M 编码 0b0010 即 DRIVER[3:2]=00，与归段差异见下条） |
| 验证状态 | 待验证（以 8M 晶振上板确认可起振；若 4~8M 段实际不含 4M 边界再修正） |

## HC32L110 · SYSCTRL · XTH/XTL 晶振 DRIVER 推荐编码 DS 与 RM 字段级描述不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，`XthConfig::crystal`/`XtlConfig::crystal` 默认值 |
| 手册依据 | `DS_HC32L110系列数据手册_Rev2.80` 7.3.7：XTH 整体推荐编码 **32/24M→0b1110、16M→0b1010、12M→0b0110、8M→0b0010**；XTL 推荐 **0b1010**（amp=10+driver=10）。`RM_HC32L110系列参考手册_Rev2.50` 3.5.5 XTH Driver[1:0] 字段级推荐“10=默认驱动能力(推荐值)”、3.5.7 XTL 字段级推荐“amp 10=较大振幅(推荐值)、driver 01=默认驱动能力(推荐值)”即整体 0b1001——**XTL 两侧手册矛盾**（DS 0b1010 vs RM 0b1001）；XTH 侧 DS 各频推荐值的 [1:0] 均为 10，与 RM 字段级推荐一致，无矛盾 |
| SDK 依据 | `driver/src/clk.c` `Clk_Enable`（723/735 行）使能晶振时固定写 `DRIVER = 0xF`（最大驱动），不按频率选择；`Clk_SetXTH_Driver`/`Clk_SetXTL_Driver` 提供按 DS 推荐编码的 API 但切换流程内不调用 |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs`：XTH 按 RM 3.5.5 分段（8M 归 4~8M 段、16M 归 16~24M 段、24M 归 24~32M 段，均与 DS 推荐编码一致）+ 默认驱动 10；XTL 默认取 DS 7.3.7 电气特性表整体推荐 0b1010（amp=Large、driver=Stronger），`XtlConfig::crystal` 文档注明取舍。SDK 的 0xF 写法不采用 |
| 验证状态 | 待验证（32.768k 晶振上板起振与功耗确认；若异常改用 RM 字段级推荐 0b1001 重试） |

## HC32L110 · SYSCTRL · XTH 8MHz 晶振驱动档边界（审阅修订项）

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，`XthConfig::crystal` 的 8MHz 边界 |
| 手册依据 | `DS_HC32L110系列数据手册_Rev2.80` 7.3.7 XTH 跨导表：“XTH_CR[3:0]=0b0010 (8MHz 推荐值)”——8M 晶振的推荐编码落在 Freq=00（4~8M）段；`RM_HC32L110系列参考手册_Rev2.50` 3.5.5 Driver[3:2] 分段“00:4M~8M 01:8M~16M”边界两侧均可读，未指定 8M 归属 |
| SDK 依据 | `driver/inc/clk.h` `en_clk_xth_freq_t`：`SysctrlXthFreq4_8MHz` 段名含 8M（含边界两端命名不一致的固有模糊） |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` `XthConfig::crystal` 初版按排除边界把 8M 归 8~16M 段（0b0110），与 DS 推荐编码及自身注释矛盾；**已按审阅意见改为 `..=8_000_000 => Mhz4_8`（0b0010）**。16M/24M 边界同理按 DS 推荐编码归属（16M→0b1010 归 16~24M 段、24M→0b1110 归 24~32M 段） |
| 验证状态 | 已确认（审阅 agent 对 DS 7.3.7 原文核对，2026-11-21）；8M 晶振上板验证后闭项 |

## HC32L110 · SYSCTRL · RM 要求 XTH 稳定后软件延时 10ms，SDK 未做

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，时钟切换流程 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.5.5 STABLE 字段注与 3.2.3 Step7：查询到 XTH 稳定标志后**需软件延时 10ms 以上**才可切换系统时钟 |
| SDK 依据 | `driver/src/clk.c` `Clk_SwitchTo`（198-255 行）：轮询 `Clk_GetClkRdy(ClkNew)` 置位后立即 `Clk_SetSource` 切换，无 10ms 延时 |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` `init` 在 XTH 稳定等待通过后调用 `delay_10ms()`（按当前 HCLK 估算循环，只长不短）再切换。XTL/RCL 无此要求（RM 3.2.2/3.2.5 示例只查 Stable），不加延时 |
| 验证状态 | 待验证（XTH 上板切换后 Flash 读写/通信稳定性观察） |

## HC32L110 · SYSCTRL · RCL 频率档位的 TRIM 位 9 语义手册未说明

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，RCL 档位解码 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.5.6 RCL_CR.TRIM：只说明“Flash 中保存了 2 组频率的校准值（38.4K @0x00100C20、32.768K @0x00100C22）”，未说明两组值如何区分、TRIM 位 9 是否为档位选择位 |
| SDK 依据 | `driver/src/clk.c` `Clk_GetHClkFreq`（279 行）：以 `RCL_CR_f.TRIM & 0x200`（位 9）索引 `{32768, 38400}` 表，位 9 = 0 为 32768、1 为 38400；`Clk_SetRCLFreq`（474 行）直接把 flash 校准值写入 TRIM（校准值自带该位） |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` 写档位时照 SDK 方式直接写入 flash 校准值（位 9 随校准值自带），`current_hclk_hz` 解码用位 9 判断 |
| 验证状态 | 待验证（写入 32.768K 校准值后回读 RCL_CR 与 CLKTRIM 实测频率比对） |

## HC32L110 · SYSCTRL · SDK 的 SysTick CLK_SEL“RCH”枚举语义与 RM 不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，SYSTICK_CR.CLK_SEL 编码 10 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.5.9 CLK_SEL：**10: 系统时钟 8 分频 SystemClk/8**；11: 外部高速时钟 XTH |
| SDK 依据 | `driver/inc/clk.h` 217-221 行注释：CLK_SEL=10 标注“RCH”，且 `Clk_SysTickConfig`（clk.c 845-858）`ClkRCH → 0x2`；同 SDK 的 `Clk_GetClkFreq` 类注释又称 RCH 档为“hclk/8” |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` `SysTickClock::SysclkDiv8` 按 RM 3.5.9 命名（系统时钟 8 分频，非 RCH） |
| 验证状态 | 待验证（上板配置后测 SysTick 实际频率） |

## HC32L110 · SYSCTRL · SDK 使能晶振时固定写 DRIVER=0xF，与手册推荐值不符

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，晶振使能序列 |
| 手册依据 | `DS_HC32L110系列数据手册_Rev2.80` 7.3.7 与 `RM_HC32L110系列参考手册_Rev2.50` 3.5.5/3.5.7：晶振驱动应按频率/特性选择推荐编码（XTH 16M 0b1010、8M 0b0010 等；XTL 0b1010），驱动越强功耗越大 |
| SDK 依据 | `driver/src/clk.c` `Clk_Enable`（723/735 行）：使能 XTL/XTH 时无条件 `DRIVER = 0xF`（最大驱动），随后切换流程并不调用 `Clk_SetXTL_Driver`/`Clk_SetXTH_Driver` 修正 |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` `init` 按配置写推荐编码（见“XTH/XTL 晶振 DRIVER 推荐编码”条），不采用 SDK 的 0xF |
| 验证状态 | 待验证（同“XTH/XTL 晶振 DRIVER 推荐编码”条） |

## HC32L110 · SYSCTRL · 时钟故障检测位 CLOCK_FT_EN 手册位表缺描述

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，SYSCTRL1 位 4 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.5.2 位表：位 4 标记 **Reserved**，无功能描述；另无其他章节描述时钟故障检测 |
| SDK 依据 | `driver/inc/clk.h` 151-162 行 `en_clk_func_t`：`ClkFuncFaultEn = 1<<4`，注释“时钟故障检测使能”；`driver/src/clk.c` `Clk_SetFunc` 据此写 SYSCTRL1 位 4 |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` `Config::clock_fault_detect` 提供该开关（以 SDK 定义为准，RM 位表缺失）；功能细节（失效后是否自动切回 RCH）两侧均未说明，标注“手册未说明”，暂不实现自动切换行为 |
| 验证状态 | 待验证（XTH 为系统时钟时手动停振观察行为——需板测） |

## HC32L110 · SYSCTRL · XTL 外部输入频率范围 DS 与 RM 不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，XTL 外部输入模式 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.1.3：“XTL 也可以不接晶振，直接从 P14 引脚输入 **32.768KHz** 的时钟信号”（限定单一频率）；`DS_HC32L110系列数据手册_Rev2.80` 7.3.7 外部输入低速时钟表：输入范围 **0~1000 kHz**（`Fxtl_ext`）——两侧不一致 |
| SDK 依据 | `driver/inc/clk.h` `CLK_XTL_VAL` 固定 32768 |
| HAL 处理 | `hc32-ddl-hal/src/clock.rs` 频率快照按 RM 3.1.3 硬编码 32.768 kHz（XTL 外部输入以 RM 限定为准）；非 32.768k 输入的快照误差由调用方自行避免 |
| 验证状态 | 待验证（若需 1kHz~1MHz 输入再按 DS 放宽并参数化频率） |


