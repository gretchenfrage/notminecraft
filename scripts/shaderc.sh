#!/usr/bin/env bash

# this script compiles all the GLSL shaders in the graphics module into spirv
# bytecode. the spirv bytecode gets baked into the binary upon compilation. if
# the shaders are changed, they must be rebuilt with this script and then the
# rust stuff must be recompiled for the changes to be reflected.
#
# this script requires the glslc binary to be in the path. for example on linux
# try following these instructions:
#
# 1. download a shaderc release from
#    https://github.com/google/shaderc/tags
# 2. unpack it to where you want to install it, eg /home/youruser/shaderc
# 3. add its bin directory to your path, eg by adding this line to your ~/.bashrc:
#        export PATH="/home/youruser/shaderc/install/bin:${PATH}"
#    and then sourcing it so it takes effect:
#        source ~/.bashrc
#
# alternatively, building the project with the `--features shaderc` flag makes
# the graphics package try to link in shaderc into itself and read glsl files
# from the source tree whenever it runs and then compile them then and there
# rather than baking the pre-compiled spirv into the binary at compile time.
# this can be nice if you're doing a lot of changes to the shaders and want a
# tighter feedback loop.

set -e
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "${SCRIPT_DIR}/../workspace/graphics/src/shaders"

for f in *.vert *.frag
do
    glslc $f -o "../shaders_spirv/${f}.spv" -I .
done
