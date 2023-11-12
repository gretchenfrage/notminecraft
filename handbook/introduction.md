
# Not Minecraft Beta 1.0.2

Welcome! This is my project, 'Not Minecraft Beta 1.0.2', which aims to be a
reimplementation of Minecraft version beta 1.0.2 in Rust. I think this is a
space with many interesting intersecting architectural and algorithmic
problems, which is why I've been into the idea for quite some time now. I just
think it's fun!

## Links

- [Home page](https://phoenixkahlo.com/hubs/notminecraftbeta102)
- [GitLab repo](https://gitlab.com/gretchenfrage/notminecraft)
- [GitHub repo (updated infrequently)](https://github.com/gretchenfrage/notminecraft)

## Basic instructions

### Compile and run

```sh
cargo run
```

This still does a lot of optimization, but also leaves debug assertions on.

### Re-compile shaders

```sh
scripts/shaderc.sh
```

Since shaders rarely change, the compiled spirv bytecode is just committed to
git. That spirv gets baked into the binary when built, so the shaders must be
re-built if they are changed.

### Build handbook

```sh
mdbook build
```

This build the mdbook at `./handbook` to `./book` in HTML form. It explains
various architectural things about the project. Consider alternatively
`mdbook serve` or `mdbook serve --open`. Or you can just read the raw markdown
files.

### Build API docs

```sh
cargo doc
```

This builds the API docs to `./target/doc`. Considering opening
`./target/doc/minecraft/index.html` Or you can just read the source code in
`./packages` directly.
