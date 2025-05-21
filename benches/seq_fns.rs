use {
    criterion::{criterion_group, criterion_main, BatchSize, Criterion},
    empl::config::clisp::{
        ast::ExprParser,
        evaluator::Environment,
        lexer::LexemeParser,
        parser::Parser,
    },
};

fn benchmark(c: &mut Criterion) {
    let lexemes = LexemeParser.iter("(progn
    (seq-map (lambda (x) 1) (list 2 2 2))
    (seq-filter (lambda (x) #t) (list 1 2 3))
    (seq-filter (lambda (x) #f) (list 1 2 3))
    (seq-flat-map (lambda (x) (list 1 2 3)) (list 2 2 2)))")
        .collect::<Vec<_>>();
    let expr = ExprParser.parse(&lexemes).unwrap().output;
    let mut env = Environment::new();

    c.bench_function("seq-fns", |b| b.iter_batched(|| expr.clone(), |expr| {
        env.eval(expr).unwrap();
    }, BatchSize::SmallInput));
}

criterion_group!(seq_fns, benchmark);
criterion_main!(seq_fns);
