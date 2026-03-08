# The Standard Library
`Opal`'s standard library is shipped with `Loupe` and it's version kept in lockstep. One release / version of 'Loupe' will always use the same standard library.

## Imports
To get started with the standard library we first need to introduce a new concept: imports. `Opal` defined the keyword `use`. Like everything in `Opal` this lives inside an S-expression.

`(use std)` at the top of the file brings the module `std` into scop.

Here's an example:

```
(use std/io)

(let main {}
  (io/println "hello")
  (io/println (string/to_upper "hello")))
```

`(use std/io)` imports a single module `io`, and brings it's functions and types into scope in a qualified manner (with the `io/` prefix).

If you only want to bring in a subset of what's defined in a module and use it in an un-qualified manner you can use square brackets to do so.

```
(use std/io [println])

(let main {}
  (println "hello"))
```

The standard library also provides some useful types like `Option` and `Result`. It is idiomatic to import these in an un-qualified manner. This also imports the constructors like `None` and `Some`.

```opal
(use std/result [Result])
(use std/Option [Option])
```

The language also provides some syntactic sugar like `let?`. `let?` is a monadic bind. It requires a `bind` function in scope and chains operations that return a `Result`, short-circuiting on the first error. This syntax can be used simply with `(use std/result [Result bind])`.

```opal
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

This desugars to `(bind (might_fail) (fn {a} (bind (also_might_fail a) (fn {b} (Ok (+ a b))))))` and if you run it you'll see 

```shell
10
20
```

