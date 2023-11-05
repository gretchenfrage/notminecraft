#!/usr/bin/env bash

# this script builds and compiles all documentation to handbook/book in HTML form

set -e
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "${SCRIPT_DIR}/.."

(cd handbook; mdbook build)
(cd workspace; cargo doc)
cp -R workspace/target/doc handbook/book/api
cp workspace/graphics/architecture.pdf handbook/book/graphics.pdf

echo "built handbook to $(pwd)/handbook/book"
firefox "$(pwd)/handbook/book/index.html"
