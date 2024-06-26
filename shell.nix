# https://eu90h.com/wgpu-winit-and-nixos.html
# https://github.com/gfx-rs/wgpu/issues/3033

{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  shellHook = ''
    LD_LIBRARY_PATH="''${LD_LIBRARY_PATH:+$LD_LIBRARY_PATH:}${
      with pkgs;
        lib.makeLibraryPath [
          vulkan-loader
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          libxkbcommon
        ]
    }"
    export LD_LIBRARY_PATH
  '';
}
