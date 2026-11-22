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
| HAL 处理 | `hc32-ddl-hal/src/gpio/mod.rs` 本变更未涉及 CTRL0（端口中断功能未实现）。后续实现中断时以 RM/SVD 为准补 CTRL0 操作，不以设备头为准 |
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
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` 的 `enable_gpio` 用读-改-写只动 bit 28，不依赖复位值假设，行为不受影响；门控"复位后为关"的结论两侧一致。后续按外层工作区 AGENTS.md 流程在 hc32-ddl-data 修正 sysctrl_ddl_v2_2 的 PERI_CLKEN 复位值并重新生成 PAC |
| 验证状态 | 已确认（审阅 agent 对 RM 3.5.8 原文核对，2026-11-21）；上游数据修复后更新本条 |

## HC32L110 · SYSCTRL · RM 解锁键“0x5A5”笔误

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2` |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50.pdf`（本机同名 md 行 1279/1295，3.5.1/3.5.2 注意栏）：“先对 SYSCTRL2 依次写入 **0x5A5**、0xA5A5”，键值少一位；同手册 3.5.3 SYSCTRL2 位描述与 3.2.2 Step3 及各切换示例均为 0x5A5A、0xA5A5 |
| SDK 依据 | `xh_ddl_sdk/HC32L110_DDL_Rev1.3.2/driver/src/clk.c` `ClkUnlock()`（120-124 行）：写 `0x5A5A; 0xA5A5`，且 3.2.7 示例代码（md 行 1114~1118）用 0X5A5A/0XA5A5 可正常解锁 |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` `unlock()` 采用 0x5A5A/0xA5A5（3.5.3 位描述与全部操作示例为准，注意栏视为 OCR/排版笔误） |
| 验证状态 | 待验证（上板解锁写 SYSCTRL0 后回读确认）；发现手册勘误时更新本条 |

## HC32L110 · SYSCTRL · XTH 晶振频率范围 DS 与 RM 不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，`XthConfig::crystal` 边界归属 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50`（md 行 931，3.1.4）：XTH 需外接 **4~32MHz** 高速晶振；`DS_HC32L110系列数据手册_Rev2.80`（md 行 226/684，时钟系统与 7.3.7 XTH 电气特性）：**8~32MHz** 晶体/陶瓷谐振器；RM 3.5.5 DRIVER[3:2] 分段为 4M~8M/8M~16M/16M~24M/24M~32M，最低段含 4M |
| SDK 依据 | `driver/inc/clk.h` `CLK_XTH_VAL` 默认 32MHz，`en_clk_xth_freq_t`（268-274 行）最低档 `SysctrlXthFreq4_8MHz` 含 4M |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` `XthConfig::crystal` 接受 4~32 MHz（RM 3.1.4 与 DRIVER 分段为准）；8M 晶振按 RM 分段落 4~8M/8~16M 边界，归 8~16M 段（DS 7.3.7 推荐 8M 编码 0b0010 即 DRIVER[3:2]=00，与归段差异见下条） |
| 验证状态 | 待验证（以 8M 晶振上板确认可起振；若 4~8M 段实际不含 4M 边界再修正） |

## HC32L110 · SYSCTRL · XTH/XTL 晶振 DRIVER 推荐编码 DS 与 RM 字段级描述不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，`XthConfig::crystal`/`XtlConfig::crystal` 默认值 |
| 手册依据 | `DS_HC32L110系列数据手册_Rev2.80` 7.3.7：XTH 整体推荐编码 **32/24M→0b1110、16M→0b1010、12M→0b0110、8M→0b0010**；XTL 推荐 **0b1010**（amp=10+driver=10）。`RM_HC32L110系列参考手册_Rev2.50` 3.5.5 XTH Driver[1:0] 字段级推荐“10=默认驱动能力(推荐值)”、3.5.7 XTL 字段级推荐“amp 10=较大振幅(推荐值)、driver 01=默认驱动能力(推荐值)”即整体 0b1001——**XTL 两侧手册矛盾**（DS 0b1010 vs RM 0b1001）；XTH 侧 DS 各频推荐值的 [1:0] 均为 10，与 RM 字段级推荐一致，无矛盾 |
| SDK 依据 | `driver/src/clk.c` `Clk_Enable`（723/735 行）使能晶振时固定写 `DRIVER = 0xF`（最大驱动），不按频率选择；`Clk_SetXTH_Driver`/`Clk_SetXTL_Driver` 提供按 DS 推荐编码的 API 但切换流程内不调用 |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs`：XTH 按 RM 3.5.5 分段（8M 归 4~8M 段、16M 归 16~24M 段、24M 归 24~32M 段，均与 DS 推荐编码一致）+ 默认驱动 10；XTL 默认取 DS 7.3.7 电气特性表整体推荐 0b1010（amp=Large、driver=Stronger），`XtlConfig::crystal` 文档注明取舍。SDK 的 0xF 写法不采用 |
| 验证状态 | 待验证（32.768k 晶振上板起振与功耗确认；若异常改用 RM 字段级推荐 0b1001 重试） |

## HC32L110 · SYSCTRL · XTH 8MHz 晶振驱动档边界（审阅修订项）

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，`XthConfig::crystal` 的 8MHz 边界 |
| 手册依据 | `DS_HC32L110系列数据手册_Rev2.80` 7.3.7 XTH 跨导表：“XTH_CR[3:0]=0b0010 (8MHz 推荐值)”——8M 晶振的推荐编码落在 Freq=00（4~8M）段；`RM_HC32L110系列参考手册_Rev2.50` 3.5.5 Driver[3:2] 分段“00:4M~8M 01:8M~16M”边界两侧均可读，未指定 8M 归属 |
| SDK 依据 | `driver/inc/clk.h` `en_clk_xth_freq_t`：`SysctrlXthFreq4_8MHz` 段名含 8M（含边界两端命名不一致的固有模糊） |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` `XthConfig::crystal` 初版按排除边界把 8M 归 8~16M 段（0b0110），与 DS 推荐编码及自身注释矛盾；**已按审阅意见改为 `..=8_000_000 => Mhz4_8`（0b0010）**。16M/24M 边界同理按 DS 推荐编码归属（16M→0b1010 归 16~24M 段、24M→0b1110 归 24~32M 段） |
| 验证状态 | 已确认（审阅 agent 对 DS 7.3.7 原文核对，2026-11-21）；8M 晶振上板验证后闭项 |

## HC32L110 · SYSCTRL · RM 要求 XTH 稳定后软件延时 10ms，SDK 未做

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，时钟切换流程 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.5.5 STABLE 字段注与 3.2.3 Step7：查询到 XTH 稳定标志后**需软件延时 10ms 以上**才可切换系统时钟 |
| SDK 依据 | `driver/src/clk.c` `Clk_SwitchTo`（198-255 行）：轮询 `Clk_GetClkRdy(ClkNew)` 置位后立即 `Clk_SetSource` 切换，无 10ms 延时 |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` `init` 在 XTH 稳定等待通过后调用 `delay_10ms()`（按当前 HCLK 估算循环，只长不短）再切换。XTL/RCL 无此要求（RM 3.2.2/3.2.5 示例只查 Stable），不加延时 |
| 验证状态 | 待验证（XTH 上板切换后 Flash 读写/通信稳定性观察） |

## HC32L110 · SYSCTRL · RCL 频率档位的 TRIM 位 9 语义手册未说明

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，RCL 档位解码 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.5.6 RCL_CR.TRIM：只说明“Flash 中保存了 2 组频率的校准值（38.4K @0x00100C20、32.768K @0x00100C22）”，未说明两组值如何区分、TRIM 位 9 是否为档位选择位 |
| SDK 依据 | `driver/src/clk.c` `Clk_GetHClkFreq`（279 行）：以 `RCL_CR_f.TRIM & 0x200`（位 9）索引 `{32768, 38400}` 表，位 9 = 0 为 32768、1 为 38400；`Clk_SetRCLFreq`（474 行）直接把 flash 校准值写入 TRIM（校准值自带该位） |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` 写档位时照 SDK 方式直接写入 flash 校准值（位 9 随校准值自带），`current_hclk_hz` 解码用位 9 判断 |
| 验证状态 | 待验证（写入 32.768K 校准值后回读 RCL_CR 与 CLKTRIM 实测频率比对） |

## HC32L110 · SYSCTRL · SDK 的 SysTick CLK_SEL“RCH”枚举语义与 RM 不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，SYSTICK_CR.CLK_SEL 编码 10 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.5.9 CLK_SEL：**10: 系统时钟 8 分频 SystemClk/8**；11: 外部高速时钟 XTH |
| SDK 依据 | `driver/inc/clk.h` 217-221 行注释：CLK_SEL=10 标注“RCH”，且 `Clk_SysTickConfig`（clk.c 845-858）`ClkRCH → 0x2`；同 SDK 的 `Clk_GetClkFreq` 类注释又称 RCH 档为“hclk/8” |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` `SysTickClock::SysclkDiv8` 按 RM 3.5.9 命名（系统时钟 8 分频，非 RCH） |
| 验证状态 | 待验证（上板配置后测 SysTick 实际频率） |

## HC32L110 · SYSCTRL · SDK 使能晶振时固定写 DRIVER=0xF，与手册推荐值不符

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，晶振使能序列 |
| 手册依据 | `DS_HC32L110系列数据手册_Rev2.80` 7.3.7 与 `RM_HC32L110系列参考手册_Rev2.50` 3.5.5/3.5.7：晶振驱动应按频率/特性选择推荐编码（XTH 16M 0b1010、8M 0b0010 等；XTL 0b1010），驱动越强功耗越大 |
| SDK 依据 | `driver/src/clk.c` `Clk_Enable`（723/735 行）：使能 XTL/XTH 时无条件 `DRIVER = 0xF`（最大驱动），随后切换流程并不调用 `Clk_SetXTL_Driver`/`Clk_SetXTH_Driver` 修正 |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` `init` 按配置写推荐编码（见“XTH/XTL 晶振 DRIVER 推荐编码”条），不采用 SDK 的 0xF |
| 验证状态 | 待验证（同“XTH/XTL 晶振 DRIVER 推荐编码”条） |

## HC32L110 · SYSCTRL · 时钟故障检测位 CLOCK_FT_EN 手册位表缺描述

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，SYSCTRL1 位 4 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.5.2 位表：位 4 标记 **Reserved**，无功能描述；另无其他章节描述时钟故障检测 |
| SDK 依据 | `driver/inc/clk.h` 151-162 行 `en_clk_func_t`：`ClkFuncFaultEn = 1<<4`，注释“时钟故障检测使能”；`driver/src/clk.c` `Clk_SetFunc` 据此写 SYSCTRL1 位 4 |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` `Config::clock_fault_detect` 提供该开关（以 SDK 定义为准，RM 位表缺失）；功能细节（失效后是否自动切回 RCH）两侧均未说明，标注“手册未说明”，暂不实现自动切换行为 |
| 验证状态 | 待验证（XTH 为系统时钟时手动停振观察行为——需板测） |

## HC32L110 · SYSCTRL · XTL 外部输入频率范围 DS 与 RM 不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，SYSCTRL `ddl_v2_2`，XTL 外部输入模式 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.1.3：“XTL 也可以不接晶振，直接从 P14 引脚输入 **32.768KHz** 的时钟信号”（限定单一频率）；`DS_HC32L110系列数据手册_Rev2.80` 7.3.7 外部输入低速时钟表：输入范围 **0~1000 kHz**（`Fxtl_ext`）——两侧不一致 |
| SDK 依据 | `driver/inc/clk.h` `CLK_XTL_VAL` 固定 32768 |
| HAL 处理 | `hc32-ddl-hal/src/clock/mod.rs` 频率快照按 RM 3.1.3 硬编码 32.768 kHz（XTL 外部输入以 RM 限定为准）；非 32.768k 输入的快照误差由调用方自行避免 |
| 验证状态 | 待验证（若需 1kHz~1MHz 输入再按 DS 放宽并参数化频率） |



## HC32L110 · WDT · WOV=8 档溢出时间标称值可疑

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，WDT `ddl_v1`，`WDT_CON.WOV` 编码 8 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 14.3.2 WOV 字段表：编码 8 标称 **500ms**；其余 15 档（1.6ms~52.4s）全部符合 1.6ms × 2ⁿ 规律，按规律编码 8 应约 **410ms**（1.6 × 2⁸）；`DS_HC32L110系列数据手册_Rev2.80` 表 7-13 仅给出总范围 1.6~52000ms，无法仲裁 |
| SDK 依据 | 同代 `xh_ddl_sdk/HC32F003_DDL_Rev2.0.0/driver/inc/wdt.h`（`wdt_ddl_v1_1` 同族寄存器）：`WdtT500ms = 8u ///<500ms`，同样标 500ms；HC32L110 SDK `wdt.h` 无档位枚举（`u8LoadValue` 由用户填写），无独立证据 |
| HAL 处理 | `hc32-ddl-hal/src/wdt.rs` `Period::Ms500` 按手册文本标称 500ms 提供；若实测为 410ms，改枚举名与文档即可（编码不变） |
| 验证状态 | 待验证（板测：WOV=8 启动后不复位喂狗计时，或示波器量测中断模式周期） |

## HC32L110 · CRC · FLAG 判定机制与校验值追加位序手册未说明

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，CRC `ddl_v1` |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 20.5.2 仅定义 FLAG 位 0/1 含义（0 错误 1 正确），比较机制（内部残差与固定常数比较，或与末次写入值比较）、FLAG 生效时刻均未说明；20.3.3 位宽原则只给单一宽度示例，无"数据写入后接 16 位校验值写"的混宽序列示例；输入/输出是否反转、有无异或输出未说明（算法等价性推断：多项式 0x1021 + 初值 0xFFFF + 不反转 = CRC-16/CCITT-FALSE，`"123456789"` → 0x29B1，属手册外推断） |
| SDK 依据 | `driver/src/crc.c` `CRC16_Check8/16/32`：数据按 8/16/32 位写后，均以一次 16 位写追加校验值，随后立即读 FLAG，无等待；`CRC16_Get8/16/32` 与 RM 20.4.1 步序一致 |
| HAL 处理 | `hc32-ddl-hal/src/crc.rs` 校验序列与 RM 20.3.1/20.4.2 步序及 SDK 一致（数据 → 一次 16 位写追加 → 读 FLAG）；模块文档将算法位序等价性与 FLAG 机制标注"手册未说明，待板测"。STK 例程按板测矩阵验证：T1 位序（0x29B1 → 非反射位序成立）、T2 FLAG 机制（RESULT 残差值 + FLAG 行为）、T3 混宽一致性、T4 空序列初值 |
| 验证状态 | 待验证（hc32l110-stk 板测后更新本条与驱动文档；若判定需改 `append_crc` 追加方式，涉及硬件行为修订，交回审阅复核） |

## HC32L110 · BT · 外部计数输入采样上限两处描述不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，BT `ddl_v1`，计数模式（CR.CT = 1）外部输入频率上限 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 9.4.4 CR.CT 字段："外部输入时钟频率要低于 **1/2 采样时钟**"；同手册 9.2.1 计数功能正文："外部输入时钟频率不能超过系统的 Pclk 时钟"——手册内部矛盾（1/2 PCLK vs 1×PCLK） |
| SDK 依据 | `driver/inc/bt.h` 无频率上限说明；SDK 示例亦未标注 |
| HAL 处理 | `hc32-ddl-hal/src/bt.rs` `new_counter` 文档按更严格的 9.4.4 字段描述（低于采样时钟 1/2）标注，行为安全 |
| 验证状态 | 待验证（高频率外部脉冲上板计数比对；若实测允许接近 PCLK 再放宽文档） |

## HC32L110 · LPTIM · WT_FLAG 复位初值与置位时序手册未说明

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，LPTIM `ddl_v1`，首次写 ARR 前的写同步等待 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 10.4.2 ARR 字段："写 ARR 前需要读取 CR.WT_FLAG，当且尽当 WT_FLAG 为 1 时，写入时才能写入数据"（正文另将该标志写作"WT2_FLAG"，与 10.4.3 位表的"WT_FLAG"命名不一致，疑为同一标志的笔误）；10.4.3 CR 复位值 0x0000_0008 → 位 7 WT_FLAG 复位为 **0**；复位后硬件是否/何时自行将该位置 1 手册未说明 |
| SDK 依据 | `driver/src/lpt.c` `Lpt_ARRSet`：读 WT_FLAG，为 1 才写 ARR，否则直接返回 Error，无等待；SDK `Lpt_Init` 流程未显式处理复位初值 |
| HAL 处理 | `hc32-ddl-hal/src/lptim.rs` `set_reload` 等待 WT_FLAG = 1（带 1e6 次循环超时，超时 panic，防死循环）；若复位后 WT_FLAG 不自行置 1，首次配置将超时 panic——上板首验项 |
| 验证状态 | 待验证（上电后立即读 CR.WT_FLAG 确认初值与置位时序；若确不自置 1，改用 SDK 式试探写+回读方案并交回复核） |

## HC32L110 · PCA · CMOD.CIDL 空闲模式语义手册内部矛盾

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，PCA `ddl_v1`，空闲模式下时基计数行为 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 11.4.2 CMOD 位 7："CIDL 1: 休眠模式(sleep)下，PCA **停止**工作；0: 继续工作"；同手册 11.2.1："当 CMOD.CIDL 置'1'后，PCA 定时器/计数器可以**继续运行**在空闲模式下"——两处完全相反 |
| SDK 依据 | `driver/inc/pca.h` 的 `stc_pca_cmod_cfg_t` 注释仅照抄位表语义，无独立证据 |
| HAL 处理 | `hc32-ddl-hal/src/pca.rs` `set_idle_halt` 文档按 11.4.2 位描述（CIDL=1 停止）标注 |
| 验证状态 | 待验证（空闲模式实验：CIDL 置 1/0 各测一次 CNT 是否递增后定论，更新驱动文档） |

## HC32L110 · ADT · 三角波周期系数手册未给出公式

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，ADT `ddl_v1`，三角波模式周期换算 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 12.3.2 PERAR："计数周期值，设定每轮计数的计数周期值"，未给周期公式；12.2.1.1 图 12-3 三角波波形（md 转换为图片无法文本核验） |
| SDK 依据 | SDK `adt.h` 无周期换算 API |
| HAL 处理 | `hc32-ddl-hal/src/adt.rs` `period_to_perar`/`perar_to_period` 三角波按 **2×(PERAR + 1)** 节拍换算（峰+谷合计；锯齿波 (PERAR+1) 节拍为自然读法且与复位值 0xFFFF→65536 节拍自洽） |
| 验证状态 | 待验证（示波器测三角波周期核对 2×(PERAR+1)；若不符修 scale 并交回复核） |

## HC32L110 · LPUART · ISR.TE 语义两处描述相反

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，LPUART `ddl_v1`，发送缓存空标志 ISR.TE（位 3） |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 16.9.5 ISR.TE 字段："当该位值为 0 时，硬件自动屏蔽软件写 SBUF 操作"（TE=0 屏蔽写、TE=1 可写）；但 16.8.2 发送数据流程的正文表述与之相反（"TE=1 期间屏蔽写"），手册内部矛盾。复位值 0x0000_0008（TE=1）与字段名"发送缓存空"支持 16.9.5 的读法 |
| SDK 依据 | `driver/src/lpuart.c` `LPUart_SendData`（570–580 行）发送前清 TI 后直接写 SBUF 并忙等 TI，未轮询 TE；`LPUart_GetStatus(LPUartTxEmpty)` 判定的是 ISR.TI（524–526 行）而非 TE；SDK 无 TE 判定代码可佐证 |
| HAL 处理 | `hc32-ddl-hal/src/uart.rs` 阻塞发送按 SDK 序列（清 TI → 写 SBUF → 忙等 TI），不依赖 TE 判定，两读法下行为一致；文档按 16.9.5（TE=1 可写）标注 |
| 验证状态 | 待验证（发送中轮询 TE 观察翻转时点上板定论；驱动当前不依赖 TE，无功能风险） |

## HC32L110 · UART/LPUART · SDK 引脚宏附带上下拉而手册未要求

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，UART0/1 与 LPUART 的 TX/RX 引脚复用配置 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 表 6-2 与逐引脚 SCON 描述：复用仅需 PxADS=0、Pxy_SEL 编码、方向位，未要求任何上/下拉 |
| SDK 依据 | `driver/inc/gpio.h` 个别宏附带额外动作：`Gpio_SetFunc_UART0TX_P35` 额外置 P3PU.P35=1（上拉），`Gpio_SetFunc_UART1RX_P36` 额外置 P3PD.P36=1（下拉）；其余 UART/LPUART 宏（如 UART1TX_P23/UART1RX_P24/UART2RX_P25）均不带上下拉 |
| HAL 处理 | `hc32-ddl-hal/src/uart.rs` 经 `gpio::set_pin_af` 按 RM 只做 ADS/SEL/方向，不自动加上/下拉；`Config::tx_pull`/`rx_pull` 提供显式配置位（默认 None），需要 SDK 行为的用户可自行设置 |
| 验证状态 | 待验证（115200 回环收发正常即可视为无影响；波特率容差与引脚上下拉无关联） |

## HC32L110 · LPUART · 波特率计算 SDK 无四舍五入且手册表与公式偶有偏差

| 字段 | 记录内容 |
| | --- |
| 适用范围 | DDL，HC32L110 全型号，LPUART `ddl_v1`，Mode1/3 定时器装载值计算 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 16.4.3：`Baud = (DBAUD+1)·f_SCLK / (32·(65536−TM))`，未规定舍入方式；15.4.2.1 波特率表在部分频率档与公式重算值存在 ±1 LSB 级偏差（表值取整方式未说明） |
| SDK 依据 | `driver/src/uart.c` `Uart_SetBaudRate`（449 行）float 计算 +0.5 四舍五入；`driver/src/lpuart.c` `LPUart_SetBaudRate`（424 行）整数除法**无** +0.5（向零截断），同一 SDK 内两驱动舍入策略不一致 |
| HAL 处理 | `hc32-ddl-hal/src/uart.rs` `timer_load` 统一按四舍五入（`(num + baud·16)/(baud·32)`，整数实现）实现，UART/LPUART 一致；误差 ≤ 半个定时器节拍，远小于波特率容差 |
| 验证状态 | 待验证（回环例程以实际波特率误差 ≤3% 为通过标准） |

## HC32L110 · UART · UART0/1 波特率互联是否受 TOG_EN 门控手册未说明

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，UART `ddl_v1` + BT `ddl_v1`，Mode1/3 波特率供给 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 15.1/9.3.2："TIM0 的翻转输出 tog0 到内部模块 UART0，控制 UART0 的波特率"，未提及 TOG_EN；9.4.4 TOG_EN 字段描述"TOG 输出使能，0:TOG,TOGN 同时输出 0"按字面读连 TOG 本体也关断——tog0 内部互联取自 TOG_EN 门控之前还是之后，手册未说明；对照 16.1 注明确要求 LPUART "必须使能 TIMER2 的 Toggle 输出"，若 UART0/1 同样需要 TOG_EN=1，手册未在 15 章作同等说明（手册未说明） |
| SDK 依据 | `example/uart/uart_int/source/main.c` 165-170 行：`stcBtConfig` 只设 enMD/enCT，enTog 未初始化（局部变量初值 0），Bt_Init 后 UART1 正常收发——SDK 行为证明 UART0/1 波特率在 TOG_EN=0 下仍产生 |
| HAL 处理 | `hc32-ddl-hal/src/uart.rs` UART0/1 侧保持 `tog_en(false)`（复位值），LPUART 侧按 RM 16.1 注置 `tog_en(true)`；注释如实标注"手册未说明，按 SDK 例程行为" |
| 验证状态 | 待验证（上板确认 TOG_EN=0 下 UART0/1 可通信；若不可通信则改置 true 并交回复核） |

## HC32L110 · UART · FE 与 RI/TI 的并发关系手册未说明

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，UART/LPUART `ddl_v1`，帧错误的标志交互 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 15.5/16.5：FE 硬件置位、软件清零，未及时清零时"后续收到数据即使带有效 Stop 位，也不会把 FE 标志清 0"——仅描述 FE 自身；错误帧是否同时置 RI、FE=1 是否阻碍后续帧置 RI，手册未说明 |
| SDK 依据 | `driver/src/uart.c` `Uart_IRQHandler`（655-690 行）按 FE→RI→TI 顺序查标志、FE 置位时清 FE 后直接 return（暗示错误帧也可能置 RI，需先排掉）；`Uart_ReceiveData` 无 FE 检查 |
| HAL 处理 | `hc32-ddl-hal/src/uart.rs` 接收路径假定错误帧同时置 RI：忙等 RI → 查 FE → 丢弃并同清 FE/RI；若实测错误帧不置 RI，FE 帧将被滞留至下一有效帧才被发现，行为仍安全（不丢有效数据），但错误上报延迟一帧 |
| 验证状态 | 待验证（错误波特率注入错误帧，观察 RI/FE 置位时序后更新本条） |

## HC32L110 · UART · RCL 38.4k/32.768k 档位的 TRIM 判别手册未提供

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L110 全型号，LPUART `ddl_v1`，`Sclk::Rcl` 的频率取值 |
| 手册依据 | `RM_HC32L110系列参考手册_Rev2.50` 3.5.6 RCL_CR：TRIM（位 9:0）调整频率，Flash 0x00100C20/0x00100C22 起存两组校准值；**TRIM 值与标称频率的映射手册未说明**（两地址与 38.4k/32.768k 的对应关系为推断） |
| SDK 依据 | `driver/src/clk.c` `Clk_GetRClkFreq`：读 RCL_CR_f.TRIM 判 bit9，为 1 判 38.4k、为 0 判 32.768k；SDK 同样未给出文档依据 |
| HAL 处理 | `hc32-ddl-hal/src/uart.rs` `Sclk::frequency` 按 SDK 同款判别（bit9 区分），文档标注"未验证推断"；影响 LPMODE=0（已被拒绝的 SCLK≠PCLK 组合不受影响）与 `low_power_baudrate()` 返回值 |
| 验证状态 | 待验证（RCL 双档位实测波特率或示波器测 RCL_OUT 引脚频率后更新本条） |

## HC32L13x · GPIO · SDK _HidePinInit 行内注释与掩码不符

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L13x 六料号（L130 E8PA/F8UA/J8TA/J8UA、L136 J8TA/K8TA），GPIO `ddl_v3_3` |
| 手册依据 | `DS_HC32L13x系列数据手册_Rev2.20` 表 3-2（64/48/32/28 四列引脚功能说明）与 3.1 封装图：各封装未引出 IO 一览（见 `hc32-ddl-hal/src/peripherals/peripherals_l13x.rs` 模块文档的封装差异表） |
| SDK 依据 | `xh_ddl_sdk/HC32L13x_DDL_Rev2.2.0/mcu/common/system_hc32l13x.c` `_HidePinInit`：48PIN 分支 `PCPU |= 0x1FFF` 注释"PC00~PC12"（0x1FFF 实为 PC00~PC12 共 13 位一致，注释少列一位无碍）；`PDPU |= 0x00F4` 注释"PD02/PD04/PD06"（0x00F4 实为 PD02/PD04/PD05 三位，注释漏 PD05 多列 PD06）；32PIN 分支 `PDPU |= 0x00F4` 注释"PD02/04/PD06"（同为 PD02/PD04/PD05）。掩码本身与 DS 表 3-2 一致，行内注释笔误 |
| HAL 处理 | `hc32-ddl-hal/src/gpio/gpio_v3_3.rs` `pull_up_unbonded` 按 SDK **掩码**而非注释确定未引出脚集合（PD02/PD04/PD05 上拉，PD06 不动），注释如实标注出处与取舍 |
| 验证状态 | 已确认（掩码与 DS 表 3-2 逐位核对一致；上拉后的实际漏电流与工作稳定性待板测） |

## HC32L13x · SYSCTRL · RCH 为当前时钟源时直写 TRIM 的时序手册未说明

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L13x 六料号，SYSCTRL `ddl_v3_1`，默认初始化的 TRIM 装载 |
| 手册依据 | `RM_HC32L13x系列参考手册_Rev2.50` 3.1.1/3.2.2：更改 RCH 输出频率"需要按照特定的更改时序，详见系统时钟切换章节"，3.2.2 给出 RCH_EN=1 下逐级换频方案（4M→8M→16M→24M/22.12M，配 HCLK_PRS=7）；但**复位默认源即 RCH**（SYSCTRL0 复位值 0x0000_0001，RM 3.5.1），复位默认 RCH_CR=0x126（晶圆 TRIM）→ 装载出厂校准值（0x00100C08，4 MHz 档）属"同档校准替换"还是"换频"、是否必须走逐级序列，手册未说明 |
| SDK 依据 | `xh_ddl_sdk/HC32L13x_DDL_Rev2.2.0/mcu/common/system_hc32l13x.c` `SystemInit` 在 RCH_EN=1、CLKSW=RCH 的复位态直接写 `RCH_CR_f.TRIM = *(0x00100C08)`，无 HCLK_PRS=7 步进、无 STABLE 等待 |
| HAL 处理 | `hc32-ddl-hal/src/clock/sysctrl_v3_1.rs` `init_default` 沿用 SDK SystemInit 行为（直接写 TRIM 装载出厂校准值），注释标注手册未说明与依据；若后续实测装载失败或频率异常，改为"先关 RCH_EN→装载→重启→等 STABLE"序列 |
| 验证状态 | 待验证（上板后以时钟输出引脚或 SysTick 校验装载后频率为 4 MHz 档） |

## HC32L13x · SYSCTRL · SDK SystemInit 不等待 RCH STABLE

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32L13x 六料号，SYSCTRL `ddl_v3_1`，默认初始化装载后是否等待稳定 |
| 手册依据 | `RM_HC32L13x系列参考手册_Rev2.50` 3.1.1："内部高速时钟从启动到稳定仅需 4us"；3.5.4 RCH_CR.STABLE 为 RO 稳定标志。TRIM 直写后 STABLE 是否重新清零，手册未说明 |
| SDK 依据 | `xh_ddl_sdk/HC32L13x_DDL_Rev2.2.0/mcu/common/system_hc32l13x.c` `SystemInit` 写 TRIM 后不查询 STABLE、不延时，直接进入后续初始化 |
| HAL 处理 | `hc32-ddl-hal/src/clock/sysctrl_v3_1.rs` `init_default` 与 SDK 一致不等待（RCH 未关停、TRIM 同档替换，4 us 量级稳定远快于任何后续寄存器访问序列）；代码注释标注该取舍 |
| 验证状态 | 待验证（与上一条同场景板测：装载后立即读时钟相关外设无异常即确认） |

## HC32F420 · SYSCTRL · SystemInit 整字写 RCHTRIM/RCLTRIM 覆盖保留位

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32F420KATB，SYSCTRL `ddl_v4`，默认初始化的 TRIM 装载 |
| 手册依据 | `RM_HC32F420系列参考手册_Rev1.30` 3.5.10 RCHTRIM：位 8~0 为 TRIM，**位 31~9 为保留位，必须保持复位值**；3.5.11 RCLTRIM：位 9~0 为 TRIM，**位 31~10 为保留位，必须保持复位值**；3.5.10/3.5.11 另载明出厂校准值存于 flash 0x0100_11E4（RCH，24 MHz 档）与 0x0100_11E8（RCL，32.768 kHz 档），以 2 字节地址区间（0x0100_11E4~E5、0x0100_11E8~E9）给出、未使用明确位宽术语 |
| SDK 依据 | `xh_ddl_sdk/HC32F420_DDL_Rev1.3.1/mcu/common/system_hc32f420.c` `SystemInit`：`HC_SYSCTRL->RCLTRIM = *((uint32_t*)0x010011E8); HC_SYSCTRL->RCHTRIM = *((uint32_t*)0x010011E4);`——按 32 位整字读 flash 并整字覆盖写寄存器，若 flash 相邻内容非零则保留位一并被改写 |
| HAL 处理 | `hc32-ddl-hal/src/clock/sysctrl_v4.rs` `init_default` 按 16 位读 flash 校准值、经 PAC 字段访问器 `set_trim` 位域写入（RMW 保留位不动），写序沿用 SDK 的先 RCL 后 RCH；反汇编核对为字段宽 10/9 的 `bfi` 写入 |
| 验证状态 | 待验证（上板后核对装载后 RCHTRIM/RCLTRIM 整字仅 TRIM 位域异于复位值、RCHCLKD 为 4 MHz 档） |

## HC32F420 · GPIO · RM 自身 GPIO_PUPD 复位值两处不一致

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL，HC32F420KATB，GPIO `ddl_v3_1`，复位态上下拉（审阅发现，HAL 不依赖复位值） |
| 手册依据 | `RM_HC32F420系列参考手册_Rev1.30` 表 6-5（寄存器偏移与复位值）：GPIO_PUPD 复位值 **(PA)0x0000_6000、(PB)0x0800_0000**（PA13/PA14 上拉 PU13/PU14、PB11 下拉 PD11，与 RM 6.3.3 复位态描述自洽）；而 RM 6.4.6 章节头标注复位值 **(PA)0x0400_6000、(PB)0x0000_0000**——两处互相矛盾，6.4.6 头部疑为笔误 |
| SDK 依据 | SDK 无 PUPD 复位值定义（`mcu/common/hc32f420.h` 头文件不含复位值；复位值信息仅来自 RM）。`driver/src/gpio.c` `GPIO_Init` 按目标态整写 PUPD 字段，不读回复位值 |
| HAL 处理 | `hc32-ddl-hal/src/gpio/gpio_v3_1.rs` `set_pull` 每次显式单次 RMW 写入目标 PU/PD 位（互斥，另一方向清零），不依赖任何复位值；PC/PD 端口的复位上下拉态以表 6-5 版本为准 |
| 验证状态 | 已确认（表 6-5 与 6.3.3 逐位互证一致，采信表 6-5；6.4.6 头部数值未采信。上板可用调试器读复位后 PUPD 整字佐证） |
