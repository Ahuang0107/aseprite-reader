use crate::raw::{AsepriteColor, RawAsepriteCel};

#[derive(Debug, Clone)]
/// A single cel in a frame in a layer
pub struct AsepriteCel {
    pub(super) x: f64,
    pub(super) y: f64,
    #[allow(dead_code)]
    pub(super) opacity: u8,
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
