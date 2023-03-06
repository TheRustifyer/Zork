use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Default, Clone)]
pub struct BuildModel<'a> {
    pub output_dir: &'a str,
}

impl<'a> BuildModel<'a> {
    pub fn to_path(&self) -> &Path {
        Path::new(self.output_dir)
    }
}
