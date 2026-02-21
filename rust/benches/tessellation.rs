use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::Vec3;

fn bench_triangulate(c: &mut Criterion) {
    let mut group = c.benchmark_group("triangulation");
    for n in [3usize, 4, 8, 16, 32] {
        // Generate a regular N-gon on the XY plane
        let points: Vec<Vec3> = (0..n)
            .map(|i| {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / n as f32;
                Vec3::new(angle.cos(), angle.sin(), 0.0)
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("polygon", n),
            &points,
            |b, points| {
                b.iter(|| rust::bim::triangulate::triangulate_polygon(points, Vec3::Z));
            },
        );
    }
    group.finish();
}

fn bench_box_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("box_generation");
    group.bench_function("generate_box_with_normals", |b| {
        b.iter(|| {
            rust::bim::geometry::generate_box_with_normals(
                [0.0, 0.0, 0.0],
                [2.0, 3.0, 0.2],
                [0.8, 0.8, 0.8, 1.0],
            );
        });
    });
    group.finish();
}

fn bench_mesh_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_merge");
    for count in [10usize, 50, 200] {
        let meshes: Vec<rust::bim::geometry::Mesh> = (0..count)
            .map(|i| {
                rust::bim::geometry::generate_box_with_normals(
                    [i as f32 * 3.0, 0.0, 0.0],
                    [2.0, 3.0, 0.2],
                    [0.8, 0.8, 0.8, 1.0],
                )
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("merge", count),
            &meshes,
            |b, meshes| {
                b.iter(|| rust::bim::geometry::merge_meshes(meshes.clone()));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_triangulate, bench_box_generation, bench_mesh_merge);
criterion_main!(benches);
