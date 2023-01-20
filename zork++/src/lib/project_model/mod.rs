pub mod build;
pub mod compiler;
pub mod executable;
pub mod modules;
pub mod project;
pub mod tests;

use std::fmt::{Debug, Display};

use self::{
    build::BuildModel, compiler::CompilerModel, executable::ExecutableModel, modules::ModulesModel,
    project::ProjectModel, tests::TestsModel,
};

#[derive(Debug, PartialEq, Eq)]
pub struct ZorkModel {
    pub project: ProjectModel,
    pub compiler: CompilerModel,
    pub build: BuildModel,
    pub executable: ExecutableModel,
    pub modules: ModulesModel,
    pub tests: TestsModel,
}

/// Represents any kind of translation unit and the generic operations
/// applicable to all the implementors
pub trait TranslationUnit: Display + Debug {
    /// Outputs the declared filename for `self` being the translation unit
    fn get_filename(&self) -> String;
}

impl TranslationUnit for &str {
    fn get_filename(&self) -> String {
        self.to_string()
    }
}

impl TranslationUnit for String {
    fn get_filename(&self) -> String {
        self.clone()
    }
}
