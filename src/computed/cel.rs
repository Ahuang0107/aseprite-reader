use crate::raw::{AsepriteColor, RawAsepriteCel};

#[derive(Debug, Clone)]
/// A single cel in a frame in a layer
pub struct AsepriteCel {
    pub(super) x: f64,
    pub(super) y: f64,
    #[allow(dead_code)]
    pub(super) opacity: u8,
    /// 针对某一帧判断图层顺序时，需要比较 layer index + z-index 的结果
    /// 如果相同，再比较 z-index
    #[allow(dead_code)]
    pub(super) z_index: i16,
    pub(super) raw_cel: RawAsepriteCel,
    pub(super) color: AsepriteColor,
    pub(super) user_data: String,
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
}
