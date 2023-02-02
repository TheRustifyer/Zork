use std::{fs, path::Path};

use clap::Parser;
use color_eyre::{eyre::Context, Result};
use env_logger::Target;
use zork::{
    cache::{self, ZorkCache},
    cli::{
        input::{CliArgs, Command},
        output::commands::{self, autorun_generated_binary},
    },
    compiler::build_project,
    config_file::ZorkConfigFile,
    project_model::{compiler::CppCompiler, ZorkModel},
    utils::{
        self,
        logger::config_logger,
        reader::{build_model, find_config_files, ConfigFile},
        template::create_templated_project,
    },
};

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli_args = CliArgs::parse();
    config_logger(cli_args.verbose, Target::Stdout).expect("Error configuring the logger");

    let config_files: Vec<ConfigFile> = find_config_files(Path::new("."))
        .with_context(|| "We didn't found a `zork.toml` configuration file")?;

    for config_file in config_files {
        log::info!(
            "Launching a Zork++ work event for the configuration file: {:?}, located at: {:?}\n",
            config_file.dir_entry.file_name(),
            config_file.path
        );
        let raw_file = fs::read_to_string(config_file.path)
            .with_context(|| {
                format!(
                    "An error happened parsing the configuration file: {:?}",
                    config_file.dir_entry.file_name()
                )
            })
            .unwrap();

        let config: ZorkConfigFile = toml::from_str(raw_file.as_str())
            .with_context(|| "Could not parse configuration file")?;

        let program_data = build_model(&config);
        create_output_directory(Path::new("."), &program_data)?;

        let cache =
            cache::load(&program_data).with_context(|| "Unable to load the Zork++ caché")?;

        do_main_work_based_on_cli_input(&cli_args, &program_data, &cache).with_context(|| {
            format!(
                "Failed to build the project for the config file: {:?}",
                config_file.dir_entry.file_name()
            )
        })?;

        // TODO cache file per configuration file 
        cache::save(&program_data, cache)?;
    }

    Ok(())
}

/// Helper for reduce the cyclomatic complextity of the main fn.
///
/// Contains the main calls to the generation of the compilers commands lines,
/// the calls to the process that runs those ones, the autorun the generated
/// binaries, the tests declared for the projects...
fn do_main_work_based_on_cli_input(
    cli_args: &CliArgs,
    program_data: &ZorkModel,
    cache: &ZorkCache,
) -> Result<()> {
    match cli_args.command {
        Command::Build => {
            let commands = build_project(program_data, cache, false)
                .with_context(|| "Failed to build project")?;
            commands::run_generated_commands(&commands)
        }
        Command::Run => {
            let commands = build_project(program_data, cache, false)
                .with_context(|| "Failed to build project")?;

            commands::run_generated_commands(&commands)?;

            autorun_generated_binary(
                &program_data.compiler.cpp_compiler,
                program_data.build.output_dir,
                program_data.executable.executable_name,
            )
        }
        Command::Test => {
            let commands = build_project(program_data, cache, true)
                .with_context(|| "Failed to build project")?;

            commands::run_generated_commands(&commands)?;

            autorun_generated_binary(
                &program_data.compiler.cpp_compiler,
                program_data.build.output_dir,
                &program_data.tests.test_executable_name,
            )
        }
        Command::New {
            ref name,
            git,
            compiler,
        } => create_templated_project(Path::new("."), &name, git, compiler.into())
            .with_context(|| "Failed to create new project"),
        Command::Cache => todo!(),
    }
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
fn create_output_directory(base_path: &Path, model: &ZorkModel) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use color_eyre::Result;
    use tempfile::tempdir;

    use crate::utils::{reader::build_model, template::resources::CONFIG_FILE};
    use zork::config_file::ZorkConfigFile;

    #[test]
    fn test_creation_directories() -> Result<()> {
        let temp = tempdir()?;

        let zcf: ZorkConfigFile = toml::from_str(CONFIG_FILE)?;
        let model = build_model(&zcf);

        // This should create and out/ directory in the ./zork++ folder at the root of this project
        super::create_output_directory(temp.path(), &model)?;

        assert!(temp.path().join("out").exists());
        assert!(temp.path().join("out/zork").exists());
        assert!(temp.path().join("out/zork/cache").exists());
        assert!(temp.path().join("out/zork/intrinsics").exists());

        Ok(())
    }
}
