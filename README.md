# empl
terminal music player

# Building
```sh
git clone https://github.com/asdfish/empl.git --depth 1
cd empl
cargo install --path .
```

# Configuration
Configuration is done at `${XDG_CONFIG_HOME}/empl/main.lisp` or `${HOME}/.config/empl/main.lisp` using a custom lisp dialect.

## Builtin functions

Argument types are listed in a pseudo s-expression format where every item in the list is the item type.

Any argument following a `...` is considered to be variadic, so `(int int ... int)` accepts two or more arguments.

Any argument with a `?` indicates it is optional.

 - `int` indicates a number
 - `str` indicates a string
 - `any` indicates anything
 - `bool` indicates a boolean
 - `path` indicates a path
 - `'(type)` indicates a cons list with specified type inside
 - `(type)` indicates an unescaped list
 - `expr` indicates any expression
 - `ident` indicates an identifier
 - `lambda (args) output` indicates a lambda that takes the type `args` and returns `output`

| name             | arguments                             | description                                                                                                          |
|------------------|---------------------------------------|----------------------------------------------------------------------------------------------------------------------|
| `+`              | `(int int ... int)`                   | Reduce all numbers with addition.                                                                                    |
| `-`              | `(int int ... int)`                   | Reduce all numbers with subtraction.                                                                                 |
| `/`              | `(int int ... int)`                   | Reduce all numbers with division.                                                                                    |
| `*`              | `(int int ... int)`                   | Reduce all numbers with multiplication.                                                                              |
| `%`              | `(int int ... int)`                   | Reduce all numbers by getting the remainder from division.                                                           |
| `concat`         | `(str str ... str)`                   | Reduce all strings by concatenating all strings.                                                                     |
| `cons`           | `(any '(any))`                        | Create a cons list with the first argument being the `car` and the second argument being the `cdr`.                  |
| `env`            | `(str)`                               | Get the first argument as an environment variable.                                                                   |
| `if`             | `(bool expr expr?)`                   | Evaluate the first expression if the predicate is true, or evaluate the second one if it exists.                     |
| `lambda`         | `((ident) expr ... expr)`             | Create an anonymous function with the specified arguments. It will return the last expression.                       |
| `let`            | `(((ident expr)? ...) expr ... expr)` | Create a new scope with the identifiers being bound to their expressions and evaluate the body.                      |
| `list`           | `(... any)`                           | Create a cons list using the arguments.                                                                              |
| `nil`            | `()`                                  | Return an empty list.                                                                                                |
| `not`            | `(bool)`                              | Reverse a boolean.                                                                                                   |
| `path`           | `(str)`                               | Convert a string to a path.                                                                                          |
| `path-children`  | `(path)`                              | Get a list of child nodes in a path.                                                                                 |
| `path-exists`    | `(path)`                              | Predicate for determining if a path exists.                                                                          |
| `path-is-dir`    | `(path)`                              | Predicate for determining if a path is a directory.                                                                  |
| `path-is-file`   | `(path)`                              | Predicate for determining if a path is a file.                                                                       |
| `path-name`      | `(path)`                              | Get the name part of a path as a string.                                                                             |
| `path-separator` | `()`                                  | Return the operating system's path separator.                                                                        |
| `progn`          | `(expr ... expr)`                     | Evaluate all arguments and return the last.                                                                          |
| `seq-filter`     | `((lambda '(any) bool) '(any))`       | Filter the list to only the elements that pass the predicate.                                                        |
| `seq-find`       | `((lambda '(any) bool) '(any))`       | Return the first element that passes the predicate.                                                                  |
| `seq-flat-map`   | `((lambda '(any) '(any)) '(any))`     | Maps each element and flattens them.                                                                                 |
| `seq-fold`       | `((lambda '(any) any) any '(any))`    | Folds each element. The first argument is the fold, the second is the accumulator and the final one is the elements. |
| `seq-map`        | `((lambda '(any) any) '(any))`        | Maps each element.                                                                                                   |
| `seq-rev`        | `('(any))`                            | Reverses a list.                                                                                                     |
| `try-catch`      | `((lambda '() any) (lambda '() any))` | Try the first function, if it fails call the second function.                                                        |

## `set-cfg!`

The `set-cfg!` function is a special function used to configure the music player.

### Arguments

`(str any)`

The first argument is the field you wish to configure, and the second is its value.

