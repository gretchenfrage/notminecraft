
# Not Minecraft Beta 1.0.2

Welcome! This is my project, 'Not Minecraft Beta 1.0.2', which aims to be a
reimplementation of Minecraft version beta 1.0.2 in Rust. I think this is a
space with many interesting intersecting architectural and algorithmic
problems, which is why I've been into the idea for quite some time now. I just
think it's fun!

![screenshot](https://phoenixkahlo.com/images/notminecraftbeta102.png)

## Links

- [Home page](https://phoenixkahlo.com/hubs/notminecraftbeta102)
- [GitLab repo](https://gitlab.com/gretchenfrage/notminecraft)
- [GitHub repo (updated infrequently)](https://github.com/gretchenfrage/notminecraft)

## Handbook

```sh
mdbook build
```

Then open `book/index.html`. This builds the "handbook" containing various
architecture explanations. Or you can just read the raw markdown files in
`handbook`.


## Compile and run

```sh
cargo run
```

This still does a lot of optimization, but also leaves debug assertions on. A
full release build can be done with `cargo run --release`.

## Re-compile shaders

```sh
scripts/shaderc.sh
```

Since shaders rarely change, the compiled spirv bytecode is just committed to
git. That spirv gets baked into the binary when built, so the shaders must be
re-built if they are changed. Or you can build with `--features shaderc` which
bakes shaderc into the binary.

## Build API docs

```sh
cargo doc
```

This builds the API docs to `./target/doc`. Considering opening
`./target/doc/minecraft/index.html` Or you can just read the source code in
`./packages` directly.
