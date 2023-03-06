use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Default, Clone)]
pub struct ProjectModel<'a> {
    pub name: &'a str,
    pub authors: Vec<&'a str>, // I don't like this, references are always better on the outer container
    pub compilation_db: bool,
}
