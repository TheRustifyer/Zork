use crate::{
    bounds::{ExecutableTarget, ExtraArgs},
    cli::output::arguments::Argument,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use super::sourceset::SourceSet;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Default, Clone)]
pub struct ExecutableModel<'a> {
    pub executable_name: &'a str,
    #[serde(borrow = "'a")]
    pub sourceset: SourceSet<'a>,
    pub main: &'a Path,
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
