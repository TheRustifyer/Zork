use crate::{
    bounds::{ExecutableTarget, ExtraArgs},
    cli::output::arguments::Argument,
};
use std::path::Path;
use serde::{Deserialize, Serialize};

use super::sourceset::SourceSet;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct TestsModel<'a> {
    pub test_executable_name: String,
    #[serde(borrow = "'a")]
    pub sourceset: SourceSet<'a>,
    #[serde(borrow = "'a")] pub main: &'a Path,
    pub extra_args: Vec<Argument<'a>>,
}

impl<'a> ExtraArgs<'a> for TestsModel<'a> {
    fn extra_args(&'a self) -> &'a [Argument<'a>] {
        &self.extra_args
    }
}

impl<'a> ExecutableTarget<'a> for TestsModel<'a> {
    fn name(&'a self) -> &'a str {
        &self.test_executable_name
    }
    fn sourceset(&'a self) -> &'a SourceSet<'a> {
        &self.sourceset
    }
}
