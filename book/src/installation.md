# Installation

Currently, the only way to install `Zier` is from source.

`Zier`'s compiler is written in Rust. To install it, you'll need a `Rust` toolchain. To run `Zier` code, you'll need to install `erlang`, and to create a release you'll need `rebar3`.

To install everything on Arch Linux, run the following:

```
sudo pacman -S rustup erlang rebar3
```

You should be able to do something similar on macOS with:

```
brew install rustup erlang rebar3
```

Once you have those installed, you can clone this repo and run this in the root:
```

cargo install --path zier 
```

Then you'll be able to use `Zier`.

