use std::{collections::BTreeMap, path::Path};

use image::{Pixel, Rgba, RgbaImage};
use tracing::error;

use cel::*;
use layer::*;
pub use palette::*;
use tag::*;

use crate::{
    error::{AsepriteError, AsepriteInvalidError, AseResult},
    raw::{
        AsepriteColor, AsepriteColorDepth, AsepritePixel, RawAseprite, RawAsepriteCel,
        RawAsepriteChunk,
    },
};
use crate::raw::RawAsepriteChunkType;

mod cel;
mod layer;
mod palette;
mod tag;
#[cfg(test)]
#[allow(deprecated)]
mod test;

#[derive(Debug, Clone)]
/// Data structure representing an Aseprite file
pub struct Aseprite {
    dimensions: (u16, u16),
    tags: BTreeMap<usize, AsepriteTag>,
    layers: BTreeMap<usize, AsepriteLayer>,
    cels: BTreeMap<usize, BTreeMap<usize, AsepriteCel>>,
    frame_count: usize,
    palette: Option<AsepritePalette>,
    transparent_palette: Option<u8>,
    frame_infos: Vec<AsepriteFrameInfo>,
}

impl Aseprite {
    /// Get the [`AsepriteTag`]s defined in this Aseprite
    pub fn tags(&self) -> impl Iterator<Item = &AsepriteTag> {
        self.tags.values()
    }

    /// Get the associated [`AsepriteLayer`]s defined in this Aseprite
    pub fn layers(&self) -> impl Iterator<Item = &AsepriteLayer> {
        self.layers.values()
    }

    /// Get the frames inside this aseprite
    pub fn get_frame(&self, frame_index: usize) -> Option<AsepriteFrame> {
        if frame_index >= self.frame_count {
            return None;
        }
        Some(AsepriteFrame {
            aseprite: self,
            frame_index,
        })
    }

    /// Get the cel of giving layer and frame
    pub fn get_cel(&self, layer_index: &usize, frame_index: &usize) -> Option<&AsepriteCel> {
        let Some(layer_cels) = self.cels.get(layer_index) else {
            return None;
        };
        let Some(cel) = layer_cels.get(frame_index) else {
            return None;
        };
        Some(cel)
    }

    /// Get a layer by its name
    ///
    /// If you have its id, prefer fetching it using [`get_by_id`]
    pub fn get_layer_by_name<N: AsRef<str>>(&self, name: N) -> Option<&AsepriteLayer> {
        let name = name.as_ref();
        self.layers
            .iter()
            .find(|(_, layer)| layer.name() == name)
            .map(|(_, layer)| layer)
    }

    /// Get a layer by its index
    pub fn get_layer_by_index(&self, index: &usize) -> Option<&AsepriteLayer> {
        self.layers.get(index)
    }

    /// 找到提供的 index 的 layer 属于的所有 groups
    pub fn find_layer_belong_groups(&self, index: usize) -> Vec<usize> {
        let Some(layer) = self.layers.get(&index) else {
            return Vec::new();
        };
        let mut cur_child_level = layer.child_level();
        let mut cur_index = index;
        let mut result = Vec::new();
        'find_all_group: while cur_child_level > 0 {
            cur_child_level -= 1;
            loop {
                cur_index -= 1;
                if let Some(layer) = self.layers.get(&cur_index) {
                    match layer {
                        AsepriteLayer::Group(GroupLayer {
                            index, child_level, ..
                                             }) => {
                            if *child_level == cur_child_level {
                                cur_index = *index;
                                result.push(*index);
                                continue 'find_all_group;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        result
    }

    /// 根据传入的像素数据生成对应图像，其中如果传入 cel 则生成完整尺寸的图像，否则生成当前 sprite trim 后的图像
    fn write_image(
        &self,
        cel: Option<&AsepriteCel>,
        width: u16,
        height: u16,
        pixels: &[AsepritePixel],
    ) -> AseResult<RgbaImage> {
        let mut image = RgbaImage::new(width as u32, height as u32);
        for x in 0..width {
            for y in 0..height {
                let mut pix_x = x as i16;
                let mut pix_y = y as i16;
                if let Some(cel) = &cel {
                    pix_x += cel.x as i16;
                    pix_y += cel.y as i16
                }

                if pix_x < 0 || pix_y < 0 {
                    continue;
                }
                let raw_pixel = &pixels[(x + y * width) as usize];
                let pixel =
                    Rgba(raw_pixel.get_rgba(self.palette.as_ref(), self.transparent_palette)?);

                image
                    .get_pixel_mut(pix_x as u32, pix_y as u32)
                    .blend(&pixel);
            }
        }
        Ok(image)
    }

    /// Get images of each layer in this frame
    ///
    /// The key of return map is layer id
    pub fn get_image_by_layer_frame(
        &self,
        layer_index: &usize,
        frame_index: &usize,
    ) -> AseResult<Option<RgbaImage>> {
        let Some(cel) = self.get_cel(layer_index, frame_index) else {
            return Ok(None);
        };
        match &cel.raw_cel {
            RawAsepriteCel::Raw {
                width,
                height,
                pixels,
            }
            | RawAsepriteCel::Compressed {
                width,
                height,
                pixels,
            } => Ok(Some(self.write_image(None, *width, *height, pixels)?)),
            RawAsepriteCel::Linked { frame_position } => {
                let frame_index = (*frame_position as usize) - 1;
                let Some(linked_cel) = self.get_cel(layer_index, &frame_index) else {
                    return Ok(None);
                };
                match &linked_cel.raw_cel {
                    RawAsepriteCel::Raw {
                        width,
                        height,
                        pixels,
                    }
                    | RawAsepriteCel::Compressed {
                        width,
                        height,
                        pixels,
                    } => Ok(Some(self.write_image(None, *width, *height, pixels)?)),
                    RawAsepriteCel::Linked { frame_position } => {
                        error!("Tried to draw a linked cel twice! This should not happen, linked cel should not link to a linked cel.");
                        Err(AsepriteError::InvalidConfiguration(
                            AsepriteInvalidError::InvalidFrame(*frame_position as usize),
                        ))
                    }
                }
            }
        }
    }
}

impl Aseprite {
    /// Construct a [`Aseprite`] from a [`RawAseprite`]
    pub fn from_raw(raw: RawAseprite) -> AseResult<Self> {
        let mut tags = BTreeMap::new();
        let mut layers = BTreeMap::new();
        let mut cels = BTreeMap::new();
        let mut palette = None;
        let mut frame_infos = vec![];

        let frame_count = raw.frames.len();

        // 记录上一个处理过的 chunk 类型，处理 user data 时需要知道他跟随在哪个 chunk 后面
        let mut last_chunk_type = RawAsepriteChunkType::ColorProfile;

        for frame in raw.frames {
            frame_infos.push(AsepriteFrameInfo {
                delay_ms: frame.duration_ms as usize,
            });

            for chunk in frame.chunks {
                match chunk {
                    RawAsepriteChunk::Layer {
                        flags,
                        layer_type,
                        layer_child,
                        width: _,
                        height: _,
                        blend_mode,
                        opacity,
                        name,
                    } => {
                        let layer_index = layers.len();
                        let layer = AsepriteLayer::new(
                            layer_index,
                            name,
                            layer_type,
                            flags & 0x1 != 0,
                            blend_mode,
                            if raw.header.flags & 0x1 != 0 {
                                Some(opacity)
                            } else {
                                None
                            },
                            layer_child,
                        );
                        layers.insert(layer_index, layer);
                        last_chunk_type = RawAsepriteChunkType::Layer;
                    }
                    RawAsepriteChunk::Cel {
                        layer_index,
                        x,
                        y,
                        opacity,
                        cel,
                    } => {
                        let layer_cels =
                            cels.entry(layer_index as usize).or_insert(BTreeMap::new());

                        let frame_index = layer_cels.len();
                        layer_cels.insert(
                            frame_index,
                            AsepriteCel::new(x as f64, y as f64, opacity, cel),
                        );

                        last_chunk_type =
                            RawAsepriteChunkType::Cel(layer_index as usize, frame_index);
                    }
                    RawAsepriteChunk::Tags { tags: raw_tags } => {
                        let start_index = tags.len();
                        let mut cur_index = start_index;
                        for raw_tag in raw_tags {
                            tags.insert(
                                cur_index,
                                AsepriteTag {
                                    index: cur_index,
                                    frames: raw_tag.from..raw_tag.to,
                                    animation_direction: raw_tag.anim_direction,
                                    name: raw_tag.name,
                                    color: AsepriteColor::default(),
                                    user_data: String::new(),
                                },
                            );
                            cur_index += 1;
                        }
                        last_chunk_type = RawAsepriteChunkType::Tags(start_index);
                    }
                    RawAsepriteChunk::Palette {
                        palette_size,
                        from_color,
                        to_color: _,
                        entries,
                    } => {
                        palette =
                            Some(AsepritePalette::from_raw(palette_size, from_color, entries));
                        last_chunk_type = RawAsepriteChunkType::Palette;
                    }
                    RawAsepriteChunk::UserData { data } => {
                        match &mut last_chunk_type {
                            RawAsepriteChunkType::Layer => {
                                let id = layers.len() - 1;
                                let layer = layers.get_mut(&id).unwrap();
                                layer.apply_raw_user_data(data);
                            }
                            // [Aseprite File Specs](https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md)
                            // After a Tags chunk, there will be several user data chunks, one for each tag,
                            // you should associate the user data in the same order as the tags are in the Tags chunk.
                            RawAsepriteChunkType::Tags(cur_index) => {
                                let tag = tags.get_mut(&cur_index).unwrap();
                                tag.apply_raw_user_data(data);
                                *cur_index += 1;
                            }
                            RawAsepriteChunkType::Cel(layer_index, frame_index) => {
                                let layer_cels = cels
                                    .get_mut(layer_index)
                                    .ok_or(AsepriteInvalidError::InvalidLayer(*layer_index))?;
                                let cel = layer_cels
                                    .get_mut(frame_index)
                                    .ok_or(AsepriteInvalidError::InvalidFrame(*frame_index))?;
                                cel.color = data.color;
                                cel.user_data = data.text;
                            }
                            _ => {}
                        }
                    }
                    RawAsepriteChunk::Slice { .. } => {
                        todo!("Not yet implemented slice")
                    }
                    RawAsepriteChunk::CelExtra { .. } => {
                        todo!("Not yet implemented cel extra")
                    }
                    RawAsepriteChunk::ColorProfile { .. } => {
                        error!("Not yet implemented color profile")
                        // todo!("Not yet implemented color profile")
                    }
                }
            }
        }

        Ok(Aseprite {
            dimensions: (raw.header.width, raw.header.height),
            transparent_palette: if raw.header.color_depth == AsepriteColorDepth::Indexed {
                Some(raw.header.transparent_palette)
            } else {
                None
            },
            tags,
            layers,
            cels,
            frame_count,
            palette,
            frame_infos,
        })
    }

    /// Construct a [`Aseprite`] from a [`Path`]
    pub fn from_path<S: AsRef<Path>>(path: S) -> AseResult<Self> {
        let buffer = std::fs::read(path)?;

        let raw_aseprite = crate::raw::read_aseprite(&buffer)?;

        Ok(Self::from_raw(raw_aseprite)?)
    }

    /// Construct a [`Aseprite`] from a `&[u8]`
    pub fn from_bytes<S: AsRef<[u8]>>(buffer: S) -> AseResult<Self> {
        let raw_aseprite = crate::raw::read_aseprite(buffer.as_ref())?;

        Ok(Self::from_raw(raw_aseprite)?)
    }
}

/// Information about a single animation frame
#[derive(Debug, Clone)]
pub struct AsepriteFrameInfo {
    /// The delay of this frame in milliseconds
    pub delay_ms: usize,
}

/// Single frame in an aseprite
/// TODO 目前看没必要存在这个结构，有空都统一到 Aseprite 上
pub struct AsepriteFrame<'a> {
    aseprite: &'a Aseprite,
    frame_index: usize,
}

impl<'a> AsepriteFrame<'a> {
    /// Get the timings
    pub fn get_infos(&self) -> AseResult<&AsepriteFrameInfo> {
        Ok(&self.aseprite.frame_infos[self.frame_index])
    }

    /// Get images of each layer in this frame
    ///
    /// The key of return map is layer id
    pub fn get_image_by_layer(&self, layer_index: &usize) -> AseResult<Option<RgbaImage>> {
        self.aseprite
            .get_image_by_layer_frame(layer_index, &self.frame_index)
    }
}

/// 这个方法是获取某一帧所有可见图层合并后的图片
///
/// TODO 没有处理透明度和混合模式的效果
#[allow(unused)]
fn image_for_frame(aseprite: &Aseprite, frame_index: u16) -> AseResult<RgbaImage> {
    let dim = aseprite.dimensions;
    let frame_index = frame_index as usize;
    let mut image = RgbaImage::new(dim.0 as u32, dim.1 as u32);
    for (layer_index, layer) in &aseprite.layers {
        if !layer.is_visible() {
            continue;
        }

        let Some(cel) = aseprite.get_cel(layer_index, &frame_index) else {
            continue;
        };

        let mut write_to_image = |cel: &AsepriteCel,
                                  width: u16,
                                  height: u16,
                                  pixels: &[AsepritePixel]|
         -> AseResult<()> {
            for x in 0..width {
                for y in 0..height {
                    let pix_x = cel.x as i16 + x as i16;
                    let pix_y = cel.y as i16 + y as i16;

                    if pix_x < 0 || pix_y < 0 {
                        continue;
                    }
                    let raw_pixel = &pixels[(x + y * width) as usize];
                    let pixel = Rgba(
                        raw_pixel
                            .get_rgba(aseprite.palette.as_ref(), aseprite.transparent_palette)?,
                    );

                    image
                        .get_pixel_mut(pix_x as u32, pix_y as u32)
                        .blend(&pixel);
                }
            }
            Ok(())
        };

        match &cel.raw_cel {
            RawAsepriteCel::Raw {
                width,
                height,
                pixels,
            }
            | RawAsepriteCel::Compressed {
                width,
                height,
                pixels,
            } => {
                write_to_image(&cel, *width, *height, pixels)?;
            }
            RawAsepriteCel::Linked { frame_position } => {
                let frame_index = *frame_position as usize - 1;

                let Some(linked_cel) = &aseprite.get_cel(layer_index, &frame_index) else {
                    return Err(AsepriteError::InvalidConfiguration(
                        AsepriteInvalidError::InvalidFrame(*frame_position as usize),
                    ));
                };

                match &linked_cel.raw_cel {
                    RawAsepriteCel::Raw {
                        width,
                        height,
                        pixels,
                    }
                    | RawAsepriteCel::Compressed {
                        width,
                        height,
                        pixels,
                    } => {
                        write_to_image(&cel, *width, *height, pixels)?;
                    }
                    RawAsepriteCel::Linked { frame_position } => {
                        error!("Tried to draw a linked cel twice!");
                        return Err(AsepriteError::InvalidConfiguration(
                            AsepriteInvalidError::InvalidFrame(*frame_position as usize),
                        ));
                    }
                }
            }
        }
    }

    Ok(image)
}
