//! A platform-neutral, serializable description of a complete imaging chain.
//!
//! This crate describes how an image is formed. Rendering implementations live in
//! specialized crates such as `film-core`, GPU backends, and platform adapters.

use media_core::WorkingColorSpace;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineIntent {
    PhysicalCapture,
    Emulation,
    PostProduction,
}

/// Distinguishes measured hardware from a creative simulation or a signal transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    Observed,
    Simulated,
    Transform,
}

/// Signal domains make physically invalid node connections detectable before rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalDomain {
    SceneLight,
    OpticalImage,
    FilmLatentImage,
    FilmDensity,
    SensorRaw,
    SceneLinear,
    DisplayLinear,
    DisplayEncoded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImagingPipeline {
    pub id: String,
    pub schema_version: u32,
    pub intent: PipelineIntent,
    pub working_color_space: WorkingColorSpace,
    pub nodes: Vec<PipelineNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineNode {
    pub id: String,
    pub role: NodeRole,
    pub enabled: bool,
    pub operation: ImagingOperation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "config", rename_all = "snake_case")]
pub enum ImagingOperation {
    Source(SourceNode),
    Camera(CameraNode),
    Lens(LensNode),
    CaptureMedium(CaptureMediumNode),
    Development(DevelopmentNode),
    Print(PrintNode),
    OutputTransform(OutputTransformNode),
    Display(DisplayNode),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceNode {
    pub source: SourceKind,
    pub output_domain: SignalDomain,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Scene,
    LiveCamera,
    ImageFile,
    VideoFile,
    Generated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraNode {
    pub profile_id: String,
    pub shutter_seconds: f64,
    pub exposure_compensation_ev: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensNode {
    pub profile_id: String,
    pub focal_length_mm: f32,
    pub aperture_f_number: f32,
    pub focus_distance_m: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureMediumNode {
    pub medium: CaptureMedium,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaptureMedium {
    Film {
        profile_id: String,
        exposure_index: f32,
    },
    DigitalSensor {
        profile_id: String,
        iso: f32,
        bit_depth: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentNode {
    pub process: DevelopmentProcess,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DevelopmentProcess {
    Chemical {
        process_id: String,
        push_pull_stops: f32,
    },
    DigitalRaw {
        profile_id: String,
        exposure_ev: f32,
        white_balance_kelvin: u32,
        tint: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintNode {
    pub process: PrintProcess,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PrintProcess {
    Photochemical { profile_id: String },
    DigitalIntermediate { profile_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputTransformNode {
    pub transform_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayNode {
    pub profile_id: String,
    pub peak_luminance_nits: f32,
    pub surround: DisplaySurround,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplaySurround {
    Dark,
    Dim,
    Average,
}

impl ImagingOperation {
    pub fn input_domain(&self) -> Option<SignalDomain> {
        match self {
            Self::Source(_) => None,
            Self::Camera(_) => Some(SignalDomain::SceneLight),
            Self::Lens(_) => Some(SignalDomain::SceneLight),
            Self::CaptureMedium(_) => Some(SignalDomain::OpticalImage),
            Self::Development(node) => Some(match node.process {
                DevelopmentProcess::Chemical { .. } => SignalDomain::FilmLatentImage,
                DevelopmentProcess::DigitalRaw { .. } => SignalDomain::SensorRaw,
            }),
            Self::Print(node) => Some(match node.process {
                PrintProcess::Photochemical { .. } => SignalDomain::FilmDensity,
                PrintProcess::DigitalIntermediate { .. } => SignalDomain::SceneLinear,
            }),
            Self::OutputTransform(_) => Some(SignalDomain::SceneLinear),
            Self::Display(_) => Some(SignalDomain::DisplayLinear),
        }
    }

    pub fn output_domain(&self) -> SignalDomain {
        match self {
            Self::Source(node) => node.output_domain,
            Self::Camera(_) => SignalDomain::SceneLight,
            Self::Lens(_) => SignalDomain::OpticalImage,
            Self::CaptureMedium(node) => match node.medium {
                CaptureMedium::Film { .. } => SignalDomain::FilmLatentImage,
                CaptureMedium::DigitalSensor { .. } => SignalDomain::SensorRaw,
            },
            Self::Development(node) => match node.process {
                DevelopmentProcess::Chemical { .. } => SignalDomain::FilmDensity,
                DevelopmentProcess::DigitalRaw { .. } => SignalDomain::SceneLinear,
            },
            Self::Print(_) | Self::OutputTransform(_) => SignalDomain::DisplayLinear,
            Self::Display(_) => SignalDomain::DisplayEncoded,
        }
    }
}

impl ImagingPipeline {
    pub fn validate(&self) -> Result<(), PipelineError> {
        if self.schema_version == 0 {
            return Err(PipelineError::InvalidSchemaVersion);
        }
        if self.nodes.is_empty() {
            return Err(PipelineError::Empty);
        }

        let enabled: Vec<_> = self.nodes.iter().filter(|node| node.enabled).collect();
        if enabled.is_empty() {
            return Err(PipelineError::Empty);
        }
        if !matches!(enabled[0].operation, ImagingOperation::Source(_)) {
            return Err(PipelineError::SourceMustBeFirst);
        }
        if enabled
            .iter()
            .skip(1)
            .any(|node| matches!(node.operation, ImagingOperation::Source(_)))
        {
            return Err(PipelineError::MultipleSources);
        }

        let mut ids = HashSet::new();
        for node in &self.nodes {
            if node.id.trim().is_empty() || !ids.insert(node.id.as_str()) {
                return Err(PipelineError::InvalidNodeId(node.id.clone()));
            }
        }

        let mut domain = enabled[0].operation.output_domain();
        for node in enabled.iter().skip(1) {
            let expected = node
                .operation
                .input_domain()
                .expect("only the first enabled node may be a source");
            if domain != expected {
                return Err(PipelineError::DomainMismatch {
                    node_id: node.id.clone(),
                    expected,
                    actual: domain,
                });
            }
            domain = node.operation.output_domain();
        }

        if domain != SignalDomain::DisplayEncoded {
            return Err(PipelineError::Incomplete {
                final_domain: domain,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineError {
    Empty,
    InvalidSchemaVersion,
    InvalidNodeId(String),
    SourceMustBeFirst,
    MultipleSources,
    DomainMismatch {
        node_id: String,
        expected: SignalDomain,
        actual: SignalDomain,
    },
    Incomplete {
        final_domain: SignalDomain,
    },
}

impl fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for PipelineError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, role: NodeRole, operation: ImagingOperation) -> PipelineNode {
        PipelineNode {
            id: id.into(),
            role,
            enabled: true,
            operation,
        }
    }

    fn base(nodes: Vec<PipelineNode>) -> ImagingPipeline {
        ImagingPipeline {
            id: "test-pipeline".into(),
            schema_version: 1,
            intent: PipelineIntent::Emulation,
            working_color_space: WorkingColorSpace::AcesCg,
            nodes,
        }
    }

    fn source() -> PipelineNode {
        node(
            "scene",
            NodeRole::Observed,
            ImagingOperation::Source(SourceNode {
                source: SourceKind::Scene,
                output_domain: SignalDomain::SceneLight,
                profile_id: None,
            }),
        )
    }

    fn lens() -> PipelineNode {
        node(
            "lens",
            NodeRole::Simulated,
            ImagingOperation::Lens(LensNode {
                profile_id: "reference-lens".into(),
                focal_length_mm: 50.0,
                aperture_f_number: 2.8,
                focus_distance_m: Some(3.0),
            }),
        )
    }

    fn camera() -> PipelineNode {
        node(
            "camera",
            NodeRole::Observed,
            ImagingOperation::Camera(CameraNode {
                profile_id: "reference-camera-body".into(),
                shutter_seconds: 1.0 / 48.0,
                exposure_compensation_ev: 0.0,
            }),
        )
    }

    fn display() -> PipelineNode {
        node(
            "display",
            NodeRole::Transform,
            ImagingOperation::Display(DisplayNode {
                profile_id: "rec709-reference".into(),
                peak_luminance_nits: 100.0,
                surround: DisplaySurround::Dim,
            }),
        )
    }

    #[test]
    fn validates_complete_film_chain() {
        let pipeline = base(vec![
            source(),
            camera(),
            lens(),
            node(
                "negative",
                NodeRole::Simulated,
                ImagingOperation::CaptureMedium(CaptureMediumNode {
                    medium: CaptureMedium::Film {
                        profile_id: "synthetic-negative-500".into(),
                        exposure_index: 500.0,
                    },
                }),
            ),
            node(
                "development",
                NodeRole::Simulated,
                ImagingOperation::Development(DevelopmentNode {
                    process: DevelopmentProcess::Chemical {
                        process_id: "ecn2-reference".into(),
                        push_pull_stops: 0.0,
                    },
                }),
            ),
            node(
                "print",
                NodeRole::Simulated,
                ImagingOperation::Print(PrintNode {
                    process: PrintProcess::Photochemical {
                        profile_id: "synthetic-print".into(),
                    },
                }),
            ),
            display(),
        ]);
        assert_eq!(pipeline.validate(), Ok(()));
    }

    #[test]
    fn validates_complete_digital_chain() {
        let pipeline = base(vec![
            source(),
            camera(),
            lens(),
            node(
                "sensor",
                NodeRole::Simulated,
                ImagingOperation::CaptureMedium(CaptureMediumNode {
                    medium: CaptureMedium::DigitalSensor {
                        profile_id: "reference-bayer-sensor".into(),
                        iso: 400.0,
                        bit_depth: 14,
                    },
                }),
            ),
            node(
                "raw-development",
                NodeRole::Transform,
                ImagingOperation::Development(DevelopmentNode {
                    process: DevelopmentProcess::DigitalRaw {
                        profile_id: "neutral-raw".into(),
                        exposure_ev: 0.0,
                        white_balance_kelvin: 5600,
                        tint: 0.0,
                    },
                }),
            ),
            node(
                "output-transform",
                NodeRole::Transform,
                ImagingOperation::OutputTransform(OutputTransformNode {
                    transform_id: "aces-2-rec709".into(),
                }),
            ),
            display(),
        ]);
        assert_eq!(pipeline.validate(), Ok(()));
    }

    #[test]
    fn rejects_film_development_after_digital_sensor() {
        let pipeline = base(vec![
            source(),
            camera(),
            lens(),
            node(
                "sensor",
                NodeRole::Observed,
                ImagingOperation::CaptureMedium(CaptureMediumNode {
                    medium: CaptureMedium::DigitalSensor {
                        profile_id: "sensor".into(),
                        iso: 100.0,
                        bit_depth: 12,
                    },
                }),
            ),
            node(
                "wrong-development",
                NodeRole::Transform,
                ImagingOperation::Development(DevelopmentNode {
                    process: DevelopmentProcess::Chemical {
                        process_id: "ecn2".into(),
                        push_pull_stops: 0.0,
                    },
                }),
            ),
            display(),
        ]);
        assert!(matches!(
            pipeline.validate(),
            Err(PipelineError::DomainMismatch { .. })
        ));
    }

    #[test]
    fn disabled_nodes_are_excluded_from_domain_flow() {
        let mut skipped_lens = lens();
        skipped_lens.enabled = false;
        let pipeline = base(vec![source(), skipped_lens, display()]);
        assert!(matches!(
            pipeline.validate(),
            Err(PipelineError::DomainMismatch { .. })
        ));
    }

    #[test]
    fn bundled_json_examples_match_the_contract() {
        let examples = [
            include_str!("../../../examples/pipelines/digital-reference.json"),
            include_str!("../../../examples/pipelines/film-reference.json"),
        ];
        for json in examples {
            let pipeline: ImagingPipeline = serde_json::from_str(json).unwrap();
            pipeline.validate().unwrap();
        }
    }
}
