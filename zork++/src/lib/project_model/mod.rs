pub mod build;
pub mod compiler;
pub mod executable;
pub mod modules;
pub mod project;
pub mod sourceset;
pub mod tests;

use std::fmt::Debug;
use serde::{Deserialize, Serialize};

use self::{
    build::BuildModel, compiler::CompilerModel, executable::ExecutableModel, modules::ModulesModel,
    project::ProjectModel, tests::TestsModel,
};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct ZorkModel<'a> {
    #[serde(borrow = "'a")]
    pub project: ProjectModel<'a>,
    #[serde(borrow = "'a")] pub compiler: CompilerModel<'a>,
    #[serde(borrow = "'a")] pub build: BuildModel<'a>,
    #[serde(borrow = "'a")] pub executable: ExecutableModel<'a>,
    #[serde(borrow = "'a")] pub modules: ModulesModel<'a>,
    #[serde(borrow = "'a")] pub tests: TestsModel<'a>,
}
