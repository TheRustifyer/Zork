//! The implementation of the Zork++ cache, for persisting data in between process

use chrono::{DateTime, Utc};
use color_eyre::{eyre::Context, Result};
use std::collections::HashMap;
use std::{
    fs,
    fs::File,
    path::{Path, PathBuf},
};

use crate::utils::constants::COMPILATION_DATABASE;
use crate::{
    cli::{
        input::CliArgs,
        output::commands::{CommandExecutionResult, Commands, SourceCommandLine},
    },
    project_model::{compiler::CppCompiler, ZorkModel},
    utils::{
        self,
        constants::{self, GCC_CACHE_DIR},
    },
};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

/// Standalone utility for retrieve the Zork++ cache file
pub fn load(program_data: &ZorkModel<'_>, cli_args: &CliArgs) -> Result<ZorkCache> {
    let compiler = program_data.compiler.cpp_compiler.as_ref();
    let cache_path = &Path::new(program_data.build.output_dir)
        .join("zork")
        .join("cache")
        .join(compiler);

    let cache_file_path = cache_path.join(constants::ZORK_CACHE_FILENAME);

    if !Path::new(&cache_file_path).exists() {
        File::create(cache_file_path).with_context(|| "Error creating the cache file")?;
    } else if Path::new(cache_path).exists() && cli_args.clear_cache {
        fs::remove_dir_all(cache_path).with_context(|| "Error cleaning the Zork++ cache")?;
        fs::create_dir(cache_path)
            .with_context(|| "Error creating the cache subdir for {compiler}")?;
        File::create(cache_file_path)
            .with_context(|| "Error creating the cache file after cleaning the cache")?;
    }

    let mut cache: ZorkCache = utils::fs::load_and_deserialize(&cache_path)
        .with_context(|| "Error loading the Zork++ cache")?;

    cache.run_tasks(program_data);

    Ok(cache)
}

/// Standalone utility for persist the cache to the file system
pub fn save(
    program_data: &ZorkModel<'_>,
    mut cache: ZorkCache,
    commands: Commands<'_>,
    test_mode: bool
) -> Result<()> {
    let cache_path = &Path::new(program_data.build.output_dir)
        .join("zork")
        .join("cache")
        .join(program_data.compiler.cpp_compiler.as_ref())
        .join(constants::ZORK_CACHE_FILENAME);

    cache.run_final_tasks(program_data, commands, test_mode)?;
    cache.last_program_execution = Utc::now();

    utils::fs::serialize_object_to_file(cache_path, &cache)
        .with_context(move || "Error saving data to the Zork++ cache")
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ZorkCache {
    pub compiler: CppCompiler,
    pub last_program_execution: DateTime<Utc>,
    pub compilers_metadata: CompilersMetadata,
    pub generated_commands: Vec<CommandsDetails>,
    pub last_generated_commands: HashMap<PathBuf, Vec<String>>,
}

impl ZorkCache {
    /// Returns a [`Option`] of [`CommandDetails`] if the file is persisted already in the cache
    pub fn is_file_cached(&self, path: impl AsRef<Path>) -> Option<&CommandDetail> {
        let last_iteration_details = self.generated_commands.last();

        if let Some(last_iteration) = last_iteration_details {
            return last_iteration
                .interfaces
                .iter()
                .chain(last_iteration.implementations.iter())
                .chain(last_iteration.sources.iter())
                .find(|comm_det| comm_det.file_path().eq(path.as_ref()));
        }
        None
    }

    /// The tasks associated with the cache after load it from the file system
    pub fn run_tasks(&mut self, program_data: &ZorkModel<'_>) {
        let compiler = program_data.compiler.cpp_compiler;
        if cfg!(target_os = "windows") && compiler == CppCompiler::MSVC {
            self.load_msvc_metadata()
        }
        if compiler != CppCompiler::MSVC {
            let i = Self::track_system_modules(program_data);
            self.compilers_metadata.system_modules.clear();
            self.compilers_metadata.system_modules.extend(i);
        }
    }

    /// Runs the tasks just before end the program and save the cache
    pub fn run_final_tasks(
        &mut self,
        program_data: &ZorkModel<'_>,
        mut commands: Commands<'_>,
        test_mode: bool
    ) -> Result<()> {
        if self.save_generated_commands(&mut commands, test_mode) && program_data.project.compilation_db {
            map_generated_commands_to_compilation_db(self)?;
        }

        if !(program_data.compiler.cpp_compiler == CppCompiler::MSVC) {
            self.compilers_metadata.system_modules = program_data
                .modules
                .sys_modules
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>();
        }

        Ok(())
    }

    fn save_generated_commands(&mut self, commands: &mut Commands<'_>, test_mode: bool) -> bool {
        log::trace!("Storing in the cache the last generated command lines...");
        let mut has_changes = false;

        self.compiler = commands.compiler;
        let process_no = if !self.generated_commands.is_empty() {
            self.generated_commands.last().unwrap().cached_process_num + 1
        } else {
            1
        };

        let mut commands_details = CommandsDetails {
            cached_process_num: process_no,
            generated_at: Utc::now(),
            interfaces: Vec::with_capacity(commands.interfaces.len()),
            implementations: Vec::with_capacity(commands.implementations.len()),
            sources: Vec::with_capacity(commands.sources.len()),
            main: MainCommandLineDetail::default(),
        };

        let mut are_new_commands = Vec::with_capacity(3);
        let interfaces_has_new_commands=
            self.extend_collection_of_source_file_details(&mut commands_details.interfaces, &mut commands.interfaces, commands.compiler);
        are_new_commands.push(interfaces_has_new_commands);
        let implementations_has_new_commands =
            self.extend_collection_of_source_file_details(&mut commands_details.implementations, &mut commands.implementations, commands.compiler);
        are_new_commands.push(implementations_has_new_commands);
        let sources_has_new_commands =
            self.extend_collection_of_source_file_details(&mut commands_details.sources, &mut commands.sources, commands.compiler);
        are_new_commands.push(sources_has_new_commands);

        commands_details.main = MainCommandLineDetail {
            files: commands.main.sources_paths.clone(),
            execution_result: commands.main.execution_result,
            command: commands
                .main
                .args
                .iter()
                .map(|arg| arg.value.to_string())
                .collect::<Vec<_>>()
                .join(" "),
        };

        let named_target = if test_mode { "test_main" } else { "main" };
        self.last_generated_commands
            .entry(PathBuf::from(named_target)) // provisional
            .or_insert_with(|| {
                has_changes = true;
                vec![commands_details.main.command.clone()]
            });

        self.generated_commands.push(commands_details);

        are_new_commands.iter().any(|b| *b)
    }

    /// If Windows is the current OS, and the compiler is MSVC, then we will try
    /// to locate the path os the vcvars64.bat scripts that launches the
    /// Developers Command Prompt
    fn load_msvc_metadata(&mut self) {
        if self.compilers_metadata.msvc.dev_commands_prompt.is_none() {
            self.compilers_metadata.msvc.dev_commands_prompt =
                WalkDir::new(constants::MSVC_BASE_PATH)
                    .into_iter()
                    .filter_map(Result::ok)
                    .find(|file| {
                        file.file_name()
                            .to_str()
                            .map(|filename| filename.eq(constants::MS_DEVS_PROMPT_BAT))
                            .unwrap_or(false)
                    })
                    .map(|e| e.path().display().to_string());
        }
    }

    /// Looks for the already precompiled `GCC` or `Clang` system headers,
    /// to avoid recompiling them on every process
    fn track_system_modules<'a>(
        program_data: &'a ZorkModel<'_>,
    ) -> impl Iterator<Item = String> + 'a {
        let root = if program_data.compiler.cpp_compiler == CppCompiler::GCC {
            Path::new(GCC_CACHE_DIR).to_path_buf()
        } else {
            program_data
                .build
                .output_dir
                .join("clang")
                .join("modules")
                .join("interfaces")
        };

        WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|file| {
                if file
                    .metadata()
                    .expect("Error retrieving metadata")
                    .is_file()
                {
                    program_data
                        .modules
                        .sys_modules
                        .iter()
                        .any(|sys_mod| file.file_name().to_str().unwrap().starts_with(sys_mod))
                } else {
                    false
                }
            })
            .map(|dir_entry| {
                dir_entry
                    .file_name()
                    .to_str()
                    .unwrap()
                    .split('.')
                    .collect::<Vec<_>>()[0]
                    .to_string()
            })
    }

    fn normalize_execution_result_status(
        &self,
        module_command_line: &SourceCommandLine,
    ) -> CommandExecutionResult {
        if module_command_line
            .execution_result
            .eq(&CommandExecutionResult::Unreached)
        {
            if let Some(prev_entry) = self.is_file_cached(module_command_line.path()) {
                prev_entry.execution_result
            } else {
                module_command_line.execution_result
            }
        } else {
            module_command_line.execution_result
        }
    }

    fn extend_collection_of_source_file_details(
        &mut self,
        collection: &mut Vec<CommandDetail>,
        target: &mut [SourceCommandLine],
        compiler: CppCompiler
    ) -> bool {
        let mut new_commands = false;
        collection.extend(target.iter().map(|module_command_line| {
            self.last_generated_commands
                .entry(module_command_line.path())
                .or_insert_with(|| {
                    new_commands = true;
                    let mut arguments = Vec::with_capacity(module_command_line.args.len() + 1);
                    arguments.push(compiler.get_driver().to_string());
                    arguments.extend(module_command_line
                        .args
                        .iter()
                        .map(|e| e.value.to_string())
                    );
                    arguments
                });
            CommandDetail {
                directory: module_command_line
                    .directory
                    .to_str()
                    .unwrap_or_default()
                    .to_string(),
                file: module_command_line.file.clone(),
                execution_result: self.normalize_execution_result_status(module_command_line),
            }
        }));

        new_commands
    }
}

/// Generates the `compile_commands.json` file, that acts as a compilation database
/// for some static analysis external tools, like `clang-tidy`, and populates it with
/// the generated commands for the translation units
fn map_generated_commands_to_compilation_db(cache: &ZorkCache) -> Result<()> {
    log::trace!("Generating the compilation database...");
    let mut compilation_db_entries = Vec::with_capacity(cache.last_generated_commands.len());

    for command in cache.last_generated_commands.iter() {
        compilation_db_entries.push(CompileCommands::from(command));
    }

    let compile_commands_path = Path::new(COMPILATION_DATABASE);
    if !Path::new(&compile_commands_path).exists() {
        File::create(compile_commands_path).with_context(|| "Error creating the cache file")?;
    }
    utils::fs::serialize_object_to_file(Path::new(compile_commands_path), &compilation_db_entries)
        .with_context(move || "Error generating the compilation database")
}

/// Data model for serialize the data that will be outputted
/// to the `compile_commands.json` compilation database file
#[derive(Serialize, Debug, Default, Clone)]
pub struct CompileCommands {
    pub directory: String,
    pub file: String,
    pub arguments: Vec<String>,
}

impl From<(&'_ PathBuf, &'_ Vec<String>)> for CompileCommands {
    fn from(value: (&PathBuf, &Vec<String>)) -> Self {
        let dir = value.0.parent().unwrap_or(Path::new("."));
        let mut file = value.0.file_stem().unwrap_or_default().to_os_string();
        file.push(".");
        file.push(value.0.extension().unwrap_or_default());

        Self {
            directory: dir.to_str().unwrap_or_default().to_string(),
            file: file.to_str().unwrap_or_default().to_string(),
            arguments: value.1.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct CommandsDetails {
    cached_process_num: i32,
    generated_at: DateTime<Utc>,
    interfaces: Vec<CommandDetail>,
    implementations: Vec<CommandDetail>,
    sources: Vec<CommandDetail>,
    main: MainCommandLineDetail,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct CommandDetail {
    directory: String,
    file: String,
    pub execution_result: CommandExecutionResult,
}

impl CommandDetail {
    #[inline(always)]
    pub fn file_path(&self) -> PathBuf {
        Path::new(&self.directory).join(&self.file)
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct MainCommandLineDetail {
    files: Vec<PathBuf>,
    execution_result: CommandExecutionResult,
    command: String,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct CompilersMetadata {
    pub msvc: MsvcMetadata,
    pub system_modules: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct MsvcMetadata {
    pub dev_commands_prompt: Option<String>,
}
