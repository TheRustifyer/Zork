pub mod build;
pub mod compiler;
pub mod executable;
pub mod modules;
pub mod project;
pub mod sourceset;
pub mod tests;

use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use crate::project_model::build::BuildModelOwned;
use crate::project_model::compiler::CompilerModelOwned;
use crate::project_model::executable::ExecutableModelOwned;
use crate::project_model::modules::ModulesModelOwned;
use crate::project_model::project::ProjectModelOwned;
use crate::project_model::tests::TestsModelOwned;

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

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct ZorkModelOwned {
    pub project: ProjectModelOwned,
    pub compiler: CompilerModelOwned,
    pub build: BuildModelOwned,
    pub executable: ExecutableModelOwned,
    pub modules: ModulesModelOwned,
    pub tests: TestsModelOwned,
}
