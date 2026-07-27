use criterion::{black_box, criterion_group, criterion_main, Criterion};
use omnilang_core::analyse::analyse_multi;
use rustc_hash::FxHashMap;

fn bench_pipeline_single_file(c: &mut Criterion) {
    let source = r#"
component Counter {
    signal count: i32 = 0;
    computed double: i32 = count * 2;
    view {
        <div>
            <Text>Count: {count}</Text>
            <Button onPress={() => count += 1}>+</Button>
        </div>
    }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert("Counter.omni".to_string(), source.to_string());
    let path_mapping = FxHashMap::default();

    c.bench_function("pipeline_counter", |b| {
        b.iter(|| analyse_multi(black_box(&sources), black_box(&path_mapping)))
    });
}

criterion_group!(benches, bench_pipeline_single_file);
criterion_main!(benches);
