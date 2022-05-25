import glob

from dataclasses import dataclass
from typing import Any

"""[summary]
    Provides dataclasses to store the options selected by the
    user in the configuration file after parse it
"""


@dataclass
class CompilerConfig:
    cpp_compiler: str

    def set_property(self, property_name: str, value: Any):
        if property_name == 'cpp_compiler':
            self.cpp_compiler = value


@dataclass
class LanguageConfig:
    cpp_standard: int
    std_lib: str
    modules: list

    def set_property(self, property_name: str, value: Any):
        if property_name == 'cpp_standard':
            self.cpp_standard = value
        elif property_name == 'std_lib':
            self.std_lib = value
        elif property_name == 'modules':
            self.modules = get_sources(value)


@dataclass
class BuildConfig:
    output_dir: str

    def set_property(self, property_name: str, value: Any):
        if property_name == 'output_dir':
            self.output_dir = value


@dataclass
class ExecutableConfig:
    executable_name: str
    sources: list
    auto_execute: str

    def set_property(self, property_name: str, value: Any):
        if property_name == 'executable_name':
            self.executable_name = value
        elif property_name == 'sources':
            self.sources = get_sources(value)
        elif property_name == 'auto_execute':
            self.auto_execute = value


def get_sources(value) -> list:
    """ Convenient function designed to retrieve the user defined
        source files or module units file names """
    sources = []
    for source in value.split(','):
        # Remove unnecesary whitespaces
        source = source.strip(' ')
        # Check if it's a path, add the relative ./ to the Zork config file
        if source.__contains__('/') and not source.startswith('./'):
            source = './' + source
        # Check for wildcards, so every file in the provided directory
        # should be included
        if source.__contains__('*'):
            for wildcarded_source in glob.glob(source):
                sources.append(wildcarded_source)
        else:
            sources.append(source)
    return sources