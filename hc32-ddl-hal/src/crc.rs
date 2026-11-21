//! 循环冗余校验（HC32L110，CRC `ddl_v1`，基址 0x4002_0900，RM 20 章）。
//!
//! 本外设只有两个寄存器（RM 20.5.1）：
//!
//! - `CRC_RESULT`（偏移 0x04）：位 15:0 `RESULT` 为 CRC 结果（读出计算值，
//!   写入 0xFFFF 初始化计算）；位 16 `FLAG` 为校验结果（只读）。
//! - `CRC_DATA`（偏移 0x80）：待运算数据入口，只写，单次写入可选 8/16/32 位。
//!
//! 算法固定为 CRC-16，生成多项式 x¹⁶ + x¹² + x⁵ + 1（即 0x1021），执行
//! 标准 ISO/IEC 13239（RM 20.2/20.3.2，DS CRC 小节同）；初值 0xFFFF 取自
//! RM 20.4.1 编码示例第一步。输入/输出是否反转、有无异或输出 RM 与 DS 均
//! 未说明（手册未说明）：按"不反转"解读即为软件定义 CRC-16/CCITT-FALSE，
//! 对 `"123456789"` 应得 0x29B1——该等价性属手册外推断，列为板测首验项，
//! 由 STK 例程验证。
//!
//! ## 两种工作模式（RM 20.3.1）
//!
//! - **编码模式**（RM 20.4.1）：[`Crc::init`] → 依次写入数据 →
//!   [`Crc::result`] 读结果；一步完成的封装见 [`Crc::compute_bytes`] 等。
//! - **校验模式**（RM 20.4.2）：[`Crc::init`] → 依次写入数据 → 追加待校验
//!   的 CRC 值 → 读 `RESULT.FLAG`；封装见 [`Crc::check_bytes`] 等。
//!
//! ## 写入位宽（RM 20.3.3）
//!
//! DATA 支持 8/16/32 位写入，须遵循"位宽一致、先低后高"：每次写入的数据
//! 用与其有效位宽相等的写操作，同一逻辑值拆分写入时低位在前。同一份字节
//! 序列的三种喂法结果相同（RM 20.3.3 示例）：
//!
//! - 8 位：`0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77`
//! - 16 位：`0x1100, 0x3322, 0x5544, 0x7766`
//! - 32 位：`0x33221100, 0x77665544`
//!
//! 即字节切片按序喂入与值数组逐值喂入等价（值数组按小端序列化成字节序
//! 列）。校验模式追加的 CRC 值按一次 16 位写入口输入。
//!
//! ## FLAG 判定机制（手册未说明，待板测）
//!
//! RM 20.5.2 只定义 FLAG 的 0/1 含义，比较机制与生效时刻未说明。候选机
//! 制：内部残差与固定常数比较（此时追加校验值的位序影响结果），或与最后
//! 一次写入值比较（一次 16 位写即交付完整值，位序无关）。本驱动的校验序
//! 列（数据 → 一次 16 位写追加校验值 → 读 FLAG）与 RM 20.3.1/20.4.2 的
//! 步序一致；板测若判定需改用其他追加方式，修订 `append_crc` 后交回审阅
//! 复核。

use crate::clock::{PeripheralGate, enable_peripheral};
use crate::pac;

/// CRC-16 驱动。
///
/// 单例外设，经 [`Crc::steal`] 取得；门控（PERI_CLKEN.CRC，RM 3.5.8 位 26）
/// 在构造时开启。PERI_RESET 位 26（RM 4.2.2，R/W）复位默认为 1（正常工作），
/// 本驱动不操作该位；若他处将其清 0，模块将处于复位状态，须重新置 1 方可
/// 使用。
pub struct Crc {
    _private: (),
}

impl Crc {
    /// 取得 CRC 驱动并开启其门控。
    ///
    /// # Safety
    ///
    /// 单例外设，调用方须保证全程序只有一份实例在用。
    pub unsafe fn steal() -> Self {
        enable_peripheral(PeripheralGate::Crc, true);
        Self { _private: () }
    }

    /// 初始化 CRC 计算（向 RESULT 写 0xFFFF，RM 20.4.1/20.4.2 Step1）。
    ///
    /// 每次新计算前必须调用；写入后内部计算值回到初值 0xFFFF。
    pub fn init(&mut self) {
        pac::CRC.result().write(|w| w.set_result(0xFFFF));
    }

    /// 读取当前 CRC 计算结果（RESULT.RESULT，RM 20.5.2）。
    ///
    /// 编码模式下即本次序列的 CRC 值。
    pub fn result(&self) -> u16 {
        pac::CRC.result().read().result()
    }

    // -----------------------------------------------------------------------
    // 数据写入（CRC_DATA @0x80，只写，RM 20.5.3）
    // -----------------------------------------------------------------------

    /// 按 8 位写入逐字节喂数据（RM 20.5.3 8bit 写入方式）。
    pub fn feed_bytes(&mut self, data: &[u8]) {
        // safety: 地址为 CRC_DATA 寄存器（0x4002_0900 + 0x80），按手册的
        // 字节写入方式操作；调用方持有单例，无并发访问。
        let ptr = pac::CRC.data().as_ptr() as *mut u8;
        for &byte in data {
            unsafe { ptr.write_volatile(byte) };
        }
    }

    /// 按 16 位写入逐半字喂数据（RM 20.5.3 16bit 写入方式）。
    pub fn feed_u16(&mut self, data: &[u16]) {
        // safety: 地址为 CRC_DATA 寄存器，半字对齐（基址 4 对齐）；调用方
        // 持有单例，无并发访问。
        let ptr = pac::CRC.data().as_ptr() as *mut u16;
        for &word in data {
            unsafe { ptr.write_volatile(word) };
        }
    }

    /// 按 32 位写入逐字喂数据（RM 20.5.3 32bit 写入方式）。
    pub fn feed_u32(&mut self, data: &[u32]) {
        // safety: 地址为 CRC_DATA 寄存器，字对齐；调用方持有单例，无并发
        // 访问。
        let ptr = pac::CRC.data().as_ptr() as *mut u32;
        for &word in data {
            unsafe { ptr.write_volatile(word) };
        }
    }

    // -----------------------------------------------------------------------
    // 编码模式（RM 20.4.1）
    // -----------------------------------------------------------------------

    /// 编码模式一步完成（8 位字节序列）：init → 喂数据 → 返回 CRC 值。
    ///
    /// 等价于依次调用 [`Crc::init`]、[`Crc::feed_bytes`]、[`Crc::result`]。
    /// 空切片返回初值 0xFFFF。
    pub fn compute_bytes(&mut self, data: &[u8]) -> u16 {
        self.init();
        self.feed_bytes(data);
        self.result()
    }

    /// 编码模式一步完成（16 位半字序列）。
    ///
    /// 与 [`Crc::compute_bytes`] 接受按小端序列化后的同一字节序列等价
    /// （RM 20.3.3）。
    pub fn compute_u16(&mut self, data: &[u16]) -> u16 {
        self.init();
        self.feed_u16(data);
        self.result()
    }

    /// 编码模式一步完成（32 位字序列）。
    ///
    /// 与 [`Crc::compute_bytes`] 接受按小端序列化后的同一字节序列等价
    /// （RM 20.3.3）。
    pub fn compute_u32(&mut self, data: &[u32]) -> u16 {
        self.init();
        self.feed_u32(data);
        self.result()
    }

    // -----------------------------------------------------------------------
    // 校验模式（RM 20.4.2）
    // -----------------------------------------------------------------------

    /// 校验模式一步完成（8 位字节序列）：init → 喂数据 → 追加 `crc` →
    /// 返回 FLAG。
    ///
    /// 校验输入为"原始数据 + CRC 校验值"（RM 20.3.1），追加的校验值按
    /// 一次 16 位写输入（位序说明见模块级 FLAG 一节）。返回 `true` 表示
    /// `RESULT.FLAG = 1`（校验正确）。
    pub fn check_bytes(&mut self, data: &[u8], crc: u16) -> bool {
        self.init();
        self.feed_bytes(data);
        self.append_crc(crc);
        self.check_flag()
    }

    /// 校验模式一步完成（16 位半字序列）。
    pub fn check_u16(&mut self, data: &[u16], crc: u16) -> bool {
        self.init();
        self.feed_u16(data);
        self.append_crc(crc);
        self.check_flag()
    }

    /// 校验模式一步完成（32 位字序列）。
    pub fn check_u32(&mut self, data: &[u32], crc: u16) -> bool {
        self.init();
        self.feed_u32(data);
        self.append_crc(crc);
        self.check_flag()
    }

    /// 追加待校验的 CRC 值（一次 16 位写，CRC_DATA @0x80）。
    fn append_crc(&mut self, crc: u16) {
        // safety: 地址为 CRC_DATA 寄存器，半字对齐；调用方持有单例，无
        // 并发访问。
        let ptr = pac::CRC.data().as_ptr() as *mut u16;
        unsafe { ptr.write_volatile(crc) };
    }

    /// 读 RESULT.FLAG（RM 20.5.2 位 16）。
    fn check_flag(&self) -> bool {
        pac::CRC.result().read().flag()
    }
}
