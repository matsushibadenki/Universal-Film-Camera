//! UI- and camera-independent film processing contracts.

use media_core::{FrameDescriptor, WorkingColorSpace};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityLevel {
    Preview,
    Realtime,
    High,
    Reference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmRecipe {
    pub profile_id: String,
    pub exposure_ev: f32,
    pub halation_amount: f32,
    pub grain_amount: f32,
    pub seed: u64,
}

#[derive(Debug, Clone)]
pub struct LinearImage {
    pub descriptor: FrameDescriptor,
    pub pixels: Vec<[f32; 4]>,
}

impl LinearImage {
    pub fn validate(&self) -> Result<(), FilmError> {
        if self.descriptor.color_space != WorkingColorSpace::AcesCg {
            return Err(FilmError::UnsupportedWorkingSpace);
        }
        let expected = self.descriptor.width as usize * self.descriptor.height as usize;
        if self.pixels.len() != expected {
            return Err(FilmError::InvalidBufferLength);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilmError {
    InvalidBufferLength,
    UnsupportedWorkingSpace,
    ProfileNotLoaded,
}

pub trait FilmEngine: Send {
    fn set_recipe(&mut self, recipe: FilmRecipe) -> Result<(), FilmError>;
    fn process_image(
        &mut self,
        image: &mut LinearImage,
        quality: QualityLevel,
    ) -> Result<(), FilmError>;
}
