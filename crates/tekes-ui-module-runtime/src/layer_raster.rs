//! Validated placement of an independent bitmap on the parent's pixel lattice.
use anyhow::{Result, ensure};
use tekes_ui_scene::{Point, Rect};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayerRasterLayout {
    logical_size: [u32; 2],
    physical_origin_px: [f32; 2],
    pixel_origin: [f32; 2],
    phase: [f32; 2],
    pixel_size: (u32, u32),
    scale: f32,
}
impl LayerRasterLayout {
    pub(crate) fn new(
        logical_size: [u32; 2],
        scale: f32,
        physical_origin_px: [f32; 2],
    ) -> Result<Self> {
        ensure!(
            logical_size.iter().all(|axis| (1..=4096).contains(axis)),
            "invalid layer logical extent"
        );
        ensure!(
            scale.is_finite() && (0.5..=4.).contains(&scale),
            "invalid layer backing scale"
        );
        ensure!(
            physical_origin_px.iter().all(|value| value.is_finite()),
            "invalid layer physical origin"
        );
        let pixel_origin = physical_origin_px.map(f32::floor);
        let phase = [
            physical_origin_px[0] - pixel_origin[0],
            physical_origin_px[1] - pixel_origin[1],
        ];
        ensure!(
            phase.iter().all(|value| (0. ..1.).contains(value)),
            "invalid layer raster phase"
        );
        let covered = [
            (logical_size[0] as f32 * scale + phase[0]).ceil(),
            (logical_size[1] as f32 * scale + phase[1]).ceil(),
        ];
        ensure!(
            covered
                .iter()
                .all(|value| value.is_finite() && (1. ..=4096.).contains(value)),
            "layer raster outside negotiated extent limits"
        );
        Ok(Self {
            logical_size,
            physical_origin_px,
            pixel_origin,
            phase,
            pixel_size: (covered[0] as u32, covered[1] as u32),
            scale,
        })
    }
    pub(crate) fn logical_size(&self) -> [u32; 2] {
        self.logical_size
    }
    pub(crate) fn physical_origin_px(&self) -> [f32; 2] {
        self.physical_origin_px
    }
    pub(crate) fn pixel_size(&self) -> (u32, u32) {
        self.pixel_size
    }
    pub(crate) fn scale(&self) -> f32 {
        self.scale
    }
    pub(crate) fn phase_logical(&self) -> Point {
        Point::new(self.phase[0] / self.scale, self.phase[1] / self.scale)
    }
    pub(crate) fn image_rect(&self) -> Rect {
        Rect::new(
            self.pixel_origin[0] / self.scale,
            self.pixel_origin[1] / self.scale,
            self.pixel_size.0 as f32 / self.scale,
            self.pixel_size.1 as f32 / self.scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_grid_phase_keeps_one_texel_per_pixel_and_exact_input_translation() {
        let layer = LayerRasterLayout::new([333, 500], 2., [666.6667, 0.]).unwrap();
        assert_eq!(layer.pixel_size(), (667, 1000));
        assert_eq!(layer.image_rect(), Rect::new(333., 0., 333.5, 500.));
        assert_eq!(
            layer.image_rect().origin.x + layer.phase_logical().x,
            333.33334
        );
        assert_eq!(layer.logical_size(), [333, 500]);
        for scale in [1., 2., 3.] {
            for x in [0., 333.33334, 666.6667] {
                let t = [x * scale, 17.25 * scale];
                let layer = LayerRasterLayout::new([333, 500], scale, t).unwrap();
                let image = layer.image_rect();
                let phase = layer.phase_logical();
                assert_eq!(image.origin.x + phase.x, x);
                assert_eq!(image.origin.y + phase.y, 17.25);
                assert_eq!(image.size.width * scale, layer.pixel_size().0 as f32);
                assert_eq!(image.size.height * scale, layer.pixel_size().1 as f32);
                assert_eq!((image.origin.x * scale).fract(), 0.);
                assert_eq!((image.origin.y * scale).fract(), 0.);
            }
        }
    }
    #[test]
    fn invalid_layer_geometry_is_rejected_before_any_render() {
        assert!(LayerRasterLayout::new([0, 500], 2., [0., 0.]).is_err());
        assert!(LayerRasterLayout::new([4096, 500], 2., [0., 0.]).is_err());
        assert!(LayerRasterLayout::new([333, 500], f32::NAN, [0., 0.]).is_err());
        assert!(LayerRasterLayout::new([333, 500], 2., [f32::INFINITY, 0.]).is_err());
        assert!(LayerRasterLayout::new([2048, 500], 2., [0.5, 0.]).is_err());
    }
}
