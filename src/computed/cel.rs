use crate::raw::{AsepriteColor, RawAsepriteCel};

#[derive(Debug, Clone)]
/// A single cel in a frame in a layer
pub struct AsepriteCel {
    /// 表示相对于整个 sprite 左上角的位置
    pub x: f64,
    /// 表示相对于整个 sprite 左上角的位置
    pub y: f64,
    /// 表示单个 cel 的透明度
    #[allow(dead_code)]
    pub opacity: u8,
    /// 针对某一帧判断图层顺序时，需要比较 layer index + z-index 的结果
    /// 如果相同，再比较 z-index
    #[allow(dead_code)]
    pub z_index: i16,
    /// 实际存储的 cel 数据
    pub raw_cel: RawAsepriteCel,
    /// Cel Properties 中的 color
    pub color: AsepriteColor,
    /// Cel Properties 中的 user data
    pub user_data: String,
}

impl AsepriteCel {
    pub(super) fn new(x: f64, y: f64, opacity: u8, z_index: i16, raw_cel: RawAsepriteCel) -> Self {
        AsepriteCel {
            x,
            y,
            opacity,
            z_index,
            raw_cel,
            color: AsepriteColor::default(),
            user_data: String::new(),
        }
    }

    /// 获取给 cel 的 sprite 尺寸，如果是 linked cel 则返回空
    pub fn get_size(&self) -> Option<[u16; 2]> {
        match self.raw_cel {
            RawAsepriteCel::Raw { width, height, .. } => Some([width, height]),
            RawAsepriteCel::Linked { .. } => None,
            RawAsepriteCel::Compressed { width, height, .. } => Some([width, height]),
        }
    }
}
