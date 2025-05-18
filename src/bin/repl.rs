#![cfg_attr(not(test), no_main)]

#[path = "./repl/config.rs"]
pub mod config;

use {
    crate::config::{Config, Stage},
    empl::{
        argv::Argv,
        config::clisp::{
            ast::ExprParser,
            evaluator::Environment,
            lexer::LexemeParser,
            parser::{Parser, ParserOutput},
        },
        flag::Arguments,
    },
    std::{
        ffi::{c_char, c_int},
        io::stdin,
    },
};

#[cfg_attr(not(test), unsafe(no_mangle))]
fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    let argv = match unsafe { Argv::new(argc, argv) } {
        Ok(a) => a,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };

    let config = match Config::new(Arguments::new(argv.skip(1))) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    println!("{:?}", config);

    let mut buf = String::new();
    loop {
        buf.clear();
        if let Err(err) = stdin().read_line(&mut buf) {
            eprintln!("failed to read line: {err}");
            break 1;
        }

        let mut input = buf.as_str();
        let mut lexemes = Vec::new();

        while let Some(ParserOutput { next, output, .. }) = LexemeParser.parse(input) {
            lexemes.push(output);
            input = next;
        }

        if config.stage == Stage::Lex {
            println!("{lexemes:?}");
            continue;
        }

        let Some(ast) = ExprParser.parse(&lexemes) else {
            continue;
        };
        if config.stage == Stage::Parse {
            println!("{ast:?}");
            continue;
        }

        let mut environment = Environment::new();
        println!("{:?}", environment.eval(ast.output));
    }
}
