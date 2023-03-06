use std::path::{Path, PathBuf};

use color_eyre::{eyre::Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::output::arguments::Argument;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub enum Source<'a> {
    #[serde(borrow = "'a")]
    File(&'a str),
    Glob(GlobPattern<'a>),
}

impl<'a> Source<'a> {
    #[inline(always)]
    pub fn paths(&self) -> Result<Vec<PathBuf>> {
        match self {
            Source::File(file) => Ok(vec![PathBuf::from(file)]),
            Source::Glob(pattern) => pattern.resolve(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Default, Clone)]
pub struct GlobPattern<'a>(pub &'a str);

impl<'a> GlobPattern<'a> {
    #[inline(always)]
    fn resolve(&self) -> Result<Vec<PathBuf>> {
        glob::glob(self.0)?
            .map(|path| path.with_context(|| ""))
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Default, Clone)]
pub struct SourceSet<'a> {
    #[serde(borrow = "'a")]
    pub base_path: &'a str,
    pub sources: Vec<Source<'a>>,
}

impl<'a> SourceSet<'a> {
    pub fn as_args_to(&'a self, dst: &mut Vec<Argument<'a>>) -> Result<()> {
        let paths: Result<Vec<Vec<PathBuf>>> = self.sources.iter().map(Source::paths).collect();

        let paths = paths?
            .into_iter()
            .flatten()
            .map(|path| Path::new(self.base_path).join(path))
            .map(Argument::from);

        dst.extend(paths);

        Ok(())
    }
}
