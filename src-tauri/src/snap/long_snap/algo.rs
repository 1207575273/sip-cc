//! 长截图**拼接算法**抽象层：与平台采集、滚轮注入解耦，仅约定输入/输出与可替换接口。
//!
//! ## 输入（[`StitchInput`]）
//! - **frames**：等宽等高的连续帧（物理像素 `RgbaImage`），时间顺序与滚动方向一致（先截的在前）。
//! - **min_overlap / max_overlap_ssd**：算法参数，语义由具体实现定义（默认实现见 [`super::ssd::SsdOverlapStitcher`]）。
//!
//! ## 输出
//! - **成功**：单张 `RgbaImage`（纵向拼接结果）。
//! - **失败**：[`StitchError`]（调用方可 `.to_string()` 或匹配变体做 i18n）。
//!
//! ## 替换算法
//! - 实现 [`VerticalStitcher`]，在 [`crate::snap::long_snap::stitch_with`] 或应用入口处注入；
//! - Windows 采集路径默认使用 [`crate::snap::long_snap::stitch_default`]（见 `long_snap/mod.rs`），**仅改一处**即可换实现。

use std::fmt;

use image::RgbaImage;

/// 拼接算法的**输入**（与采集层、OS 无关）。
#[derive(Debug, Clone)]
pub struct StitchInput<'a> {
    /// 等尺寸、按时间排序的帧
    pub frames: &'a [RgbaImage],
    /// 重叠搜索下限（行数）；语义由具体算法解释
    pub min_overlap: u32,
    /// 重叠质量阈值上限；语义由具体算法解释（如 SSD）
    pub max_overlap_ssd: f64,
}

/// 拼接失败原因（便于替换实现时保持统一错误面）
#[derive(Debug, Clone)]
pub enum StitchError {
    /// 无帧
    EmptyFrames,
    /// 帧之间宽高不一致
    SizeMismatch,
    /// 单帧高度不足以搜索重叠
    RegionTooSmall { min_height: u32 },
    /// 无法对齐相邻帧
    AlignmentFailed { detail: String },
}

impl fmt::Display for StitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StitchError::EmptyFrames => write!(f, "没有截取到任何帧"),
            StitchError::SizeMismatch => write!(f, "长截图帧尺寸不一致"),
            StitchError::RegionTooSmall { min_height } => {
                write!(f, "选区高度过小，无法长截图（需大于 {} 行）", min_height)
            }
            StitchError::AlignmentFailed { detail } => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for StitchError {}

/// 纵向滚动长截图的**拼接算法**（可替换）。
///
/// 实现须为 [`Send`] + [`Sync`]，以便在采集线程中调用。
pub trait VerticalStitcher: Send + Sync {
    fn stitch(&self, input: StitchInput<'_>) -> Result<RgbaImage, StitchError>;
}
