use {
    empl::config::clisp::{lexer::LexemeParser, parser::Parser},
    std::io::stdin,
};

fn main() {
    let mut buf = String::new();
    while stdin().read_line(&mut buf).is_ok() {
        let mut next = buf.as_str();
        while let Ok(output) = LexemeParser.parse(next) {
            println!("{:?}", output.output);
            next = output.next;
        }

        buf.clear();
    }
}
