{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation {
  pname = "calendar-egui";
  version = "0.1.0";
  src = ./.;

  nativeBuildInputs = with pkgs; [
    autoPatchelfHook
    addDriverRunpath
  ];

  buildInputs = [
    pkgs.stdenv.cc.cc.lib
  ];

  runtimeDependencies = with pkgs; [
    libxkbcommon
    libGL
    libglvnd      
    vulkan-loader

    # wayland support
    wayland

    # x11 support
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    xorg.libxcb
  ];

  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp target/release/calendar-egui $out/bin/
    runHook postInstall
  '';

  postFixup = ''
    addDriverRunpath $out/bin/calendar-egui
  '';
}