use {
    criterion::{BatchSize, Criterion, criterion_group, criterion_main},
    empl::config::lisp::{
        ast::ExprParser, evaluator::Environment, lexer::LexemeParser, parser::Parser,
    },
};

fn bench(c: &mut Criterion) {
    let lexemes = LexemeParser
        .iter(
            "(progn
    (seq-map (lambda (x) 1) (list 2 2 2))
    (seq-filter (lambda (x) #t) (list 1 2 3))
    (seq-filter (lambda (x) #f) (list 1 2 3))
    (seq-flat-map (lambda (x) (list 1 2 3)) (list 2 2 2)))",
        )
        .collect::<Vec<_>>();
    let expr = ExprParser.parse(&lexemes).unwrap().output;
    let mut env = Environment::new();

    let mut c = c.benchmark_group("seq-*");
    c.sample_size(10_000);
    c.bench_function("progn", |b| {
        b.iter_batched(
            || expr.clone(),
            |expr| {
                env.eval(expr).unwrap();
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
