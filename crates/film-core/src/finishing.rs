use crate::{FilmDensityImage, FilmError, LinearImage};
use imaging_core::{
    Chromaticity, DevelopmentProfileData, DevelopmentProfileKind, DisplayPrimaries,
    DisplayProfileData, OutputEncoding, OutputTransformMethod, OutputTransformProfileData,
    PrintProfileData, PrintProfileKind, PrintResponseModel, ToneMappingMethod,
};
use media_core::{PixelFormat, TransferFunction};
use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayEncodedImage {
    pub width: u32,
    pub height: u32,
    pub transfer_function: TransferFunction,
    pub pixels: Vec<[f32; 4]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayLinearImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 4]>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FinishingError {
    InvalidInput(FilmError),
    InvalidProfile { path: String, reason: String },
    UnsupportedDevelopment,
    UnsupportedPushPull { stops: f32 },
    UnsupportedPrintProcess,
    UnsupportedPrintResponse,
    UnsupportedOutputMethod,
    UnsupportedToneMapping,
    MismatchedDisplayTarget,
    InvalidColorMatrix,
    NonFinitePixel { pixel_index: usize },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuReferencePrintExecutor;

impl CpuReferencePrintExecutor {
    pub fn process(
        &self,
        input: &FilmDensityImage,
        profile: &PrintProfileData,
    ) -> Result<DisplayLinearImage, FinishingError> {
        profile
            .validate()
            .map_err(|error| FinishingError::InvalidProfile {
                path: error.path,
                reason: error.reason,
            })?;
        if !matches!(
            profile.print_type,
            PrintProfileKind::Photochemical | PrintProfileKind::Paper
        ) {
            return Err(FinishingError::UnsupportedPrintProcess);
        }
        if profile.response_model != PrintResponseModel::InverseDensityPreviewV1 {
            return Err(FinishingError::UnsupportedPrintResponse);
        }
        let expected = input.width as usize * input.height as usize;
        if input.pixels.len() != expected {
            return Err(FinishingError::InvalidInput(FilmError::InvalidBufferLength));
        }
        let base = profile.base_density.expect("validated print base density");
        let printer_offset = profile.exposure_offset_ev * std::f32::consts::LOG10_2;
        let mut pixels = Vec::with_capacity(input.pixels.len());
        for (pixel_index, pixel) in input.pixels.iter().enumerate() {
            if !pixel.iter().all(|value| value.is_finite()) {
                return Err(FinishingError::NonFinitePixel { pixel_index });
            }
            let response = |density: f32, base_density: f32| {
                let effective =
                    ((density - base_density) * profile.contrast_scale - printer_offset).max(0.0);
                1.0 - 10.0_f32.powf(-effective)
            };
            pixels.push([
                response(pixel[0], base.red),
                response(pixel[1], base.green),
                response(pixel[2], base.blue),
                pixel[3],
            ]);
        }
        Ok(DisplayLinearImage {
            width: input.width,
            height: input.height,
            pixels,
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuReferenceDisplayEncoder;

impl CpuReferenceDisplayEncoder {
    pub fn process(
        &self,
        input: &DisplayLinearImage,
        display: &DisplayProfileData,
    ) -> Result<DisplayEncodedImage, FinishingError> {
        display
            .validate()
            .map_err(|error| FinishingError::InvalidProfile {
                path: error.path,
                reason: error.reason,
            })?;
        let expected = input.width as usize * input.height as usize;
        if input.pixels.len() != expected {
            return Err(FinishingError::InvalidInput(FilmError::InvalidBufferLength));
        }
        let mut pixels = Vec::with_capacity(input.pixels.len());
        for (pixel_index, pixel) in input.pixels.iter().enumerate() {
            if !pixel.iter().all(|value| value.is_finite()) {
                return Err(FinishingError::NonFinitePixel { pixel_index });
            }
            pixels.push([
                encode(pixel[0].clamp(0.0, 1.0), display.transfer_function),
                encode(pixel[1].clamp(0.0, 1.0), display.transfer_function),
                encode(pixel[2].clamp(0.0, 1.0), display.transfer_function),
                pixel[3],
            ]);
        }
        Ok(DisplayEncodedImage {
            width: input.width,
            height: input.height,
            transfer_function: display.transfer_function,
            pixels,
        })
    }
}

impl fmt::Display for FinishingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for FinishingError {}

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuReferenceDevelopmentExecutor;

impl CpuReferenceDevelopmentExecutor {
    pub fn process(
        &self,
        input: &FilmDensityImage,
        profile: &DevelopmentProfileData,
        push_pull_stops: f32,
    ) -> Result<FilmDensityImage, FinishingError> {
        profile
            .validate()
            .map_err(|error| FinishingError::InvalidProfile {
                path: error.path,
                reason: error.reason,
            })?;
        if profile.development_type != DevelopmentProfileKind::Chemical {
            return Err(FinishingError::UnsupportedDevelopment);
        }
        if push_pull_stops < profile.push_pull_stops.min
            || push_pull_stops > profile.push_pull_stops.max
            || !push_pull_stops.is_finite()
        {
            return Err(FinishingError::UnsupportedPushPull {
                stops: push_pull_stops,
            });
        }
        // v1 has no measured push/pull response curve. Only the calibrated normal process
        // is scientifically defined; accepting another value would fabricate a response.
        if push_pull_stops != 0.0 {
            return Err(FinishingError::UnsupportedPushPull {
                stops: push_pull_stops,
            });
        }
        let expected = input.width as usize * input.height as usize;
        if input.pixels.len() != expected {
            return Err(FinishingError::InvalidInput(FilmError::InvalidBufferLength));
        }
        let mut pixels = Vec::with_capacity(input.pixels.len());
        for (pixel_index, pixel) in input.pixels.iter().enumerate() {
            if !pixel.iter().all(|value| value.is_finite()) {
                return Err(FinishingError::NonFinitePixel { pixel_index });
            }
            pixels.push([
                pixel[0] * profile.contrast_scale,
                pixel[1] * profile.contrast_scale,
                pixel[2] * profile.contrast_scale,
                pixel[3],
            ]);
        }
        Ok(FilmDensityImage {
            width: input.width,
            height: input.height,
            pixels,
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuReferenceOutputExecutor;

impl CpuReferenceOutputExecutor {
    pub fn process(
        &self,
        input: &LinearImage,
        output: &OutputTransformProfileData,
        display: &DisplayProfileData,
    ) -> Result<DisplayEncodedImage, FinishingError> {
        input.validate().map_err(FinishingError::InvalidInput)?;
        if input.descriptor.pixel_format != PixelFormat::Rgba32Float
            || input.descriptor.transfer_function != TransferFunction::Linear
        {
            return Err(FinishingError::InvalidInput(
                FilmError::UnsupportedWorkingSpace,
            ));
        }
        output
            .validate()
            .map_err(|error| FinishingError::InvalidProfile {
                path: error.path,
                reason: error.reason,
            })?;
        display
            .validate()
            .map_err(|error| FinishingError::InvalidProfile {
                path: error.path,
                reason: error.reason,
            })?;
        if output.method != OutputTransformMethod::MatrixToneCurve {
            return Err(FinishingError::UnsupportedOutputMethod);
        }
        if output.tone_mapping != ToneMappingMethod::None {
            return Err(FinishingError::UnsupportedToneMapping);
        }
        if output.output_transfer_function != display.transfer_function
            || (output.peak_luminance_nits - display.peak_luminance_nits).abs() > 1.0e-4
            || !display_matches_encoding(display.primaries, output.output_encoding)
        {
            return Err(FinishingError::MismatchedDisplayTarget);
        }

        let matrix = acescg_to_display_matrix(display.primaries)?;
        let mut pixels = Vec::with_capacity(input.pixels.len());
        for (pixel_index, pixel) in input.pixels.iter().enumerate() {
            if !pixel.iter().all(|value| value.is_finite()) {
                return Err(FinishingError::NonFinitePixel { pixel_index });
            }
            let linear =
                multiply_vector(matrix, [pixel[0] as f64, pixel[1] as f64, pixel[2] as f64]);
            pixels.push([
                encode(linear[0].clamp(0.0, 1.0) as f32, display.transfer_function),
                encode(linear[1].clamp(0.0, 1.0) as f32, display.transfer_function),
                encode(linear[2].clamp(0.0, 1.0) as f32, display.transfer_function),
                pixel[3],
            ]);
        }
        Ok(DisplayEncodedImage {
            width: input.descriptor.width,
            height: input.descriptor.height,
            transfer_function: display.transfer_function,
            pixels,
        })
    }
}

fn display_matches_encoding(primaries: DisplayPrimaries, encoding: OutputEncoding) -> bool {
    let expected = match encoding {
        OutputEncoding::Rec709 | OutputEncoding::Srgb => DisplayPrimaries {
            red: Chromaticity { x: 0.64, y: 0.33 },
            green: Chromaticity { x: 0.30, y: 0.60 },
            blue: Chromaticity { x: 0.15, y: 0.06 },
            white: Chromaticity {
                x: 0.3127,
                y: 0.3290,
            },
        },
        OutputEncoding::DisplayP3 => DisplayPrimaries {
            red: Chromaticity { x: 0.68, y: 0.32 },
            green: Chromaticity { x: 0.265, y: 0.69 },
            blue: Chromaticity { x: 0.15, y: 0.06 },
            white: Chromaticity {
                x: 0.3127,
                y: 0.3290,
            },
        },
        OutputEncoding::Rec2020Pq | OutputEncoding::Rec2020Hlg => DisplayPrimaries {
            red: Chromaticity { x: 0.708, y: 0.292 },
            green: Chromaticity { x: 0.170, y: 0.797 },
            blue: Chromaticity { x: 0.131, y: 0.046 },
            white: Chromaticity {
                x: 0.3127,
                y: 0.3290,
            },
        },
        OutputEncoding::Custom => return true,
    };
    let close = |left: Chromaticity, right: Chromaticity| {
        (left.x - right.x).abs() <= 1.0e-4 && (left.y - right.y).abs() <= 1.0e-4
    };
    close(primaries.red, expected.red)
        && close(primaries.green, expected.green)
        && close(primaries.blue, expected.blue)
        && close(primaries.white, expected.white)
}

type Matrix3 = [[f64; 3]; 3];

fn acescg_to_display_matrix(target: DisplayPrimaries) -> Result<Matrix3, FinishingError> {
    let source = DisplayPrimaries {
        red: Chromaticity { x: 0.713, y: 0.293 },
        green: Chromaticity { x: 0.165, y: 0.830 },
        blue: Chromaticity { x: 0.128, y: 0.044 },
        white: Chromaticity {
            x: 0.32168,
            y: 0.33767,
        },
    };
    let source_to_xyz = rgb_to_xyz(source)?;
    let target_to_xyz = rgb_to_xyz(target)?;
    let target_from_xyz = invert(target_to_xyz).ok_or(FinishingError::InvalidColorMatrix)?;
    let adaptation = bradford_adaptation(source.white, target.white)?;
    Ok(multiply_matrix(
        target_from_xyz,
        multiply_matrix(adaptation, source_to_xyz),
    ))
}

fn rgb_to_xyz(primaries: DisplayPrimaries) -> Result<Matrix3, FinishingError> {
    let column = |xy: Chromaticity| {
        [
            xy.x as f64 / xy.y as f64,
            1.0,
            (1.0 - xy.x - xy.y) as f64 / xy.y as f64,
        ]
    };
    let red = column(primaries.red);
    let green = column(primaries.green);
    let blue = column(primaries.blue);
    let basis = [
        [red[0], green[0], blue[0]],
        [red[1], green[1], blue[1]],
        [red[2], green[2], blue[2]],
    ];
    let inverse = invert(basis).ok_or(FinishingError::InvalidColorMatrix)?;
    let white = column(primaries.white);
    let scale = multiply_vector(inverse, white);
    Ok([
        [
            basis[0][0] * scale[0],
            basis[0][1] * scale[1],
            basis[0][2] * scale[2],
        ],
        [
            basis[1][0] * scale[0],
            basis[1][1] * scale[1],
            basis[1][2] * scale[2],
        ],
        [
            basis[2][0] * scale[0],
            basis[2][1] * scale[1],
            basis[2][2] * scale[2],
        ],
    ])
}

fn bradford_adaptation(
    source: Chromaticity,
    target: Chromaticity,
) -> Result<Matrix3, FinishingError> {
    let bradford = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    let inverse = invert(bradford).ok_or(FinishingError::InvalidColorMatrix)?;
    let xyz = |xy: Chromaticity| {
        [
            xy.x as f64 / xy.y as f64,
            1.0,
            (1.0 - xy.x - xy.y) as f64 / xy.y as f64,
        ]
    };
    let source_cone = multiply_vector(bradford, xyz(source));
    let target_cone = multiply_vector(bradford, xyz(target));
    let scale = [
        [target_cone[0] / source_cone[0], 0.0, 0.0],
        [0.0, target_cone[1] / source_cone[1], 0.0],
        [0.0, 0.0, target_cone[2] / source_cone[2]],
    ];
    Ok(multiply_matrix(inverse, multiply_matrix(scale, bradford)))
}

fn multiply_matrix(left: Matrix3, right: Matrix3) -> Matrix3 {
    let mut output = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            output[row][column] = (0..3)
                .map(|index| left[row][index] * right[index][column])
                .sum();
        }
    }
    output
}

fn multiply_vector(matrix: Matrix3, vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn invert(matrix: Matrix3) -> Option<Matrix3> {
    let determinant = matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);
    if !determinant.is_finite() || determinant.abs() < 1.0e-12 {
        return None;
    }
    let inverse = 1.0 / determinant;
    Some([
        [
            (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1]) * inverse,
            (matrix[0][2] * matrix[2][1] - matrix[0][1] * matrix[2][2]) * inverse,
            (matrix[0][1] * matrix[1][2] - matrix[0][2] * matrix[1][1]) * inverse,
        ],
        [
            (matrix[1][2] * matrix[2][0] - matrix[1][0] * matrix[2][2]) * inverse,
            (matrix[0][0] * matrix[2][2] - matrix[0][2] * matrix[2][0]) * inverse,
            (matrix[0][2] * matrix[1][0] - matrix[0][0] * matrix[1][2]) * inverse,
        ],
        [
            (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]) * inverse,
            (matrix[0][1] * matrix[2][0] - matrix[0][0] * matrix[2][1]) * inverse,
            (matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]) * inverse,
        ],
    ])
}

fn encode(value: f32, transfer: TransferFunction) -> f32 {
    match transfer {
        TransferFunction::Srgb => {
            if value <= 0.003_130_8 {
                12.92 * value
            } else {
                1.055 * value.powf(1.0 / 2.4) - 0.055
            }
        }
        TransferFunction::Rec709 => {
            if value < 0.018 {
                4.5 * value
            } else {
                1.099 * value.powf(0.45) - 0.099
            }
        }
        TransferFunction::Pq => {
            let m1 = 2610.0 / 16384.0;
            let m2 = 2523.0 / 32.0;
            let c1 = 3424.0 / 4096.0;
            let c2 = 2413.0 / 128.0;
            let c3 = 2392.0 / 128.0;
            let power = value.powf(m1);
            ((c1 + c2 * power) / (1.0 + c3 * power)).powf(m2)
        }
        TransferFunction::Hlg => {
            let a: f32 = 0.178_832_77;
            let b = 1.0 - 4.0 * a;
            let c = 0.5 - a * (4.0 * a).ln();
            if value <= 1.0 / 12.0 {
                (3.0 * value).sqrt()
            } else {
                a * (12.0 * value - b).ln() + c
            }
        }
        TransferFunction::Linear => value,
        TransferFunction::Log => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imaging_core::ProfileEnvelope;
    use media_core::{FrameDescriptor, WorkingColorSpace};

    fn linear_image(pixels: Vec<[f32; 4]>) -> LinearImage {
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
    fn reference_development_applies_normal_process_contrast_and_rejects_push() {
        let development = ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/synthetic-ecn2-development.json"
        ))
        .unwrap()
        .development_data()
        .unwrap();
        let input = FilmDensityImage {
            width: 1,
            height: 1,
            pixels: vec![[0.5, 0.4, 0.3, 0.25]],
        };
        let output = CpuReferenceDevelopmentExecutor
            .process(&input, &development, 0.0)
            .unwrap();
        assert_eq!(output.pixels[0], input.pixels[0]);
        assert!(matches!(
            CpuReferenceDevelopmentExecutor.process(&input, &development, 1.0),
            Err(FinishingError::UnsupportedPushPull { .. })
        ));
    }

    #[test]
    fn matrix_output_maps_neutral_acescg_to_neutral_rec709_and_preserves_alpha() {
        let output = ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/aces-rec709-output-transform.json"
        ))
        .unwrap()
        .output_transform_data()
        .unwrap();
        let mut display = ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/reference-rec709-display.json"
        ))
        .unwrap()
        .display_data()
        .unwrap();
        let rendered = CpuReferenceOutputExecutor
            .process(
                &linear_image(vec![[0.18, 0.18, 0.18, 0.25]]),
                &output,
                &display,
            )
            .unwrap();
        let pixel = rendered.pixels[0];
        assert!((pixel[0] - pixel[1]).abs() < 1.0e-5);
        assert!((pixel[1] - pixel[2]).abs() < 1.0e-5);
        assert!((pixel[0] - 0.409_007_7).abs() < 1.0e-5);
        assert_eq!(pixel[3], 0.25);

        display.primaries.red.x = 0.65;
        assert!(matches!(
            CpuReferenceOutputExecutor.process(
                &linear_image(vec![[0.18, 0.18, 0.18, 1.0]]),
                &output,
                &display
            ),
            Err(FinishingError::MismatchedDisplayTarget)
        ));
    }

    #[test]
    fn inverse_density_print_is_monotonic_and_printer_exposure_darkens_output() {
        let mut print = ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/synthetic-theatrical-print.json"
        ))
        .unwrap()
        .print_data()
        .unwrap();
        let input = FilmDensityImage {
            width: 2,
            height: 1,
            pixels: vec![[0.08, 0.07, 0.06, 0.25], [1.0, 1.0, 1.0, 0.75]],
        };
        let normal = CpuReferencePrintExecutor.process(&input, &print).unwrap();
        assert_eq!(normal.pixels[0], [0.0, 0.0, 0.0, 0.25]);
        assert!(normal.pixels[1][0] > normal.pixels[0][0]);
        assert_eq!(normal.pixels[1][3], 0.75);

        print.exposure_offset_ev = 1.0;
        let exposed = CpuReferencePrintExecutor.process(&input, &print).unwrap();
        assert!(exposed.pixels[1][0] < normal.pixels[1][0]);

        let display = ProfileEnvelope::from_json(include_str!(
            "../../../examples/profiles/reference-rec709-display.json"
        ))
        .unwrap()
        .display_data()
        .unwrap();
        let encoded = CpuReferenceDisplayEncoder
            .process(&normal, &display)
            .unwrap();
        assert_eq!(encoded.transfer_function, TransferFunction::Rec709);
        assert!(encoded.pixels[1][0] > encoded.pixels[0][0]);
        assert_eq!(encoded.pixels[0][3], 0.25);
        assert_eq!(encoded.pixels[1][3], 0.75);
    }
}
