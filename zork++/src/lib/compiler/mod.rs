//! The crate responsible for executing the core work of `Zork++`,
// generate command lines and execute them in a shell of the current
// operating system against the designed compilers in the configuration
// file.

use color_eyre::Result;
use std::path::Path;

use crate::bounds::TranslationUnit;
use crate::cli::output::commands::{CommandExecutionResult, ModuleCommandLine};
use crate::compiler::helpers::flag_modules_without_changes;
use crate::{
    cache::ZorkCache,
    cli::output::{arguments::Argument, commands::Commands},
    project_model::{
        compiler::CppCompiler,
        modules::{ModuleImplementationModel, ModuleInterfaceModel},
        ZorkModel,
    },
};

/// The entry point of the compilation process
///
/// Whenever this process gets triggered, the files declared within the
/// configuration file will be build
pub fn build_project<'a>(
    model: &'a ZorkModel<'a>,
    cache: &'a ZorkCache,
    tests: bool,
) -> Result<Commands<'a>> {
    // A registry of the generated command lines
    let mut commands = Commands::new(&model.compiler.cpp_compiler);

    if model.compiler.cpp_compiler != CppCompiler::MSVC {
        helpers::prebuild_sys_modules(model, &mut commands, cache)
    }

    // 1st - Build the modules
    build_modules(model, cache, &mut commands)?;
    // 2st - Build the executable or the tests
    build_executable(model, &mut commands, tests)?;

    Ok(commands)
}

/// Triggers the build process for compile the source files declared for the project
/// If this flow is enabled by the Cli arg `Tests`, then the executable will be generated
/// for the files and properties declared for the tests section in the configuration file
fn build_executable<'a>(
    model: &'a ZorkModel<'_>,
    commands: &'_ mut Commands<'a>,
    tests: bool,
) -> Result<()> {
    // TODO Check if the command line is the same as the previous? If there's no new sources?
    // And avoid reexecuting?
    if tests {
        sources::generate_main_command_line_args(model, commands, &model.tests)
    } else {
        sources::generate_main_command_line_args(model, commands, &model.executable)
    }
}

/// Triggers the build process for compile the declared modules in the project
///
/// This function acts like a operation result processor, by running instances
/// and parsing the obtained result, handling the flux according to the
/// compiler responses>
fn build_modules<'a>(
    model: &'a ZorkModel,
    cache: &'a ZorkCache,
    commands: &mut Commands<'a>,
) -> Result<()> {
    log::info!("Building the module interfaces and partitions...");
    prebuild_module_interfaces(model, cache, &model.modules.interfaces, commands);

    log::info!("Building the module implementations...");
    compile_module_implementations(model, cache, &model.modules.implementations, commands);

    Ok(())
}

/// Parses the configuration in order to build the BMIs declared for the project,
/// by precompiling the module interface units
fn prebuild_module_interfaces<'a>(
    model: &'a ZorkModel<'_>,
    cache: &'a ZorkCache,
    interfaces: &'a [ModuleInterfaceModel],
    commands: &mut Commands<'a>,
) {
    interfaces.iter().for_each(|module_interface| {
        if !flag_modules_without_changes(&model.compiler.cpp_compiler, cache, &module_interface.file()) {
            sources::generate_module_interfaces_args(model, module_interface, commands);
        } else {
            let mut command_line = ModuleCommandLine {
                path: module_interface.file(),
                args: Vec::with_capacity(0),
                processed: true,
                execution_result: CommandExecutionResult::default(),
            };
            command_line.execution_result = CommandExecutionResult::Cached;
            log::trace!("Translation unit: {:?} was not modified since the last iteration. No need to rebuilt it again.", &module_interface.file());
            commands.interfaces.push(command_line);
            commands.generated_files_paths.push(Argument::from(helpers::generate_prebuild_miu(
                model.compiler.cpp_compiler, model.build.output_dir, module_interface
            )))
        }
    });
}

/// Parses the configuration in order to compile the module implementation
/// translation units declared for the project
fn compile_module_implementations<'a>(
    model: &'a ZorkModel,
    cache: &'a ZorkCache,
    impls: &'a [ModuleImplementationModel],
    commands: &mut Commands<'a>,
) {
    impls.iter().for_each(|module_impl| {
        if !flag_modules_without_changes(&model.compiler.cpp_compiler, cache, &module_impl.file()) {
            sources::generate_module_implementation_args(model, module_impl, commands);
        } else {
            let mut command_line = ModuleCommandLine {
                path: module_impl.file(),
                args: Vec::with_capacity(0),
                processed: true,
                execution_result: CommandExecutionResult::default(),
            };
            command_line.execution_result = CommandExecutionResult::Cached;
            log::trace!("Translation unit: {:?} was not modified since the last iteration. No need to rebuilt it again.", &module_impl.file());
            commands.interfaces.push(command_line);
            commands.generated_files_paths.push(Argument::from(helpers::generate_impl_obj_file(
                model.compiler.cpp_compiler, model.build.output_dir, module_impl
            )))
        }
    });
}

/// Specific operations over source files
mod sources {
    use super::helpers;
    use crate::{
        bounds::{ExecutableTarget, TranslationUnit},
        cli::output::{
            arguments::{clang_args, Argument},
            commands::{CommandExecutionResult, Commands, ModuleCommandLine},
        },
        project_model::{
            compiler::CppCompiler,
            modules::{ModuleImplementationModel, ModuleInterfaceModel},
            ZorkModel,
        },
        utils::constants,
    };
    use color_eyre::Result;

    /// Generates the command line arguments for non-module source files, including the one that
    /// holds the main function
    pub fn generate_main_command_line_args<'a>(
        model: &'a ZorkModel,
        commands: &mut Commands<'a>,
        target: &'a impl ExecutableTarget<'a>,
    ) -> Result<()> {
        log::info!("Generating the main command line...");

        let compiler = &model.compiler.cpp_compiler;
        let out_dir = model.build.output_dir;
        let executable_name = target.name();

        let mut arguments = Vec::new();
        arguments.push(model.compiler.language_level_arg());

        match compiler {
            CppCompiler::CLANG => {
                arguments.extend(clang_args::add_std_lib(model));
                arguments.extend_from_slice(target.extra_args());
                arguments.push(Argument::from("-fimplicit-modules"));
                arguments.push(clang_args::implicit_module_maps(out_dir));

                arguments.push(Argument::from(format!(
                    "-fprebuilt-module-path={}",
                    out_dir
                        .join(compiler.as_ref())
                        .join("modules")
                        .join("interfaces")
                        .display()
                )));

                arguments.push(Argument::from("-o"));
                arguments.push(Argument::from(format!(
                    "{}",
                    out_dir
                        .join(compiler.as_ref())
                        .join(executable_name)
                        .with_extension(constants::BINARY_EXTENSION)
                        .display()
                )));

                arguments.extend(commands.generated_files_paths.clone().into_iter());
            }
            CppCompiler::MSVC => {
                arguments.push(Argument::from("/EHsc"));
                arguments.push(Argument::from("/nologo"));
                // If /std:c++20 this, else should be the direct options
                // available on C++23 to use directly import std by precompiling the standard library
                arguments.push(Argument::from("/experimental:module"));
                arguments.push(Argument::from("/stdIfcDir \"$(VC_IFCPath)\""));

                // helpers::add_extra_args_if_present(&config.executable, &mut arguments);
                arguments.extend_from_slice(target.extra_args());
                arguments.push(Argument::from("/ifcSearchDir"));
                arguments.push(Argument::from(
                    out_dir
                        .join(compiler.as_ref())
                        .join("modules")
                        .join("interfaces"),
                ));
                arguments.push(Argument::from(format!(
                    "/Fo{}\\",
                    out_dir.join(compiler.as_ref()).display()
                )));
                arguments.push(Argument::from(format!(
                    "/Fe{}",
                    out_dir
                        .join(compiler.as_ref())
                        .join(executable_name)
                        .with_extension(constants::BINARY_EXTENSION)
                        .display()
                )));
                arguments.extend(commands.generated_files_paths.clone().into_iter());
            }
            CppCompiler::GCC => {
                arguments.push(Argument::from("-fmodules-ts"));
                arguments.push(Argument::from("-o"));
                arguments.push(Argument::from(format!(
                    "{}",
                    out_dir
                        .join(compiler.as_ref())
                        .join(executable_name)
                        .with_extension(constants::BINARY_EXTENSION)
                        .display()
                )));
                arguments.extend(commands.generated_files_paths.clone().into_iter());
            }
        };

        target.sourceset().as_args_to(&mut arguments)?;
        commands.sources.args.extend(arguments.into_iter());
        commands.sources.sources_paths = target
            .sourceset()
            .sources
            .iter()
            .flat_map(|s| s.paths().unwrap_or_default())
            .collect::<Vec<_>>();

        Ok(())
    }

    /// Generates the expected arguments for precompile the BMIs depending on self
    pub fn generate_module_interfaces_args<'a>(
        model: &'a ZorkModel,
        interface: &'a ModuleInterfaceModel,
        commands: &mut Commands<'a>,
    ) {
        let compiler = model.compiler.cpp_compiler;
        let out_dir = model.build.output_dir;

        let mut arguments = Vec::with_capacity(8);
        arguments.push(model.compiler.language_level_arg());

        match compiler {
            CppCompiler::CLANG => {
                arguments.extend(clang_args::add_std_lib(model));
                arguments.push(Argument::from("-fimplicit-modules"));
                arguments.push(Argument::from("-x"));
                arguments.push(Argument::from("c++-module"));
                arguments.push(Argument::from("--precompile"));

                arguments.push(clang_args::implicit_module_maps(out_dir));

                arguments.push(Argument::from(format!(
                    "-fprebuilt-module-path={}/clang/modules/interfaces",
                    out_dir.display()
                )));
                clang_args::add_direct_module_interfaces_dependencies(
                    &interface.dependencies,
                    compiler,
                    out_dir,
                    &mut arguments,
                );

                // The resultant BMI as a .pcm file
                arguments.push(Argument::from("-o"));
                // The output file
                let miu_file_path =
                    Argument::from(helpers::generate_prebuild_miu(compiler, out_dir, interface));
                commands.generated_files_paths.push(miu_file_path.clone());
                arguments.push(miu_file_path);
                // The input file
                arguments.push(Argument::from(interface.file()));
            }
            CppCompiler::MSVC => {
                arguments.push(Argument::from("/EHsc"));
                arguments.push(Argument::from("/nologo"));
                arguments.push(Argument::from("/experimental:module"));
                arguments.push(Argument::from("/stdIfcDir \"$(VC_IFCPath)\""));
                arguments.push(Argument::from("/c"));

                let implicit_lookup_mius_path = out_dir
                    .join(compiler.as_ref())
                    .join("modules")
                    .join("interfaces")
                    .display()
                    .to_string();
                arguments.push(Argument::from("/ifcSearchDir"));
                arguments.push(implicit_lookup_mius_path.clone().into());
                arguments.push(Argument::from("/ifcOutput"));
                arguments.push(implicit_lookup_mius_path.into());

                // The output .obj file
                let obj_file =
                    Argument::from(helpers::generate_prebuild_miu(compiler, out_dir, interface));
                commands.generated_files_paths.push(obj_file.clone());
                arguments.push(Argument::from(format!("/Fo{obj_file}")));

                if let Some(partition) = &interface.partition {
                    if partition.is_internal_partition {
                        arguments.push(Argument::from("/internalPartition"));
                    } else {
                        arguments.push(Argument::from("/interface"));
                    }
                } else {
                    arguments.push(Argument::from("/interface"));
                }
                arguments.push(Argument::from("/TP"));
                // The input file
                arguments.push(Argument::from(interface.file()))
            }
            CppCompiler::GCC => {
                arguments.push(Argument::from("-fmodules-ts"));
                arguments.push(Argument::from("-x"));
                arguments.push(Argument::from("c++"));
                arguments.push(Argument::from("-c"));
                // The input file
                arguments.push(Argument::from(interface.file()));
                // The output file
                arguments.push(Argument::from("-o"));
                let miu_file_path =
                    Argument::from(helpers::generate_prebuild_miu(compiler, out_dir, interface));
                commands.generated_files_paths.push(miu_file_path.clone());
                arguments.push(miu_file_path);
            }
        }

        let command_line = ModuleCommandLine {
            path: interface.file(),
            args: arguments,
            processed: false,
            execution_result: CommandExecutionResult::default(),
        };
        commands.interfaces.push(command_line);
    }

    /// Generates the expected arguments for compile the implementation module files
    pub fn generate_module_implementation_args<'a>(
        model: &'a ZorkModel,
        implementation: &'a ModuleImplementationModel,
        commands: &mut Commands<'a>,
    ) {
        let compiler = model.compiler.cpp_compiler;
        let out_dir = model.build.output_dir;

        let mut arguments = Vec::with_capacity(12);
        arguments.push(model.compiler.language_level_arg());

        match compiler {
            CppCompiler::CLANG => {
                arguments.extend(clang_args::add_std_lib(model));
                arguments.push(Argument::from("-fimplicit-modules"));
                arguments.push(Argument::from("-c"));
                arguments.push(clang_args::implicit_module_maps(out_dir));

                // The resultant object file
                arguments.push(Argument::from("-o"));
                let obj_file_path = Argument::from(helpers::generate_impl_obj_file(
                    compiler,
                    out_dir,
                    implementation,
                ));
                commands.generated_files_paths.push(obj_file_path.clone());
                arguments.push(obj_file_path);

                clang_args::add_direct_module_interfaces_dependencies(
                    &implementation.dependencies,
                    compiler,
                    out_dir,
                    &mut arguments,
                );

                // The input file
                arguments.push(Argument::from(implementation.file()))
            }
            CppCompiler::MSVC => {
                arguments.push(Argument::from("/EHsc"));
                arguments.push(Argument::from("/nologo"));
                arguments.push(Argument::from("-c"));
                arguments.push(Argument::from("/experimental:module"));
                arguments.push(Argument::from("/stdIfcDir \"$(VC_IFCPath)\""));
                arguments.push(Argument::from("/ifcSearchDir"));
                arguments.push(Argument::from(
                    out_dir
                        .join(compiler.as_ref())
                        .join("modules")
                        .join("interfaces"),
                ));
                // The input file
                arguments.push(Argument::from(implementation.file()));
                // The output .obj file
                let obj_file_path = out_dir
                    .join(compiler.as_ref())
                    .join("modules")
                    .join("implementations")
                    .join(implementation.filestem())
                    .with_extension(compiler.get_obj_file_extension());

                commands
                    .generated_files_paths
                    .push(Argument::from(obj_file_path.clone()));
                arguments.push(Argument::from(format!("/Fo{}", obj_file_path.display())));
            }
            CppCompiler::GCC => {
                arguments.push(Argument::from("-fmodules-ts"));
                arguments.push(Argument::from("-c"));
                // The input file
                arguments.push(Argument::from(implementation.file()));
                // The output file
                arguments.push(Argument::from("-o"));
                let obj_file_path = Argument::from(helpers::generate_impl_obj_file(
                    compiler,
                    out_dir,
                    implementation,
                ));
                commands.generated_files_paths.push(obj_file_path.clone());
                arguments.push(obj_file_path);
            }
        }

        let command_line = ModuleCommandLine {
            path: implementation.file(),
            args: arguments,
            processed: false,
            execution_result: CommandExecutionResult::default(),
        };
        commands.implementations.push(command_line);
    }
}

/// Helpers for reduce the cyclomatic complexity introduced by the
/// kind of workflow that should be done with this parse, format and
/// generate.
mod helpers {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::{
        bounds::TranslationUnit,
        cache::ZorkCache,
        cli::output::commands::{CommandExecutionResult, ModuleCommandLine},
    };
    use std::path::PathBuf;
    use crate::utils::fs;

    /// Creates the path for a prebuilt module interface, based on the default expected
    /// extension for BMI's given a compiler
    ///
    /// We use join for the extension instead `with_extension` because modules are allowed to contain
    /// dots in their module identifier declaration. So, for example, a module with a declaration of:
    /// `export module dotted.module`, in Clang, due to the expected `.pcm` extension, the final path
    /// will be generated as `dotted.pcm`, instead `dotted.module.pcm`.
    ///
    /// For MSVC, we are relying in the autogenerate of the BMI automatically by the compiler,
    /// so the output file that we need is an obj file (.obj), and not the
    /// binary module interface (.ifc)
    pub(crate) fn generate_prebuild_miu(
        compiler: CppCompiler,
        out_dir: &Path,
        interface: &ModuleInterfaceModel,
    ) -> PathBuf {
        let mod_unit = if compiler.eq(&CppCompiler::CLANG) {
            let mut temp = String::new();
            if let Some(partition) = &interface.partition {
                temp.push_str(partition.module);
                temp.push('-');
                if !partition.partition_name.is_empty() {
                    temp.push_str(partition.partition_name)
                } else {
                    temp.push_str(&interface.filestem())
                }
            } else {
                temp.push_str(interface.module_name)
            }
            temp
        } else {
            interface.module_name.to_string()
        };

        out_dir
            .join(compiler.as_ref())
            .join("modules")
            .join("interfaces")
            .join(format!(
                "{mod_unit}.{}",
                if compiler.eq(&CppCompiler::MSVC) {
                    compiler.get_obj_file_extension()
                } else {
                    compiler.get_typical_bmi_extension()
                }
            ))
    }

    pub(crate) fn generate_impl_obj_file(
        compiler: CppCompiler,
        out_dir: &Path,
        implementation: &ModuleImplementationModel,
    ) -> PathBuf {
        out_dir
            .join(compiler.as_ref())
            .join("modules")
            .join("implementations")
            .join(implementation.filestem())
            .with_extension(compiler.get_obj_file_extension())
    }

    /// System headers as can be imported as modules must be built before being imported.
    /// First it will compare with the elements stored in the cache, and only we will
    /// generate commands for the non processed elements yet.
    ///
    /// This is for `GCC` and `Clang`
    pub(crate) fn prebuild_sys_modules<'a>(
        model: &'a ZorkModel,
        commands: &mut Commands<'a>,
        cache: &ZorkCache,
    ) {
        if !cache.compilers_metadata.system_modules.is_empty() {
            log::info!(
                "System modules already build: {:?}. They will be skipped!",
                cache.compilers_metadata.system_modules
            );
        }

        let language_level = model.compiler.language_level_arg();
        let sys_modules = model
            .modules
            .sys_modules
            .iter()
            .filter(|sys_module| {
                !cache
                    .compilers_metadata
                    .system_modules
                    .iter()
                    .any(|s| s.eq(**sys_module))
            })
            .map(|sys_module| {
                let mut v = vec![
                    language_level.clone(),
                    Argument::from("-fmodules-ts"),
                    Argument::from("-x"),
                    Argument::from("c++-system-header"),
                    Argument::from(*sys_module),
                ];

                if model.compiler.cpp_compiler.eq(&CppCompiler::CLANG) {
                    v.push(Argument::from("-o"));
                    v.push(Argument::from(
                        Path::new(model.build.output_dir)
                            .join(model.compiler.cpp_compiler.as_ref())
                            .join("modules")
                            .join("interfaces")
                            .join(sys_module)
                            .with_extension(
                                model.compiler.cpp_compiler.get_typical_bmi_extension(),
                            ),
                    ));
                }

                v
            })
            .collect::<Vec<_>>();

        // TODO Cached sys headers will deserve in a near future its own type,
        // storing the commands and the paths there, and checking for rebuild them
        // independently, also check for execution independently
        for collection_args in sys_modules {
            commands.interfaces.push(ModuleCommandLine {
                path: PathBuf::from(collection_args[4].value),
                args: collection_args,
                processed: false,
                execution_result: CommandExecutionResult::default(),
            });
        }
    }

    /// Marks the given module translation unit as already processed,
    /// or if it should be reprocessed again due to a previous failure status,
    /// to avoid losing time rebuilding that module if the source file
    /// hasn't been modified since the last build process iteration.
    ///
    /// True means already processed and previous iteration Success
    pub(crate) fn flag_modules_without_changes(
        compiler: &CppCompiler,
        cache: &ZorkCache,
        file: &Path,
    ) -> bool {
        if compiler.eq(&CppCompiler::CLANG) {
            log::trace!("Module unit {file:?} will be rebuilt since we've detected that you are using Clang");
        }
        // Check first if the file is already on the cache, and if it's last iteration was successful
        if let Some(cached_file) = cache.is_file_cached(file) {
            if cached_file.execution_result != CommandExecutionResult::Success
                && cached_file.execution_result != CommandExecutionResult::Cached
            {
                log::trace!(
                    "File {file:?} with status: {:?}. Marked to reprocess",
                    cached_file.execution_result
                );
                return false;
            };

            // If exists and was successful, let's see if has been modified after the program last iteration
            fs::did_file_changed_since_last_run(cache, file).unwrap_or(false)
        } else {
            false
        }
    }
}
