use crate::{FilmError, LinearImage};
use imaging_core::{
    CurveExtrapolation, CurveInterpolation, FilmProfileData, LogExposureRgb, VirtualExposureNode,
};
use media_core::{PixelFormat, TransferFunction};
use std::{error::Error, fmt};

/// RGB optical density plus untouched straight alpha.
#[derive(Debug, Clone, PartialEq)]
pub struct FilmDensityImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 4]>,
}

#[derive(Debug, Clone)]
struct ChannelCurve {
    values: Vec<f32>,
    tangents: Vec<f32>,
}

/// Compiled, immutable RGB sensitometry curves for deterministic CPU evaluation.
#[derive(Debug, Clone)]
pub struct SensitometryEvaluator {
    exposure_axis: Vec<f32>,
    red: ChannelCurve,
    green: ChannelCurve,
    blue: ChannelCurve,
    interpolation: CurveInterpolation,
    extrapolation: CurveExtrapolation,
}

impl SensitometryEvaluator {
    pub fn compile(profile: &FilmProfileData) -> Result<Self, ReferenceRenderError> {
        profile
            .validate()
            .map_err(|error| ReferenceRenderError::InvalidProfile {
                path: error.path,
                reason: error.reason,
            })?;

        let data = &profile.sensitometry;
        let exposure_axis: Vec<_> = data
            .samples
            .iter()
            .map(|sample| sample.log_exposure)
            .collect();
        let channel = |values: Vec<f32>| ChannelCurve {
            tangents: monotonic_cubic_tangents(&exposure_axis, &values),
            values,
        };
        Ok(Self {
            red: channel(data.samples.iter().map(|sample| sample.red).collect()),
            green: channel(data.samples.iter().map(|sample| sample.green).collect()),
            blue: channel(data.samples.iter().map(|sample| sample.blue).collect()),
            exposure_axis,
            interpolation: data.interpolation,
            extrapolation: data.extrapolation,
        })
    }

    pub fn evaluate(&self, exposure: LogExposureRgb) -> Result<[f32; 3], ReferenceRenderError> {
        Ok([
            self.evaluate_channel("red", exposure.red, &self.red)?,
            self.evaluate_channel("green", exposure.green, &self.green)?,
            self.evaluate_channel("blue", exposure.blue, &self.blue)?,
        ])
    }

    fn evaluate_channel(
        &self,
        channel_name: &'static str,
        exposure: f32,
        curve: &ChannelCurve,
    ) -> Result<f32, ReferenceRenderError> {
        if !exposure.is_finite() {
            return Err(ReferenceRenderError::NonFiniteExposure {
                channel: channel_name,
            });
        }
        let last = self.exposure_axis.len() - 1;
        if exposure < self.exposure_axis[0] {
            return self.extrapolate(channel_name, exposure, curve, 0, 1);
        }
        if exposure > self.exposure_axis[last] {
            return self.extrapolate(channel_name, exposure, curve, last, last - 1);
        }
        if exposure == self.exposure_axis[last] {
            return Ok(curve.values[last]);
        }

        let upper = self
            .exposure_axis
            .partition_point(|sample| *sample <= exposure);
        let lower = upper - 1;
        if exposure == self.exposure_axis[lower] {
            return Ok(curve.values[lower]);
        }
        let x0 = self.exposure_axis[lower];
        let x1 = self.exposure_axis[upper];
        let interval = x1 - x0;
        let t = (exposure - x0) / interval;
        let value = match self.interpolation {
            CurveInterpolation::Linear => {
                curve.values[lower] + t * (curve.values[upper] - curve.values[lower])
            }
            CurveInterpolation::MonotonicCubic => {
                let t2 = t * t;
                let t3 = t2 * t;
                (2.0 * t3 - 3.0 * t2 + 1.0) * curve.values[lower]
                    + (t3 - 2.0 * t2 + t) * interval * curve.tangents[lower]
                    + (-2.0 * t3 + 3.0 * t2) * curve.values[upper]
                    + (t3 - t2) * interval * curve.tangents[upper]
            }
        };
        validate_density(channel_name, value)
    }

    fn extrapolate(
        &self,
        channel_name: &'static str,
        exposure: f32,
        curve: &ChannelCurve,
        endpoint: usize,
        neighbor: usize,
    ) -> Result<f32, ReferenceRenderError> {
        match self.extrapolation {
            CurveExtrapolation::Clamp => Ok(curve.values[endpoint]),
            CurveExtrapolation::Reject => Err(ReferenceRenderError::ExposureOutOfRange {
                channel: channel_name,
                exposure,
                minimum: self.exposure_axis[0],
                maximum: self.exposure_axis[self.exposure_axis.len() - 1],
            }),
            CurveExtrapolation::Linear => {
                let slope = (curve.values[endpoint] - curve.values[neighbor])
                    / (self.exposure_axis[endpoint] - self.exposure_axis[neighbor]);
                let value =
                    curve.values[endpoint] + (exposure - self.exposure_axis[endpoint]) * slope;
                validate_density(channel_name, value)
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuReferenceFilmExecutor;

impl CpuReferenceFilmExecutor {
    pub fn process(
        &self,
        input: &LinearImage,
        exposure: &VirtualExposureNode,
        profile: &FilmProfileData,
    ) -> Result<FilmDensityImage, ReferenceRenderError> {
        input
            .validate()
            .map_err(ReferenceRenderError::InvalidInput)?;
        if input.descriptor.pixel_format != PixelFormat::Rgba32Float {
            return Err(ReferenceRenderError::UnsupportedPixelFormat {
                actual: input.descriptor.pixel_format,
            });
        }
        if input.descriptor.transfer_function != TransferFunction::Linear {
            return Err(ReferenceRenderError::UnsupportedTransferFunction {
                actual: input.descriptor.transfer_function,
            });
        }
        let evaluator = SensitometryEvaluator::compile(profile)?;
        let mut pixels = Vec::with_capacity(input.pixels.len());
        for (pixel_index, pixel) in input.pixels.iter().enumerate() {
            if !pixel[3].is_finite() {
                return Err(ReferenceRenderError::NonFiniteAlpha { pixel_index });
            }
            let log_exposure = exposure
                .map_acescg([pixel[0], pixel[1], pixel[2]])
                .map_err(|error| ReferenceRenderError::InvalidExposure {
                    pixel_index,
                    path: error.path,
                    reason: error.reason,
                })?;
            let density = evaluator
                .evaluate(log_exposure)
                .map_err(|error| error.with_pixel_index(pixel_index))?;
            pixels.push([density[0], density[1], density[2], pixel[3]]);
        }
        Ok(FilmDensityImage {
            width: input.descriptor.width,
            height: input.descriptor.height,
            pixels,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceRenderError {
    InvalidInput(FilmError),
    UnsupportedPixelFormat {
        actual: PixelFormat,
    },
    UnsupportedTransferFunction {
        actual: TransferFunction,
    },
    InvalidProfile {
        path: String,
        reason: String,
    },
    InvalidExposure {
        pixel_index: usize,
        path: String,
        reason: String,
    },
    NonFiniteExposure {
        channel: &'static str,
    },
    ExposureOutOfRange {
        channel: &'static str,
        exposure: f32,
        minimum: f32,
        maximum: f32,
    },
    NonPhysicalDensity {
        channel: &'static str,
        density: f32,
    },
    NonFiniteAlpha {
        pixel_index: usize,
    },
    Pixel {
        pixel_index: usize,
        source: Box<ReferenceRenderError>,
    },
}

impl ReferenceRenderError {
    fn with_pixel_index(self, pixel_index: usize) -> Self {
        Self::Pixel {
            pixel_index,
            source: Box::new(self),
        }
    }
}

impl fmt::Display for ReferenceRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for ReferenceRenderError {}

fn validate_density(channel: &'static str, density: f32) -> Result<f32, ReferenceRenderError> {
    if !density.is_finite() || density < 0.0 {
        return Err(ReferenceRenderError::NonPhysicalDensity { channel, density });
    }
    Ok(density)
}

/// Fritsch-Carlson/PCHIP derivatives: shape-preserving for monotonic spans and
/// zero derivative at a sampled extremum.
fn monotonic_cubic_tangents(x: &[f32], y: &[f32]) -> Vec<f32> {
    let count = x.len();
    let intervals: Vec<_> = x.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let slopes: Vec<_> = y
        .windows(2)
        .zip(intervals.iter())
        .map(|(pair, interval)| (pair[1] - pair[0]) / interval)
        .collect();
    if count == 2 {
        return vec![slopes[0], slopes[0]];
    }

    let mut tangents = vec![0.0; count];
    tangents[0] = endpoint_tangent(intervals[0], intervals[1], slopes[0], slopes[1]);
    for index in 1..count - 1 {
        let before = slopes[index - 1];
        let after = slopes[index];
        if before == 0.0 || after == 0.0 || before.signum() != after.signum() {
            tangents[index] = 0.0;
        } else {
            let before_width = intervals[index - 1];
            let after_width = intervals[index];
            let weight_before = 2.0 * after_width + before_width;
            let weight_after = after_width + 2.0 * before_width;
            tangents[index] =
                (weight_before + weight_after) / (weight_before / before + weight_after / after);
        }
    }
    tangents[count - 1] = endpoint_tangent(
        intervals[count - 2],
        intervals[count - 3],
        slopes[count - 2],
        slopes[count - 3],
    );
    tangents
}

fn endpoint_tangent(
    endpoint_width: f32,
    adjacent_width: f32,
    endpoint_slope: f32,
    adjacent_slope: f32,
) -> f32 {
    let candidate = ((2.0 * endpoint_width + adjacent_width) * endpoint_slope
        - endpoint_width * adjacent_slope)
        / (endpoint_width + adjacent_width);
    if candidate.signum() != endpoint_slope.signum() {
        0.0
    } else if endpoint_slope.signum() != adjacent_slope.signum()
        && candidate.abs() > 3.0 * endpoint_slope.abs()
    {
        3.0 * endpoint_slope
    } else {
        candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CpuReferenceDevelopmentExecutor, CpuReferencePrintExecutor, LinearImage};
    use imaging_core::{NegativeSceneLinearPolicy, ProfileEnvelope};
    use media_core::{FrameDescriptor, PixelFormat, TransferFunction, WorkingColorSpace};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct GoldenFixture {
        fixture_version: u32,
        film_profile_id: String,
        development_profile_id: String,
        adapter: VirtualExposureNode,
        samples: Vec<GoldenSample>,
    }

    #[derive(Deserialize)]
    struct GoldenSample {
        scene_linear: f32,
        alpha: f32,
        expected_density: [f32; 3],
        expected_display_linear: [f32; 3],
    }

    fn profile() -> FilmProfileData {
        ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/synthetic-color-negative-500.json"
        ))
        .unwrap()
        .film_data()
        .unwrap()
    }

    fn adapter() -> VirtualExposureNode {
        VirtualExposureNode {
            reference_scene_linear: 0.18,
            reference_log_exposure: -1.0,
            exposure_compensation_ev: 0.0,
            minimum_scene_linear: 1.0e-6,
            negative_policy: NegativeSceneLinearPolicy::Reject,
        }
    }

    fn image(pixels: Vec<[f32; 4]>) -> LinearImage {
        LinearImage {
            descriptor: FrameDescriptor {
                width: pixels.len() as u32,
                height: 1,
                pixel_format: PixelFormat::Rgba32Float,
                color_space: WorkingColorSpace::AcesCg,
                transfer_function: TransferFunction::Linear,
            },
            pixels,
        }
    }

    #[test]
    fn monotonic_cubic_matches_knots_and_shape_preserving_midpoints() {
        let evaluator = SensitometryEvaluator::compile(&profile()).unwrap();
        let knots = evaluator
            .evaluate(LogExposureRgb {
                red: -4.0,
                green: -2.0,
                blue: 0.0,
            })
            .unwrap();
        assert_eq!(knots, [0.1, 0.52, 1.6]);

        let midpoint = evaluator
            .evaluate(LogExposureRgb {
                red: -3.0,
                green: -3.0,
                blue: -3.0,
            })
            .unwrap();
        assert!((midpoint[0] - 0.256_640_6).abs() < 1.0e-6);
        assert!(midpoint[0] > 0.1 && midpoint[0] < 0.55);
    }

    #[test]
    fn cpu_reference_executor_applies_exposure_and_preserves_straight_alpha() {
        let source = image(vec![
            [0.000_18, 0.001_8, 0.018, 0.0],
            [0.18, 0.18, 0.18, 0.25],
            [1.8, 1.8, 1.8, 1.0],
        ]);
        let output = CpuReferenceFilmExecutor
            .process(&source, &adapter(), &profile())
            .unwrap();
        assert_eq!((output.width, output.height), (3, 1));
        assert!((output.pixels[0][0] - 0.1).abs() < 1.0e-6);
        assert!((output.pixels[0][1] - 0.237_131_42).abs() < 1.0e-6);
        assert!((output.pixels[0][2] - 0.5).abs() < 1.0e-6);
        assert_eq!(output.pixels[0][3], 0.0);
        assert_eq!(output.pixels[1][3], 0.25);
        assert_eq!(output.pixels[2], [1.7, 1.65, 1.6, 1.0]);
    }

    #[test]
    fn extrapolation_policy_is_enforced() {
        let mut film = profile();
        film.sensitometry.extrapolation = CurveExtrapolation::Reject;
        let error = SensitometryEvaluator::compile(&film)
            .unwrap()
            .evaluate(LogExposureRgb {
                red: -5.0,
                green: -2.0,
                blue: -2.0,
            })
            .unwrap_err();
        assert!(matches!(
            error,
            ReferenceRenderError::ExposureOutOfRange { channel: "red", .. }
        ));
    }

    #[test]
    fn linear_interpolation_clamp_and_linear_extrapolation_follow_profile() {
        let mut film = profile();
        film.sensitometry.interpolation = CurveInterpolation::Linear;
        film.sensitometry.extrapolation = CurveExtrapolation::Clamp;
        let clamped = SensitometryEvaluator::compile(&film)
            .unwrap()
            .evaluate(LogExposureRgb {
                red: -5.0,
                green: -3.0,
                blue: 1.0,
            })
            .unwrap();
        assert!((clamped[0] - 0.1).abs() < 1.0e-6);
        assert!((clamped[1] - 0.305).abs() < 1.0e-6);
        assert!((clamped[2] - 1.6).abs() < 1.0e-6);

        film.sensitometry.extrapolation = CurveExtrapolation::Linear;
        let extended = SensitometryEvaluator::compile(&film)
            .unwrap()
            .evaluate(LogExposureRgb {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
            })
            .unwrap();
        assert!((extended[0] - 2.275).abs() < 1.0e-6);
        assert!((extended[1] - 2.215).abs() < 1.0e-6);
        assert!((extended[2] - 2.15).abs() < 1.0e-6);
    }

    #[test]
    fn invalid_pixel_reports_its_index() {
        let source = image(vec![[0.18; 4], [-0.01, 0.18, 0.18, 1.0]]);
        let error = CpuReferenceFilmExecutor
            .process(&source, &adapter(), &profile())
            .unwrap_err();
        assert!(matches!(
            error,
            ReferenceRenderError::InvalidExposure { pixel_index: 1, .. }
        ));
    }

    #[test]
    fn reference_executor_rejects_non_reference_frame_encoding() {
        let mut source = image(vec![[0.18; 4]]);
        source.descriptor.pixel_format = PixelFormat::Rgba16Float;
        assert!(matches!(
            CpuReferenceFilmExecutor.process(&source, &adapter(), &profile()),
            Err(ReferenceRenderError::UnsupportedPixelFormat { .. })
        ));

        source.descriptor.pixel_format = PixelFormat::Rgba32Float;
        source.descriptor.transfer_function = TransferFunction::Srgb;
        assert!(matches!(
            CpuReferenceFilmExecutor.process(&source, &adapter(), &profile()),
            Err(ReferenceRenderError::UnsupportedTransferFunction { .. })
        ));
    }

    #[test]
    fn bundled_golden_exposure_sweep_matches_reference_density() {
        let fixture: GoldenFixture = serde_json::from_str(include_str!(
            "../../../examples/fixtures/cpu-reference-film-density-v1.json"
        ))
        .unwrap();
        assert_eq!(fixture.fixture_version, 1);
        assert_eq!(
            fixture.film_profile_id,
            "org.universal-imaging.synthetic-color-negative-500"
        );
        assert_eq!(
            fixture.development_profile_id,
            "org.universal-imaging.synthetic-ecn2-development"
        );
        let source = image(
            fixture
                .samples
                .iter()
                .map(|sample| [sample.scene_linear; 3].map_with_alpha(sample.alpha))
                .collect(),
        );
        let density = CpuReferenceFilmExecutor
            .process(&source, &fixture.adapter, &profile())
            .unwrap();
        let development = ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/synthetic-ecn2-development.json"
        ))
        .unwrap()
        .development_data()
        .unwrap();
        let output = CpuReferenceDevelopmentExecutor
            .process(&density, &development, 0.0)
            .unwrap();
        for (pixel, sample) in output.pixels.iter().zip(&fixture.samples) {
            for channel in 0..3 {
                assert!((pixel[channel] - sample.expected_density[channel]).abs() < 1.0e-5);
            }
            assert_eq!(pixel[3], sample.alpha);
        }
        let print = ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/synthetic-theatrical-print.json"
        ))
        .unwrap()
        .print_data()
        .unwrap();
        let display_linear = CpuReferencePrintExecutor.process(&output, &print).unwrap();
        for (pixel, sample) in display_linear.pixels.iter().zip(&fixture.samples) {
            for channel in 0..3 {
                assert!((pixel[channel] - sample.expected_display_linear[channel]).abs() < 1.0e-5);
            }
            assert_eq!(pixel[3], sample.alpha);
        }
    }

    trait RgbWithAlpha {
        fn map_with_alpha(self, alpha: f32) -> [f32; 4];
    }

    impl RgbWithAlpha for [f32; 3] {
        fn map_with_alpha(self, alpha: f32) -> [f32; 4] {
            [self[0], self[1], self[2], alpha]
        }
    }
}
