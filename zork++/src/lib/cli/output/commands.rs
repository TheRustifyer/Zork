use std::path::Path;

///! Contains helpers and data structure to process in
/// a nice and neat way the commands generated to be executed
/// by Zork++

use crate::{project_model::compiler::CppCompiler, utils::constants};
use color_eyre::{eyre::Context, Result};
use serde::{Serialize, Deserialize};

use super::arguments::Argument;


/// TODO this is just a provisional impl, in order to free the
/// build_project(...) function for dealing with the generated
/// command lines
pub fn run_generated_commands(commands: &Commands<'_>) -> Result<()> {
    for miu in &commands.interfaces {
        execute_command(&commands.compiler, miu)?
    }

    for impls in &commands.implementations {
        execute_command(&commands.compiler, impls)?
    }

    execute_command(&commands.compiler, &commands.sources)?;

    Ok(())
}


/// Executes a new [`std::process::Command`] to run the generated binary
/// after the build process in the specified shell
pub fn autorun_generated_binary(
    compiler: &CppCompiler,
    output_dir: &Path,
    executable_name: &str
) -> Result<()> {
    let args = &[
        Argument::from(
            output_dir
                .join(compiler.as_ref())
                .join(executable_name)
                .with_extension(constants::BINARY_EXTENSION)
        ),
    ];

    log::info!(
        "\n\n[{compiler}] - Executing the generated binary => {:?}", args.join(" ")
    );

    let process = std::process::Command::new(
        Argument::from(
        output_dir.join(compiler.as_ref()).join(executable_name)
    )).spawn()?
    .wait()
    .with_context(|| format!("[{compiler}] - Command {:?} failed!", args.join(" ")))?;

    log::info!("[{compiler}] - Result: {:?}", process);
    Ok(())
}


/// Executes a new [`std::process::Command`] configured according the choosen
/// compiler and the current operating system
fn execute_command(compiler: &CppCompiler, arguments: &[Argument<'_>]) -> Result<()> {
    log::info!(
        "[{compiler}] - Executing command => {:?}",
        format!("{} {}", compiler.get_driver(), arguments.join(" "))
    );

    let process = if compiler.eq(&CppCompiler::MSVC) {
        std::process::Command::new( // TODO The initialization process + cache process MUST dynamically get this path and store it in cache
            "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat"
        ).arg("&&")
            .arg(compiler.get_driver())
            .args(arguments)
            .spawn()?
            .wait()
            .with_context(|| format!("[{compiler}] - Command {:?} failed!", arguments.join(" ")))?
    } else {
        std::process::Command::new(compiler.get_driver())
            .args(arguments)
            .spawn()?
            .wait()
            .with_context(|| format!("[{compiler}] - Command {:?} failed!", arguments.join(" ")))?
    };

    log::info!("[{compiler}] - Result: {:?}", process);
    Ok(())
}

/// Executes a new [`std::process::Command`] configured according the choosen
/// compiler and the current operating system composed of multiple prebuilt command
/// lines joining them in one statement
/// 
/// TODO! Probably it would be better only make a big command formed by all the commands
/// for the MSVC compiler in order to avoid to launch the developers command prompt
/// for every commmand, but, as observed, generally speaking opening a shell under
/// Unix using Clang or GCC it's extremily fast, so we can mantain the curren architecture
/// of opening a shell for command, so the user is able to track better failed commands
fn _execute_commands(compiler: &CppCompiler, arguments_for_commands: &Vec<Vec<Argument<'_>>>) -> Result<()> {
    let mut commands = if compiler.eq(&CppCompiler::MSVC) {
        std::process::Command::new( // TODO The initialization process + cache process MUST dynamically get this path and store it in cache
            "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat"
        )
    } else {
        std::process::Command::new("sh")
    };

    arguments_for_commands.iter().for_each(|args_collection| {
        log::info!(
            "[{compiler}] - Generating command => {:?}",
            format!("{} {}", compiler.get_driver(), args_collection.join(" "))
        );

        commands.arg("&&")
            .arg(compiler.get_driver())
            .args(args_collection);
    });

    commands.spawn()?
        .wait()
        .with_context(|| format!("[{compiler}] - Command {commands:?} failed!"))?;
    

    log::info!("[{compiler}] - Result: {:?}", commands);
    Ok(())
}


/// Holds the generated command line arguments for a concrete compiler
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Commands<'a> {
    pub compiler: CppCompiler,
    #[serde(borrow)]
    pub interfaces: Vec<Vec<Argument<'a>>>,
    pub implementations: Vec<Vec<Argument<'a>>>,
    pub sources: Vec<Argument<'a>>,
    pub generated_files_paths: Vec<Argument<'a>>,
}

impl<'a> Commands<'a> {
    pub fn new(compiler: &'a CppCompiler) -> Self {
        Self {
            compiler: *compiler,
            interfaces: Vec::with_capacity(0),
            implementations: Vec::with_capacity(0),
            sources: Vec::with_capacity(0),
            generated_files_paths: Vec::with_capacity(0),
        }
    }
}

impl<'a> core::fmt::Display for Commands<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Commands for [{}]:\n- Interfaces: {:?},\n- Implementations: {:?},\n- Main command line: {:?}",
            self.compiler,
            self.interfaces.iter().map(|vec| { vec.iter().map(|e| e.value).collect::<Vec<_>>().join(" "); }),
            self.implementations.iter().map(|vec| { vec.iter().map(|e| e.value).collect::<Vec<_>>().join(" "); }),
            self.sources.iter().map(|e| e.value).collect::<Vec<_>>().join(" ")
        )
    }
}
