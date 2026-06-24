use image::{GrayImage, Luma, Rgb, RgbImage, Rgba, RgbaImage, imageops};
use newtype::NewType;
use std::cmp::max;

const DEFAULT_TEXTURE_DIM: u32 = 64;

pub fn max_dimensions(dims: &[(u32, u32)]) -> (u32, u32) {
    if dims.is_empty() {
        panic!("Empty image array passed to max_dimensions()");
    }
    let mut max_height = 0;
    let mut max_width = 0;
    for dim in dims.iter() {
        max_width = max(dim.0, max_width);
        max_height = max(dim.1, max_height);
    }

    (max_width, max_height)
}

pub fn swizzle_4(images: (&GrayImage, &GrayImage, &GrayImage, &GrayImage)) -> RgbaImage {
    let max_dims = max_dimensions(&[
        images.0.dimensions(),
        images.1.dimensions(),
        images.2.dimensions(),
        images.3.dimensions(),
    ]);

    RgbaImage::from_fn(max_dims.0, max_dims.1, |x, y| {
        let u = x as f32 / max_dims.0 as f32;
        let v = y as f32 / max_dims.1 as f32;

        Rgba([
            imageops::sample_nearest(images.0, u, v)
                .expect("uv in bounds")
                .0[0],
            imageops::sample_nearest(images.1, u, v)
                .expect("uv in bounds")
                .0[0],
            imageops::sample_nearest(images.2, u, v)
                .expect("uv in bounds")
                .0[0],
            imageops::sample_nearest(images.3, u, v)
                .expect("uv in bounds")
                .0[0],
        ])
    })
}

pub fn _swizzle_3(images: (&GrayImage, &GrayImage, &GrayImage)) -> RgbImage {
    let max_dims = max_dimensions(&[
        images.0.dimensions(),
        images.1.dimensions(),
        images.2.dimensions(),
    ]);

    RgbImage::from_fn(max_dims.0, max_dims.1, |x, y| {
        let u = x as f32 / max_dims.0 as f32;
        let v = y as f32 / max_dims.1 as f32;

        Rgb([
            imageops::sample_nearest(images.0, u, v)
                .expect("uv in bounds")
                .0[0],
            imageops::sample_nearest(images.1, u, v)
                .expect("uv in bounds")
                .0[0],
            imageops::sample_nearest(images.2, u, v)
                .expect("uv in bounds")
                .0[0],
        ])
    })
}

#[derive(NewType)]
pub struct NormalMap(pub RgbImage);

impl NormalMap {
    pub fn flipped(mut self) -> Self {
        self.pixels_mut()
            .for_each(|pixel| pixel.0[1] = 255u8 - pixel.0[1]);

        self
    }
}

impl Default for NormalMap {
    fn default() -> Self {
        let default = RgbImage::from_fn(DEFAULT_TEXTURE_DIM, DEFAULT_TEXTURE_DIM, |_, _| {
            Rgb([128, 128, 255])
        });

        NormalMap(default)
    }
}

#[derive(NewType)]
pub struct RoughnessMap(pub GrayImage);

#[derive(NewType)]
pub struct AoMap(pub GrayImage);

#[derive(NewType)]
pub struct MetallicMap(pub GrayImage);

#[derive(NewType)]
pub struct EmissiveMap(pub GrayImage);

pub fn default_pbr_channel(value: u8) -> GrayImage {
    GrayImage::from_fn(DEFAULT_TEXTURE_DIM, DEFAULT_TEXTURE_DIM, |_, _| {
        Luma([value])
    })
}

#[derive(NewType)]
pub struct BasicPBR(RgbaImage);

impl BasicPBR {
    pub fn new(
        metallic: &MetallicMap,
        roughness: &RoughnessMap,
        ao: &AoMap,
        emissive: &EmissiveMap,
    ) -> Self {
        let result = swizzle_4((metallic, roughness, ao, emissive));

        Self(result)
    }
}

#[derive(NewType)]
pub struct AdvancedPBR(RgbaImage);

impl AdvancedPBR {
    pub fn new(normal: &NormalMap, ao: &AoMap, roughness: &RoughnessMap) -> Self {
        let max_dims = max_dimensions(&[
            normal.0.dimensions(),
            ao.dimensions(),
            roughness.dimensions(),
        ]);

        let result = RgbaImage::from_fn(max_dims.0, max_dims.1, |x, y| {
            let u = x as f32 / max_dims.0 as f32;
            let v = y as f32 / max_dims.1 as f32;

            let normal_sample = imageops::sample_nearest(&normal.0, u, v)
                .expect("uv in bounds")
                .0;

            Rgba([
                normal_sample[0],
                normal_sample[1],
                imageops::sample_nearest(&ao.0, u, v)
                    .expect("uv in bounds")
                    .0[0],
                imageops::sample_nearest(&roughness.0, u, v)
                    .expect("uv in bounds")
                    .0[0],
            ])
        });

        Self(result)
    }
}

#[cfg(test)]
mod test {
    use crate::hd2_pbr::{
        AdvancedPBR, AoMap, BasicPBR, EmissiveMap, MetallicMap, NormalMap, RoughnessMap,
        default_pbr_channel,
    };
    use image::ImageReader;
    use image::codecs::png::PngEncoder;
    use std::error::Error;
    use std::fs::{OpenOptions, create_dir_all};
    use std::path::PathBuf;

    #[test]
    pub fn test_basic_pbr() -> Result<(), Box<dyn Error>> {
        let test_folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test");
        let out_folder = test_folder.join("out");
        create_dir_all(&out_folder)?;
        let metallic = ImageReader::open(test_folder.join("HELMM.png"))?
            .decode()?
            .into_luma8();
        let metallic = MetallicMap(metallic);
        let roughness = ImageReader::open(test_folder.join("HELMR.png"))?
            .decode()?
            .into_luma8();
        let roughness = RoughnessMap(roughness);
        let ao = ImageReader::open(test_folder.join("HELMAO.png"))?
            .decode()?
            .into_luma8();
        let ao = AoMap(ao);
        let emissive = default_pbr_channel(255);
        let emissive = EmissiveMap(emissive);

        let pbr = BasicPBR::new(&metallic, &roughness, &ao, &emissive);
        let mut out_file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(out_folder.join("basic_pbr.png"))?;
        let png_encoder = PngEncoder::new(&mut out_file);
        pbr.write_with_encoder(png_encoder)?;

        Ok(())
    }

    #[test]
    pub fn test_advanced() -> Result<(), Box<dyn Error>> {
        let test_folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test");
        let out_folder = test_folder.join("out");
        create_dir_all(&out_folder)?;
        let normal = NormalMap(
            ImageReader::open(test_folder.join("HELMN.png"))?
                .decode()?
                .into_rgb8(),
        );
        let ao = ImageReader::open(test_folder.join("HELMAO.png"))?
            .decode()?
            .into_luma8();
        let _ao = AoMap(ao);
        let roughness = ImageReader::open(test_folder.join("HELMR.png"))?
            .decode()?
            .into_luma8();
        let roughness = RoughnessMap(roughness);
        let ao = AoMap(default_pbr_channel(255));

        let pbr = AdvancedPBR::new(&normal, &ao, &roughness);
        let mut out_file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(out_folder.join("advanced_pbr.png"))?;
        let png_encoder = PngEncoder::new(&mut out_file);
        pbr.write_with_encoder(png_encoder)?;

        Ok(())
    }
}
