//! Benchmarks tests for measuring the performance of the code

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zork::{
    cache::{self, ZorkCache},
    compiler::build_project,
    config_file::ZorkConfigFile,
    utils::{self, reader::build_model},
};

/// To succesfully run the benchmarks, change the filename of the
/// [deactivated]zork_clang.toml under the github-example folder
pub fn build_project_benchmark(c: &mut Criterion) {
    let config: ZorkConfigFile = toml::from_str(utils::constants::CONFIG_FILE_MOCK).unwrap();
    let program_data = build_model(&config);

    c.bench_function("Build project", |b| {
        b.iter(|| {
            build_project(
                black_box(&program_data),
                black_box(&ZorkCache::default()),
                false,
            )
        })
    });

    c.bench_function("Cache loading time", |b| {
        b.iter(|| cache::load(black_box(&program_data)))
    });
}

criterion_group!(benches, build_project_benchmark);
criterion_main!(benches);
