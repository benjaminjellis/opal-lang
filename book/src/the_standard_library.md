# The Standard Library
`Zier`'s standard library is shipped with the CLI and its version is kept in lockstep. One release/version of the CLI will always use the same standard library.

The standard library is intended to provide the well tested building blocks the language and ecosystem needs to grow and succeed.

## Imports
To get started with the standard library, we first need to introduce a new concept: imports. `Zier` defines the keyword `use`. Like everything in `Zier`, this lives inside an S-expression.

`(use std)` at the top of the file brings the module `std` into scope.

Here's an example:

```
(use std/io)

(let main {}
  (io/println "hello")
  (io/println (string/to_upper "hello")))
```

`(use std/io)` imports a single module `io`, and brings its functions and types into scope in a qualified manner (with the `io/` prefix).

If you only want to bring in a subset of what's defined in a module and use it in an unqualified manner, you can use square brackets to do so.

```
(use std/io [println])

(let main {}
  (println "hello"))
```


## Monadic Types

The standard library also provides some useful types like `Option` and `Result`. It is idiomatic to import these in an unqualified manner. This also imports constructors like `None` and `Some`.

```zier
(use std/result [Result])
(use std/Option [Option])
```

The language also provides some syntactic sugar like `let?`. `let?` is a monadic bind. It requires a `bind` function in scope and chains operations that return a `Result`, short-circuiting on the first error. This syntax can be used simply with `(use std/result [Result bind])`.

```zier
(use std/result [Result bind])
(use std/io)

(let might_fail {} (Ok 10))

(let might_also_fail {x} (Ok (+ x 10)))

(let main {}
  (let? [a (might_fail) b (might_also_fail a)]
    (do (io/debug a)
        (io/debug b)
        (Ok (+ a b)))))
```

This desugars to `(bind (might_fail) (f {a} -> (bind (also_might_fail a) (f {b} -> (Ok (+ a b))))))` and if you run it, you'll see:

```shell
10
20
```

## Processes
Because `Zier` targets the `BEAM` we can leverage it's model of concurrency. The basic building block of this are `processes`. 

## Unknown
