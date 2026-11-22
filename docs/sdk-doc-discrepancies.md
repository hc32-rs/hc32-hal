# HC32 手册与 SDK 差异记录

每项差异使用“芯片系列 · 外设 · 差异主题”作标题，按以下字段登记。

| 字段 | 记录内容 |
| --- | --- |
| 适用范围 | DDL 或 LL、芯片型号、涉及的 IP 版本（如已知） |
| 手册依据 | DS、UM 或 RM 的文件名、修订版、章节或表号，以及手册描述的行为 |
| SDK 依据 | SDK 包及版本、文件路径、函数或宏名，以及代码实际执行的行为 |
| HAL 处理 | 受影响的驱动和代码位置、采用的行为、选择依据及影响 |
| 验证状态 | 待验证或已确认；记录勘误出处或实测板卡、方法、结果和日期 |

条目按芯片系列拆分到 `docs/sdk-doc-discrepancies/` 下的文件。新增条目时写入对应系列文件，并同步更新下表的条目数和条目清单。

| 系列 | 文件 | 条目数 |
| --- | --- | --- |
| HC32L110 | [hc32l110.md](sdk-doc-discrepancies/hc32l110.md) | 25 |
| HC32L13x | [hc32l13x.md](sdk-doc-discrepancies/hc32l13x.md) | 3 |
| HC32F420 | [hc32f420.md](sdk-doc-discrepancies/hc32f420.md) | 2 |

## HC32L110

- GPIO · SDK 设备头文件缺少 CTRL0 寄存器
- GPIO · SVD 将中断使能与清除寄存器标注为只读
- SYSCTRL · SVD 的 PERI_CLKEN 复位值与手册不符
- SYSCTRL · RM 解锁键“0x5A5”笔误
- SYSCTRL · XTH 晶振频率范围 DS 与 RM 不一致
- SYSCTRL · XTH/XTL 晶振 DRIVER 推荐编码 DS 与 RM 字段级描述不一致
- SYSCTRL · XTH 8MHz 晶振驱动档边界（审阅修订项）
- SYSCTRL · RM 要求 XTH 稳定后软件延时 10ms，SDK 未做
- SYSCTRL · RCL 频率档位的 TRIM 位 9 语义手册未说明
- SYSCTRL · SDK 的 SysTick CLK_SEL“RCH”枚举语义与 RM 不一致
- SYSCTRL · SDK 使能晶振时固定写 DRIVER=0xF，与手册推荐值不符
- SYSCTRL · 时钟故障检测位 CLOCK_FT_EN 手册位表缺描述
- SYSCTRL · XTL 外部输入频率范围 DS 与 RM 不一致
- WDT · WOV=8 档溢出时间标称值可疑
- CRC · FLAG 判定机制与校验值追加位序手册未说明
- BT · 外部计数输入采样上限两处描述不一致
- LPTIM · WT_FLAG 复位初值与置位时序手册未说明
- PCA · CMOD.CIDL 空闲模式语义手册内部矛盾
- ADT · 三角波周期系数手册未给出公式
- LPUART · ISR.TE 语义两处描述相反
- UART/LPUART · SDK 引脚宏附带上下拉而手册未要求
- LPUART · 波特率计算 SDK 无四舍五入且手册表与公式偶有偏差
- UART · UART0/1 波特率互联是否受 TOG_EN 门控手册未说明
- UART · FE 与 RI/TI 的并发关系手册未说明
- UART · RCL 38.4k/32.768k 档位的 TRIM 判别手册未提供

## HC32L13x

- GPIO · SDK _HidePinInit 行内注释与掩码不符
- SYSCTRL · RCH 为当前时钟源时直写 TRIM 的时序手册未说明
- SYSCTRL · SDK SystemInit 不等待 RCH STABLE

## HC32F420

- SYSCTRL · SystemInit 整字写 RCHTRIM/RCLTRIM 覆盖保留位
- GPIO · RM 自身 GPIO_PUPD 复位值两处不一致
