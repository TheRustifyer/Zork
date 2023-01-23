//! The crate responsable for executing the core work of `Zork++`,
// generate command lines and execute them in a shell of the current
// operating system against the designed compilers in the configuration
// file.

use color_eyre::Result;
use std::{path::Path, rc::Rc, cell::RefCell};

use crate::{
    cli::{input::CliArgs, output::{commands::{Commands, execute_command}, arguments::Argument}},
    project_model::{
        compiler::CppCompiler,
        modules::{ModuleImplementationModel, ModuleInterfaceModel},
        ZorkModel,
    }, utils
};

/// The entry point of the compilation process
///
/// Whenever this process gets triggered, the files declared within the
/// configuration file will be build
pub fn build_project<'a>(
    base_path: &Path, 
    model: &ZorkModel<'a>,
    _cli_args: &CliArgs
) -> Result<()> {
    // A registry of the generated command lines
    let mut commands = Rc::new(
        RefCell::new(
            Commands::new(&model.compiler.cpp_compiler)
        )
    );

    // Create the directory for dump the generated files
    create_output_directory(base_path, &model)?;

    if model.compiler.cpp_compiler == CppCompiler::GCC { // Special GCC case
        helpers::process_gcc_system_modules(&model, &mut commands.borrow_mut())
    }

    // let mut binding = commands.borrow_mut();
    // let mut binding2 = commands.borrow_mut();
    // 1st - Build the modules
    let mut binding = commands.borrow_mut();
    // build_modules(&model, &mut *binding)?;
    // 2st - Build the executable or the tests
    // build_executable(&model, &mut binding2)?;

    // execute_command(&model.compiler.cpp_compiler, &commands.borrow().sources)?;
    // for miu in &commands.borrow().interfaces {
    //     execute_command(commands.borrow().compiler, miu)?
    // }

    Ok(())
}

/// Triggers the build process for compile the source files declared for the project
/// and the
fn build_executable<'a>(
    model: &'a ZorkModel,
    commands: &'a mut Commands<'a>,
) -> Result<()> {
    Ok(
        sources::generate_main_command_line_args(
            model, commands,&model.executable,
        )
    ?)
}

/// Triggers the build process for compile the declared modules in the project
///
/// This function acts like a operation result processor, by running instances
/// and parsing the obtained result, handling the flux according to the
/// compiler responses>
fn build_modules<'a: 'b, 'b>(model: &'a ZorkModel<'a>, commands: &'b mut Commands<'b>) -> Result<()> {
    log::info!("\n\nBuilding the module interfaces");
    prebuild_module_interfaces(
        model,
        commands
    );

    log::info!("\n\nBuilding the module implementations");
    compile_module_implementations(model, commands);

    Ok(())
}

/// Parses the configuration in order to build the BMIs declared for the project,
/// by precompiling the module interface units
fn prebuild_module_interfaces<'a, 'b: 'a>(
    model: &'a ZorkModel<'b>,
    commands: &'a  mut Commands<'b>,
) {
    model.modules.interfaces.iter().for_each(|module_interface| {
        sources::generate_module_interfaces_args(model, module_interface, commands);
    });
}

/// Parses the configuration in order to compile the module implementation
/// translation units declared for the project
fn compile_module_implementations(
    model: &'_ ZorkModel<'_>,
    // impls: &'_ [ModuleImplementationModel<'_>],
    commands: &'_ mut Commands<'_>,
) {
    // impls.iter().for_each(|module_impl| {
    //     // sources::generate_module_implementation_args(model, module_impl, commands);
    // });
}

/// Creates the directory for output the elements generated
/// during the build process. Also, it will generate the
/// ['output_build_dir'/zork], which is a subfolder
/// where Zork dumps the things that needs to work correctly
/// under different conditions.
///
/// Under /zork, some new folders are created:
/// - a /intrinsics folder in created as well,
/// where different specific details of Zork++ are stored
/// related with the C++ compilers
///
/// - a /cache folder, where lives the metadata cached by Zork++
/// in order to track different aspects of the program (last time
/// modified files, last process build time...)
pub fn create_output_directory(base_path: &Path, model: &ZorkModel) -> Result<()> {
    let out_dir = &model.build.output_dir;
    let compiler = &model.compiler.cpp_compiler;

    // Recursively create a directory and all of its parent components if they are missing
    let modules_path = Path::new(base_path)
        .join(out_dir)
        .join(compiler.to_string())
        .join("modules");
    let zork_path = base_path.join(out_dir).join("zork");
    let zork_cache_path = zork_path.join("cache");
    let zork_intrinsics_path = zork_path.join("intrinsics");

    utils::fs::create_directory(&modules_path.join("interfaces"))?;
    utils::fs::create_directory(&modules_path.join("implementations"))?;
    utils::fs::create_directory(&zork_cache_path)?;
    utils::fs::create_directory(&zork_intrinsics_path)?;

    // TODO This possibly would be temporary
    if compiler.eq(&CppCompiler::CLANG) && cfg!(target_os = "windows") {
        utils::fs::create_file(
            &zork_intrinsics_path,
            "std.h",
            utils::template::resources::STD_HEADER.as_bytes(),
        )?;
        utils::fs::create_file(
            &zork_intrinsics_path,
            "zork.modulemap",
            utils::template::resources::ZORK_MODULEMAP.as_bytes(),
        )?;
    }

    Ok(())
}

/// Specific operations over source files
mod sources {
    use color_eyre::Result;

    use crate::{
        bounds::{ExecutableTarget, TranslationUnit},
        project_model::{
            compiler::CppCompiler,
            modules::{ModuleImplementationModel, ModuleInterfaceModel},
            ZorkModel,
        }, cli::output::{commands::Commands, arguments::{Argument, clang_args}}, utils::constants,
    };

    use super::helpers;

    /// Generates the command line arguments for non-module source files, including the one that
    /// holds the main function
    pub fn generate_main_command_line_args<'a>(
        model: &'a ZorkModel,
        commands: &'a mut Commands<'a>,
        target: &'a impl ExecutableTarget<'a>,
    ) -> Result<()> {
        log::info!("\n\nGenerating the main command line");

        let compiler = &model.compiler.cpp_compiler;
        let out_dir = model.build.output_dir;
        let executable_name = target.name();

        let mut arguments = Vec::new();
        arguments.push(model.compiler.language_level_arg());

        match compiler {
            CppCompiler::CLANG => {
                if let Some(arg) = model.compiler.stdlib_arg() {
                    arguments.push(arg);
                }

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
        commands.sources = arguments;

        Ok(())
    }

    /// Generates the expected arguments for precompile the BMIs depending on self
    pub fn generate_module_interfaces_args<'a: 'b, 'b>(
        model: &'a ZorkModel<'b>,
        interface: &'_ ModuleInterfaceModel,
        commands: &'a mut Commands<'b>,
    ) {
        let compiler = &model.compiler.cpp_compiler;
        let base_path = model.modules.base_ifcs_dir;
        let out_dir = model.build.output_dir;

        let mut arguments = Vec::with_capacity(8);
        arguments.push(model.compiler.language_level_arg());

        match *compiler {
            CppCompiler::CLANG => {
                if let Some(arg) = model.compiler.stdlib_arg() {
                    arguments.push(arg);
                }

                arguments.push(Argument::from("-fimplicit-modules"));
                arguments.push(Argument::from("-x"));
                arguments.push(Argument::from("c++-module"));
                arguments.push(Argument::from("--precompile"));

                arguments.push(clang_args::implicit_module_maps(out_dir));
                // The resultant BMI as a .pcm file
                arguments.push(Argument::from("-o"));
                // The output file
                let miu_file_path =
                    Argument::from(helpers::generate_prebuild_miu(compiler, out_dir, interface));
                commands.generated_files_paths.push(miu_file_path.clone());
                arguments.push(miu_file_path);
                // The input file
                arguments.push(Argument::from(helpers::add_input_file(
                    interface, base_path,
                )));
            }
            CppCompiler::MSVC => {
                arguments.push(Argument::from("/EHsc"));
                arguments.push(Argument::from("/nologo"));
                arguments.push(Argument::from("/experimental:module"));
                arguments.push(Argument::from("/stdIfcDir \"$(VC_IFCPath)\""));
                arguments.push(Argument::from("/c"));
                // The output .ifc file
                arguments.push(Argument::from("/ifcOutput"));
                let miu_file_path =
                    Argument::from(helpers::generate_prebuild_miu(compiler, out_dir, interface));
                arguments.push(miu_file_path);
                // The output .obj file
                arguments.push(Argument::from(format!(
                    "/Fo{}",
                    out_dir
                        .join(compiler.as_ref())
                        .join("modules")
                        .join("interfaces")
                        .display()
                )));
                // The input file
                arguments.push(Argument::from("/interface"));
                arguments.push(Argument::from("/TP"));
                arguments.push(Argument::from(helpers::add_input_file(
                    interface, base_path,
                )))
            }
            CppCompiler::GCC => {
                arguments.push(Argument::from("-fmodules-ts"));
                arguments.push(Argument::from("-x"));
                arguments.push(Argument::from("c++"));
                arguments.push(Argument::from("-c"));
                // The input file
                arguments.push(Argument::from(helpers::add_input_file(
                    interface, base_path,
                )));
                // The output file
                arguments.push(Argument::from("-o"));
                let miu_file_path =
                    Argument::from(helpers::generate_prebuild_miu(compiler, out_dir, interface));
                commands.generated_files_paths.push(miu_file_path.clone());
                arguments.push(miu_file_path);
            }
        }

        commands.interfaces.push(arguments);
    }

    /// Generates the expected arguments for compile the implementation module files
    pub fn generate_module_implementation_args<'a: 'b, 'b>(
        model: &'a ZorkModel<'a>,
        implementation: &'a ModuleImplementationModel,
        commands: &'a mut Commands<'b>,
    ) {
        let compiler = &model.compiler.cpp_compiler;
        let base_path = model.modules.base_impls_dir;
        let out_dir = model.build.output_dir;

        let mut arguments = Vec::with_capacity(8);
        arguments.push(model.compiler.language_level_arg());

        match *compiler {
            CppCompiler::CLANG => {
                if let Some(arg) = model.compiler.stdlib_arg() {
                    arguments.push(arg);
                }

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

                implementation.dependencies.iter().for_each(|ifc_dep| {
                    arguments.push(Argument::from(format!(
                        "-fmodule-file={}",
                        out_dir
                            .join(compiler.as_ref())
                            .join("modules")
                            .join("interfaces")
                            .join(ifc_dep)
                            .with_extension(compiler.get_typical_bmi_extension())
                            .display()
                    )))
                });

                // The input file
                arguments.push(Argument::from(helpers::add_input_file(
                    implementation,
                    base_path,
                )))
            }
            CppCompiler::MSVC => {
                arguments.push(Argument::from("/EHsc"));
                arguments.push(Argument::from("/nologo"));
                arguments.push(Argument::from("-c"));
                arguments.push(Argument::from("/experimental:module"));
                arguments.push(Argument::from("/stdIfcDir \"$(VC_IFCPath)\""));
                arguments.push(Argument::from("-ifcSearchDir"));
                arguments.push(Argument::from(
                    out_dir
                        .join(compiler.as_ref())
                        .join("modules")
                        .join("interfaces"),
                ));
                // The input file
                arguments.push(Argument::from(helpers::add_input_file(
                    implementation,
                    base_path,
                )));
                // The output .obj file
                let obj_file_path = out_dir
                    .join(compiler.as_ref())
                    .join("modules")
                    .join("implementations")
                    .join(implementation.filestem())
                    .with_extension(".obj");

                commands
                    .generated_files_paths
                    .push(Argument::from(obj_file_path.clone()));
                arguments.push(Argument::from(format!("/Fo{}", obj_file_path.display())));
            }
            CppCompiler::GCC => {
                arguments.push(Argument::from("-fmodules-ts"));
                arguments.push(Argument::from("-c"));
                // The input file
                arguments.push(Argument::from(helpers::add_input_file(
                    implementation,
                    base_path,
                )));
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

        commands.implementations.push(arguments);
    }
}

/// Helpers for reduce the cyclomatic complexity introduced by the
/// kind of workflow that should be done with this parse, format and
/// generate
mod helpers {
    use crate::bounds::TranslationUnit;
    use std::path::PathBuf;

    use super::*;

    /// Formats the string that represents an input file that will be the target of
    /// the build process and that will be passed to the compiler
    pub(crate) fn add_input_file<T: TranslationUnit>(
        translation_unit: &T,
        base_path: &Path,
    ) -> PathBuf {
        base_path.join(translation_unit.filename())
    }

    pub(crate) fn generate_prebuild_miu(
        compiler: &CppCompiler,
        out_dir: &Path,
        interface: &ModuleInterfaceModel,
    ) -> PathBuf {
        out_dir
            .join(compiler.as_ref())
            .join("modules")
            .join("interfaces")
            .join(interface.module_name)
            .with_extension(compiler.get_typical_bmi_extension())
    }

    pub(crate) fn generate_impl_obj_file(
        compiler: &CppCompiler,
        out_dir: &Path,
        implementation: &ModuleImplementationModel,
    ) -> PathBuf {
        out_dir
            .join(compiler.as_ref())
            .join("modules")
            .join("implementations")
            .join(implementation.filestem())
            .with_extension("o")
    }

    /// GCC specific requirement. System headers as modules must be built before being imported
    pub(crate) fn process_gcc_system_modules<'a>(
        model: &'a ZorkModel,
        commands: &mut Commands<'a>,
    ) {
        let language_level = model.compiler.language_level_arg();
        let sys_modules = model.modules
            .gcc_sys_headers
            .iter()
            .map(|sys_module| {
                vec![
                    language_level.clone(),
                    Argument::from("-fmodules-ts"),
                    Argument::from("-x"),
                    Argument::from("c++-system-header"),
                    Argument::from(sys_module.to_path_buf()),
                ]
        });

        commands.interfaces.extend(sys_modules);
    }
}

#[cfg(test)]
mod tests {
    use color_eyre::Result;
    use tempfile::tempdir;

    use crate::{
        config_file::ZorkConfigFile,
        utils::{reader::build_model, template::resources::CONFIG_FILE},
    };

    use super::*;

    #[test]
    fn test_creation_directories() -> Result<()> {
        let temp = tempdir()?;

        let zcf: ZorkConfigFile = toml::from_str(CONFIG_FILE)?;
        let model = build_model(&zcf);

        // This should create and out/ directory in the ./zork++ folder at the root of this project
        create_output_directory(temp.path(), &model)?;

        assert!(temp.path().join("out").exists());
        assert!(temp.path().join("out/zork").exists());
        assert!(temp.path().join("out/zork/cache").exists());
        assert!(temp.path().join("out/zork/intrinsics").exists());

        Ok(())
    }
}
