# Erlang FFI
One of the big benefits of targeting the BEAM is being able to leverage a rich and mature ecosystem.

We can do so by binding Erlang functions to `Zier` names with `extern let`. We said earlier that `Zier` does not use type signatures, this was a little white lie. `extern` declarations are the only place where `Zier` uses type signatures.

```
(extern let system-time ~ (Unit -> Int) erlang/system_time)
```

`pub extern let` makes the binding importable by other modules — this is how large parts of the standard library are implemented e.g.

```
(pub extern let println ~ (String -> Unit) io/format)
```

We can do something very for types e.g. using erlang's dict.

```opal
(pub extern type ['k 'v] Map maps/map)
```
