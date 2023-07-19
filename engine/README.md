# How I achieved fast compile times and automatic code reloading

## Fast compile times through dynamic linking

This was my initial goal - waiting ten seconds to see the effect of a color
change was annoying, and loading them from a config file seemed more
complicated and would make the game not a standalone binary.

Linking was the most time-consuming part of the build.

First I tried just moving all the dependencies into a library that exposed the
parts that I needed. That didn't help. Next step was to make the library a
dynamic library, but for that to help / work nothing can be generic.
I therefore added wrappers around the generic functions.

To make sure the exported surface is as minimal as possible, I also replaced
any public type with custom reimplementations: structs are not simple wrappers
/ newtypes. traits were replaced with custom ones and the structs that
implements them only exposed as trait objects (which makes it safe for them to
contain piston types).

This helped quite a bit (numbers in commit messages I think)

## Reloading of code

With the small surface, I got the idea to load the main binary as a dynamic
library at runtime so that code changes could take effect immediately.
It's not that valuable in this game which has so little state though.

When a new version is loaded the previous code is not unloaded, as that cannot
be made safe (it breaks `'static` lifetimes). This creates a leak, but that's
OK because i'm not going to reload it hundreds of times.

If the in-memory data structures change, the game might crash or run into UB,
but that's just a minor annoyance as you're the only one affected.
Still, I've added a small sanity check by storing the size of the game data struct.
That won't detect changes in a referenced struct though.

## Calling `rustc` directly

With dynamic linking, `cargo build` was still not instant, and I noticed that
running the rustc command shown by `cargo build --verbose` was noticably faster.
Parsing the `--verbose` output seemed brittle and complicated though, so I
looked for other ways. cargo has an unstable --build-plan flag, which prints
a big json object. I extracted the last rustc invocation from that, and it worked!
I noticed that the generated command differed from what cargo --verbose printed
though: Turns out that using the build plan requires you to run all the build
scripts and parse their output! And so I did, collecting the one relevant line.
Running the build scripts takes time though, about as much as a cargo build without
the dynamic library. (Caching the info in memory was trivial, and was already done
with the build plan). The final step was caching the final command to disk,
and having a way to invalidate that cache. (Cargo.lock timestamp and compiler version)

## Remaining issues

Switching between dynamic library for development and standalone executable
for release cannot be done with a cargo flag but requires changes to `Cargo.toml`
of the library: This is because [cargo doesn't allow overriding the crate type](https://github.com/rust-lang/cargo/issues/4881):
It ignores `crate-type` entries in `[profile.*]`s or
target-specifig `[lib]` sections (`[target.*.lib]`), `#![crate_type=]`
at the source root and `RUSTFLAGS` environment variables.

Asset reloading is a big TODO, because this game doesn't need it.
