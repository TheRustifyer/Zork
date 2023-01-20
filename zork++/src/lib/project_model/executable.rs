#[derive(Debug, PartialEq, Eq)]
pub struct ExecutableModel {
    pub executable_name: String,
    pub sources_base_path: String,
    pub sources: Vec<String>,
    pub extra_args: Vec<String>,
}
