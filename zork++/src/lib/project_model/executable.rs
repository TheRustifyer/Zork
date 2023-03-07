use crate::{
    bounds::{ExecutableTarget, ExtraArgs},
    cli::output::arguments::Argument,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::cli::output::arguments::ArgumentOwned;
use super::sourceset::SourceSet;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct ExecutableModel<'a> {
    pub executable_name: &'a str,
    #[serde(borrow = "'a")]
    pub sourceset: SourceSet<'a>,
    #[serde(borrow = "'a")] pub main: &'a Path,
    pub extra_args: Vec<Argument<'a>>,
}

impl<'a> ExtraArgs<'a> for ExecutableModel<'a> {
    fn extra_args(&'a self) -> &'a [Argument<'a>] {
        &self.extra_args
    }
}

impl<'a> ExecutableTarget<'a> for ExecutableModel<'a> {
    fn name(&'a self) -> &'a str {
        self.executable_name
    }
    fn sourceset(&'a self) -> &'a SourceSet<'a> {
        &self.sourceset
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct ExecutableModelOwned {
    pub executable_name: String,
    pub sourceset: SourceSetOwned,
    pub main: PathBuf,
    pub extra_args: Vec<ArgumentOwned>,
}
