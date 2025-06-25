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

## Default configuration

An example configuration file is provided at [./main.lisp](./main.lisp). It will also be automatically copied into the path above if it does noe exist.

The configuration must also be a singular expression, so if you need multiple expressions, put everything in a `progn`.

The default configuration file requires you to put files in the format of `~/Music/PLAYLIST/SONG` where `PLAYLIST` is the name of a playlist and `SONG` is a song in the playlist.

For example, if you had files in the structure of:

 - ~/Music/playlist1/
 - ~/Music/playlist1/song1.mp3
 - ~/Music/playlist1/song2.mp3
 
This would create a playlist named `playlist1` with the songs `song1.mp3` and `song2.mp3`.

## Builtin functions

Argument types are listed in a pseudo s-expression format where every item in the list is the item type.

Any argument following a `...` is considered to be variadic, so `(int int ... int)` accepts two or more arguments.

Any argument with a `?` indicates it is optional.

Any argument between `|` indicates it can be either.

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

These have their own types.

 - `color` indicates a [color][#color]
 - `modifiers` indicates a [modifier][#key-modifier]
 - `key-code` indicates a [key code][#key-code]
 - `key-action` indicates a [key action][#key-action]

### Arguments

`(str any)`

The first argument is the field you wish to configure, and the second is its value.

| field              | type                              | description                   |
|--------------------|-----------------------------------|-------------------------------|
| `cursor-colors`    | `'(color)`                        | Set the cursor colors.        |
| `menu-colors`      | `'(color)`                        | Set the menu colors.          |
| `selection-colors` | `'(color)`                        | Set the selection colors.     |
| `playlists`        | `'(str '('(path string))`         | Set the playlists to be used. |
| `key-bindings`     | `'(str '('(modifiers key-code)))` | Set the key bindings.         |
|                    |                                   |                               |

#### Color

The colors can be one of the following:

 - `none`
 - `reset`
 - `black`
 - `dark_grey`
 - `red`
 - `dark_red`
 - `green`
 - `dark_green`
 - `yellow`
 - `dark_yellow`
 - `blue`
 - `dark_blue`
 - `magenta`
 - `dark_magenta`
 - `cyan`
 - `dark_cyan`
 - `white`
 - `grey`

#### Key modifier

Key modifiers are a sequence of letters that accumulate to a key modifier.

| letter | key        |
|--------|------------|
| `a`    | alt        |
| `c`    | control    |
| `l`    | super/logo |
| `h`    | hyper      |
| `m`    | meta       |
| `s`    | shift      |

For example, `sa` would be a modifier that requires shift and alt to be pressed.

#### Key action

Key actions are the actions that get executed once a key binding is performed.

| name             | description                            |
|------------------|----------------------------------------|
| `quit`           | Halt the program.                      |
| `move-up`        | Move the cursor up.                    |
| `move-down`      | Move the cursor down.                  |
| `move-left`      | Move the cursor left.                  |
| `move-right`     | Move the cursor right.                 |
| `move-top`       | Move the cursor to the top.            |
| `move-bottom`    | Move the cursor to the bottom.         |
| `move-selection` | Move the cursor to the selected index. |
| `select`         | Select the item under the cursor.      |
| `skip-song`      | Skip the current song.                 |

#### Key code

 - `backspace`
 - `enter`
 - `left`
 - `right`
 - `up`
 - `down`
 - `home`
 - `end`
 - `page-up`
 - `page-down`
 - `tab`
 - `back-tab`
 - `delete`
 - `insert`
 - `null`
 - `esc`
 - `caps-lock`
 - `scroll-lock`
 - `num-lock`
 - `print-screen`
 - `pause`
 - `menu`
 - `keypad-begin`
 - `media-play`
 - `media-pause`
 - `media-play-pause`
 - `media-reverse`
 - `media-stop`
 - `media-fast-forward`
 - `media-rewind`
 - `media-track-next`
 - `media-track-previous`
 - `media-record`
 - `media-lower-volume`
 - `media-raise-volume`
 - `media-mute-volume`
 - `left-shift`
 - `left-control`
 - `left-alt`
 - `left-super`
 - `left-hyper`
 - `left-meta`
 - `right-shift`
 - `right-control`
 - `right-alt`
 - `right-super`
 - `right-hyper`
 - `right-meta`
 - `iso-level-3-shift`
 - `iso-level-5-shift`
 - `f\d+`
 The `f\d+` keys.
 - `.`
 Assume the key is a character.
