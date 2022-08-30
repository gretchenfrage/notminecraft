## Packages

### `graphics`

This is the renderer.

By default, it bakes the `.spv` files in `src/shaders` into the binary.
However, if you build with the `shaderc` feature (requires shaderc installed on
your system) it will load and compile the GLSL files at runtime, for more rapid
iteration of shaders.

### `opentype437`

The renderer uses `ab_glyph`'s implementation of opentype font handling for
text layout and rasterization. However, minecraft's font is a "code page 437"
font, a rudimentary but simple to implement font system introduced by the
original 1982 IBM PC. This package provides a sort of adapter to allow a code
page 437 image to work as a freetype font.

### `ab_glyph`

This is my fork of the `ab_glyph` crate. I just needed to add a public
constructor to `CodepointIdIter`.

### `winit-main`

This is my fork of my own `winit-main` crate. I change it to make it async and
tokio-based, possibly some other tweaks.

### `minecraft`

This is supposed to contain the minecraft code. You need to download the
minecraft.jar file for game version beta 1.0.2 (some later versions will
probably work) and point to it with the `MINECRAFT_JAR` environment variable
when running.
