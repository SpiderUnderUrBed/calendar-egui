{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation {
  name = "calendar-egui";
  nativeBuildInputs = [ pkgs.autoPatchelfHook pkgs.libgcc ];
  
  src = ./.;

  installPhase = ''
    mkdir -p $out/bin
    cp target/release/calendar-egui $out/bin/
  '';

  runtimeDependencies = with pkgs; [
    libxkbcommon
    libGL
    #libgcc
 
    # wayland support
    wayland

    # x11 support
    libX11
    libXcursor
    libXi
  ];
}
