use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct BuildModel<'a> {
    #[serde(borrow = "'a")]
    pub output_dir: &'a Path,
}

