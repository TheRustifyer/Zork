"""[summary]

    This file provides several functions that creates the
    command line compiler calls, generated after parsing the
    Zork config file and retrieve the data
"""

import os
import subprocess

from program_definitions import CLANG, GCC, MSVC
from utils.exceptions import LanguageLevelNotEnought, UnsupportedCompiler


def build_project(config: dict, verbose: bool) -> int:
    """ Calls the selected compiler to perform the build of the project """

    generate_build_output_directory(config)

    compiler = config['compiler'].cpp_compiler
    command_line: list = []

    if compiler == CLANG:
        command_line = call_clang_to_compile(config, verbose)
    elif compiler == GCC:
        raise UnsupportedCompiler(GCC)
    else:
        raise UnsupportedCompiler(MSVC)

    if verbose:
        print(f'Command line executed: {" ".join(command_line)}\n')

    return subprocess.Popen(command_line).wait()


def call_clang_to_compile(config: dict, verbose: bool):
    """ Calls Clang++ to compile the provide files / project """
    # Generates the compiler and linker calls
    command_line = [
        config.get("compiler").cpp_compiler,
        '--std=c++' + config.get("language").cpp_standard,
        '-stdlib=' + config.get("language").std_lib,
        '-o', config['build'].output_dir + '/' +
        config.get("executable").executable_name,
    ]

    for source in config.get("executable").sources:
        command_line.append(source)

    # Generates a compiler call to prebuild the module units, in case that
    # the attribute it's present, have a valid path to the .cppm module units
    # and the language level it's at least, c++20.
    if config['language'].modules != []:
        if int(config.get("language").cpp_standard) < 20:
            raise LanguageLevelNotEnought(
                20,
                config.get("language").cpp_standard,
                "Modules"
            )
        # TODO Modulos en Clang requieren de extensión .cppm
        prebuild_modules_path = call_clang_to_prebuild_modules(config, verbose)
        for module_src in config['language'].modules:
            command_line.append(module_src)
        command_line.append('-fmodules')
        command_line.append('-fmodules-ts')
        command_line.append(
            f'-fprebuilt-module-path={prebuild_modules_path}'
        )

    return command_line


def call_clang_to_prebuild_modules(config: dict, verbose: bool) -> list:
    """ The responsable for generate de module units
        for the C++20 modules feature.
        Returns a list with the args that should be passed into them
        main compiler call in order to enable the modules compilation
        and linkage """
    output_dir: str = config['build'].output_dir
    modules_dir_path = config['build'].output_dir + '/modules'

    if verbose:
        print('Precompiling the module units...')
    # Generate the precompiled modules directory if it doesn't exists
    if 'modules' not in os.listdir(output_dir):
        subprocess.Popen(['mkdir', modules_dir_path]).wait()

    for module in config.get('language').modules:
        # Strip the path part if the module name it's inside a path,
        # (like 'src/inner/module_file_name.cppm') and not alone,
        # as a *.cppm file, and strips the file extension
        if module.__contains__('/'):
            module_dir_parts_no_slashes: list = module.split('/')
            module_name: str = \
                module_dir_parts_no_slashes[
                    len(module_dir_parts_no_slashes) - 1
                ]
            module_name_no_extensions = ''.join(module_name.split('.')[0])
            module_name: str = module_name_no_extensions

        subprocess.Popen(
            [
                config.get("compiler").cpp_compiler,
                '--std=c++' + config.get("language").cpp_standard,
                '-stdlib=' + config.get("language").std_lib,
                '-fmodules',
                '--precompile',
                '-o', f'{modules_dir_path}/{module_name}.pcm',
                module
            ]
        ).wait()
    if verbose:
        print('...\nPrecompilation finished!')

    return modules_dir_path


def generate_build_output_directory(config: dict):
    """ Creates the directory where the compiler will dump the
        generated files after the build process """
    output_build_dir = config['build'].output_dir
    if not output_build_dir.strip('./') in os.listdir():
        subprocess.Popen(['mkdir', output_build_dir]).wait()
