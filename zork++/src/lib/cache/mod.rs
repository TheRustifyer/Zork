//! The implementation of the Zork++ cache, for persisting data in between process

use chrono::{DateTime, Utc};
use color_eyre::{eyre::Context, Result};
use std::{
    fs,
    fs::File,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use crate::utils::constants::COMPILATION_DATABASE;
use crate::{cli::{
    input::CliArgs,
    output::commands::{CommandExecutionResult, Commands, ModuleCommandLine},
}, project_model::{compiler::CppCompiler, ZorkModel}, project_model, utils::{
    self,
    constants::{self, GCC_CACHE_DIR},
}};
use serde::{Deserialize, Serialize};
use crate::cli::output::arguments::Argument;
use crate::config_file::ZorkConfigFile;
use crate::project_model::project::ProjectModel;
use crate::project_model::sourceset::SourceSet;

/// Standalone utility for retrieve the Zork++ cache file
pub fn load<'a>(config: &ZorkConfigFile<'_>, cli_args: &CliArgs) -> Result<ZorkCache<'a>> {
    let compiler = project_model::compiler::CppCompiler::from(
        &config.compiler.cpp_compiler
    );
    let out_dir = Path::new(
        config
            .build
            .as_ref()
            .and_then(|build_attr| build_attr.output_dir)
            .unwrap_or_default()
    );
    let cache_path = &Path::new(out_dir)
        .join("zork")
        .join("cache")
        .join(compiler.as_ref());

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

    cache.run_tasks(compiler, out_dir, config);

    Ok(cache)
}

/// Standalone utility for persist the cache to the file system
pub fn save(
    program_data: &ZorkModel<'_>,
    mut cache: ZorkCache,
    commands: Commands<'_>,
    test_mode: bool,
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
pub struct ZorkCache<'a> {
    pub last_program_execution: DateTime<Utc>,
    pub compilers_metadata: CompilersMetadata,
    #[serde(borrow = "'a")] pub last_generated_project_model: ProjectModel<'a>,
    pub generated_commands: CachedCommands,
}

impl<'a> ZorkCache<'a> {
    /// Returns a [`Option`] of [`CommandDetails`] if the file is persisted already in the cache
    pub fn is_file_cached(&self, path: &Path) -> Option<&CommandDetail> {
        let last_iteration_details = self.generated_commands.details.last();

        if let Some(last_iteration) = last_iteration_details {
            let found_as_ifc = last_iteration.interfaces.iter().find(|f| {
                path.to_str()
                    .unwrap_or_default()
                    .contains(&f.translation_unit)
            });

            if found_as_ifc.is_some() {
                return found_as_ifc;
            } else {
                let found_as_impl = last_iteration
                    .implementations
                    .iter()
                    .find(|f| path.to_str().unwrap_or_default().eq(&f.translation_unit));

                if found_as_impl.is_some() {
                    return found_as_impl;
                }
            }
        }
        None
    }

    /// The tasks associated with the cache after load it from the file system
    pub fn run_tasks(
        &mut self,
        compiler: CppCompiler,
        out_dir: &Path,
        config: &ZorkConfigFile<'_>
    ) {
        if cfg!(target_os = "windows") && compiler == CppCompiler::MSVC {
            self.load_msvc_metadata()
        }
        if compiler != CppCompiler::MSVC {
            let i = Self::track_system_modules(compiler, out_dir, config);
            self.compilers_metadata.system_modules.clear();
            self.compilers_metadata.system_modules.extend(i);
        }
    }

    /// Runs the tasks just before end the program and save the cache
    pub fn run_final_tasks(
        &mut self,
        program_data: &ZorkModel<'_>,
        commands: Commands<'_>,
        test_mode: bool,
    ) -> Result<()> {
        self.save_generated_commands_and_execution_status(&commands);
        if program_data.project.compilation_db {
            map_generated_commands_to_compilation_db(program_data, &commands, test_mode)?;
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

    fn save_generated_commands_and_execution_status(&mut self, commands: &Commands<'_>) {
        log::trace!(
            "Storing in the cache the last generated command lines and execution results..."
        );
        self.generated_commands.compiler = commands.compiler;
        let process_no = if !self.generated_commands.details.is_empty() {
            self.generated_commands
                .details
                .last()
                .unwrap()
                .cached_process_num
                + 1
        } else {
            1
        };

        let mut commands_details = CommandsDetails {
            cached_process_num: process_no,
            generated_at: Utc::now(),
            interfaces: Vec::with_capacity(commands.interfaces.len()),
            implementations: Vec::with_capacity(commands.implementations.len()),
            main: MainCommandLineDetail::default(),
        };

        commands_details
            .interfaces
            .extend(
                commands
                    .interfaces
                    .iter()
                    .map(|module_command_line| CommandDetail {
                        translation_unit: self.set_translation_unit_identifier(module_command_line),
                        execution_result: self
                            .normalize_execution_result_status(module_command_line),
                        command: self.set_module_generated_command_line(module_command_line),
                    }),
            );

        commands_details
            .implementations
            .extend(
                commands
                    .implementations
                    .iter()
                    .map(|module_command_line| CommandDetail {
                        translation_unit: self.set_translation_unit_identifier(module_command_line),
                        execution_result: self
                            .normalize_execution_result_status(module_command_line),
                        command: self.set_module_generated_command_line(module_command_line),
                    }),
            );

        commands_details.main = MainCommandLineDetail {
            files: commands.sources.sources_paths.clone(),
            execution_result: commands.sources.execution_result.clone(),
            command: commands
                .sources
                .args
                .iter()
                .map(|arg| arg.value.to_string())
                .collect::<Vec<_>>()
                .join(" "),
        };

        self.generated_commands.details.push(commands_details)
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
    fn track_system_modules<'b>(
        compiler: CppCompiler,
        out_dir: &Path,
        config: &'b ZorkConfigFile<'b>
    ) -> impl Iterator<Item = String> + 'b {
        let root = if compiler == CppCompiler::GCC {
            Path::new(GCC_CACHE_DIR).to_path_buf()
        } else {
            out_dir
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
                    config
                        .modules
                        .as_ref()
                        .and_then(|mods| mods.sys_modules.as_ref())
                        .unwrap_or(&Vec::with_capacity(0))
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
        module_command_line: &ModuleCommandLine,
    ) -> CommandExecutionResult {
        if module_command_line
            .execution_result
            .eq(&CommandExecutionResult::Unreached)
        {
            if let Some(prev_entry) = self.is_file_cached(&module_command_line.path) {
                prev_entry.execution_result.clone()
            } else {
                module_command_line.execution_result.clone()
            }
        } else {
            module_command_line.execution_result.clone()
        }
    }

    fn set_module_generated_command_line(&self, module_command_line: &ModuleCommandLine) -> String {
        if module_command_line.processed {
            String::with_capacity(0)
        } else {
            module_command_line
                .args
                .iter()
                .map(|argument| argument.value)
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    fn set_translation_unit_identifier(&self, module_command_line: &ModuleCommandLine) -> String {
        String::from(
            module_command_line
                .path
                .as_os_str()
                .to_str()
                .unwrap_or_default(),
        )
    }
}

/// Generates the `compile_commands.json` file, that acts as a compilation database
/// for some static analysis external tools, like `clang-tidy`, and populates it with
/// the generated commands for the translation units
fn map_generated_commands_to_compilation_db(
    program_data: &ZorkModel,
    commands: &Commands<'_>,
    test_mode: bool,
) -> Result<()> {
    log::trace!("Generating the compilation database...");
    let total_commands = commands.interfaces.len() + commands.implementations.len() + 1;
    let mut compilation_db_entries = Vec::with_capacity(total_commands);

    for command in &commands.interfaces {
        let path = fs::canonicalize(command.path.parent().unwrap_or(Path::new("")))
            .map(|p| String::from(p.to_str().unwrap_or_default()))
            .unwrap_or_default();

        let mut arguments = vec![commands.compiler.get_driver()];
        arguments.extend(
            command
                .args
                .iter()
                .map(|arg| arg.value)
                .collect::<Vec<&str>>(),
        );
        let file = command
            .path
            .file_name()
            .map_or("", |f| f.to_str().unwrap_or_default());

        compilation_db_entries.push(CompileCommands {
            directory: path,
            arguments,
            file,
        })
    }

    for command in &commands.implementations {
        let path = fs::canonicalize(command.path.parent().unwrap_or(Path::new("")))
            .map(|p| String::from(p.to_str().unwrap_or_default()))
            .unwrap_or_default();

        let mut arguments = vec![commands.compiler.get_driver()];
        arguments.extend(
            command
                .args
                .iter()
                .map(|arg| arg.value)
                .collect::<Vec<&str>>(),
        );
        let file = command
            .path
            .file_name()
            .map_or("", |f| f.to_str().unwrap_or_default());

        compilation_db_entries.push(CompileCommands {
            directory: path,
            arguments,
            file,
        })
    }

    // generated command for the binary (exe or tests exe)
    let entry_point = if !test_mode {
        Path::new(".").join(program_data.executable.main)
    } else {
        Path::new(".").join(program_data.tests.main)
    };

    let mut main_arguments = vec![commands.compiler.get_driver()];
    main_arguments.extend(
        commands
            .sources
            .args
            .iter()
            .map(|arg| arg.value)
            .collect::<Vec<&str>>(),
    );
    compilation_db_entries.push(CompileCommands {
        directory: fs::canonicalize(entry_point.parent().unwrap_or(Path::new(".")))
            .map(|p| String::from(p.to_str().unwrap_or_default()))
            .unwrap_or_default(),
        arguments: main_arguments,
        file: entry_point
            .file_name()
            .map_or("", |f| f.to_str().unwrap_or_default()),
    });

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
pub struct CompileCommands<'a> {
    pub directory: String,
    pub arguments: Vec<&'a str>,
    pub file: &'a str,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct CachedCommands {
    compiler: CppCompiler,
    details: Vec<CommandsDetails>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct CommandsDetails {
    cached_process_num: i32,
    generated_at: DateTime<Utc>,
    interfaces: Vec<CommandDetail>,
    implementations: Vec<CommandDetail>,
    main: MainCommandLineDetail,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct CommandDetail {
    translation_unit: String,
    pub execution_result: CommandExecutionResult,
    command: String,
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
