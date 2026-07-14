#![allow(dead_code)]

use crate::document::ColorConfig;

pub struct ColorManager {
    pub config: ColorConfig,
}

impl ColorManager {
    pub fn new() -> Self {
        Self {
            config: ColorConfig::default(),
        }
    }

    pub fn new_with_config(config: ColorConfig) -> Self {
        Self { config }
    }

    pub fn transform_pixels(&self, _data: &mut [u8], _src_profile: Option<&[u8]>) {
        if let Some(embedded) = &self.config.embedded_profile {
            if let Some(embedded) = embedded.as_slice().first() {
                let _ = embedded;
            }
        }
    }
}

impl Default for ColorManager {
    fn default() -> Self {
        Self::new()
    }
}
