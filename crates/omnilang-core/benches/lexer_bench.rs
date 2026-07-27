use criterion::{black_box, criterion_group, criterion_main, Criterion};
use omnilang_core::lexer::tokenize;

fn bench_lexer_counter(c: &mut Criterion) {
    let source = r#"
component Counter {
    signal count: i32 = 0;
    computed double: i32 = count * 2;
    effect { console.log(count); }
    view {
        <div>
            <Text>Count: {count}</Text>
            <Button onPress={() => count += 1}>+</Button>
        </div>
    }
}
"#;
    c.bench_function("lexer_counter_component", |b| {
        b.iter(|| tokenize(black_box(source)))
    });
}

fn bench_lexer_large(c: &mut Criterion) {
    let source = include_str!("../../../test/projects/learn-omniview/src/pages/TodoPage.omni");
    c.bench_function("lexer_todo_page", |b| {
        b.iter(|| tokenize(black_box(source)))
    });
}

criterion_group!(benches, bench_lexer_counter, bench_lexer_large);
criterion_main!(benches);
