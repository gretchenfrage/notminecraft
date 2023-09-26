#!/usr/bin/env bash
set -e
cd graphics/src/shaders
for f in *.vert *.frag
do
    ~/shaderc/install/bin/glslc $f -o "../shaders_spirv/${f}.spv" -I .
done
