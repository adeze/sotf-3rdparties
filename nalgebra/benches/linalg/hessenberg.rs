use na::{DMatrix, Hessenberg, Matrix4};


// Without unpack.
fn hessenberg_decompose_4x4(bh: &mut criterion::Criterion) {
    bh.bench_function("hessenberg_decompose_4x4", |bh| {
        bh.iter_batched(
            Matrix4::<f64>::new_random,
            Hessenberg::new,
            criterion::BatchSize::SmallInput,
        )
    });
}

fn hessenberg_decompose_100x100(bh: &mut criterion::Criterion) {
    bh.bench_function("hessenberg_decompose_100x100", |bh| {
        bh.iter_batched(
            || DMatrix::<f64>::new_random(100, 100),
            Hessenberg::new,
            criterion::BatchSize::SmallInput,
        )
    });
}

fn hessenberg_decompose_200x200(bh: &mut criterion::Criterion) {
    bh.bench_function("hessenberg_decompose_200x200", |bh| {
        bh.iter_batched(
            || DMatrix::<f64>::new_random(200, 200),
            Hessenberg::new,
            criterion::BatchSize::SmallInput,
        )
    });
}

// With unpack.
fn hessenberg_decompose_unpack_100x100(bh: &mut criterion::Criterion) {
    bh.bench_function("hessenberg_decompose_unpack_100x100", |bh| {
        bh.iter_batched(
            || DMatrix::<f64>::new_random(100, 100),
            |m| {
                let hess = Hessenberg::new(m);
                hess.unpack()
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn hessenberg_decompose_unpack_200x200(bh: &mut criterion::Criterion) {
    bh.bench_function("hessenberg_decompose_unpack_200x200", |bh| {
        bh.iter_batched(
            || DMatrix::<f64>::new_random(200, 200),
            |m| {
                let hess = Hessenberg::new(m);
                hess.unpack()
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    hessenberg,
    hessenberg_decompose_4x4,
    hessenberg_decompose_100x100,
    hessenberg_decompose_200x200,
    hessenberg_decompose_unpack_100x100,
    hessenberg_decompose_unpack_200x200,
);
