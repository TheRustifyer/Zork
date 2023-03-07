use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct BuildModel<'a> {
    #[serde(borrow = "'a")]
    pub output_dir: &'a Path,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct BuildModelOwned {
    pub output_dir: PathBuf,
}

