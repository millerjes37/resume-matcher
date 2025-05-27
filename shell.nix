# shell.nix
# This file defines a development environment for the resume-matcher Rust project.
# To use it, navigate to your project directory in the terminal and run:
# nix-shell

# Import nixpkgs. You can pin this to a specific version for more reproducible builds.
# For example: pkgs = import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/nixos-23.11.tar.gz") {};
{ pkgs ? import <nixpkgs> {} }:

let
  # Use LLVM/Clang provided by Nixpkgs for C/C++ compilation.
  # We'll refer to the specific LLVM toolchain package set.
  llvmPkgs = pkgs.llvmPackages_latest; # Or a specific version like pkgs.llvmPackages_17
  
  # For macOS, we might need specific SDK headers
  darwinSystem = pkgs.darwin.apple_sdk.frameworks;
in

pkgs.mkShell {
  # Name for the shell (optional)
  name = "resume-matcher-dev-shell";
  
  # Packages to make available in the development shell.
  nativeBuildInputs = [
    pkgs.rustc              # Rust compiler
    pkgs.cargo              # Rust package manager and build tool
    pkgs.cmake              # CMake, required by xgboost-rs-sys and other native builds
    pkgs.pkg-config         # Helper tool for finding compiler and linker flags for libraries
    llvmPkgs.clang          # Clang C/C++ compiler from the chosen LLVM package set
    llvmPkgs.openmp         # OpenMP runtime library from LLVM
    pkgs.typst              # Typst CLI for compiling .typ files to PDF
    # Add other build tools if your project acquires more native dependencies
  ];
  
  buildInputs = [
    # Libraries that your Rust code might link against.
    llvmPkgs.openmp         # OpenMP runtime library from LLVM
  ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
    # On macOS, ensure system frameworks are available for linking if needed by native code.
    darwinSystem.Security
    darwinSystem.SystemConfiguration
  ];
  
  # Environment variables to set in the shell.
  shellHook = ''
    export CC="${llvmPkgs.clang}/bin/clang"
    export CXX="${llvmPkgs.clang}/bin/clang++"
    export AR="${llvmPkgs.clang}/bin/llvm-ar"
    export NM="${llvmPkgs.clang}/bin/llvm-nm"
    export RANLIB="${llvmPkgs.clang}/bin/llvm-ranlib"
    
    # Set library path for OpenMP, might be needed by some build scripts or at runtime for tests
    export LIBRARY_PATH="${llvmPkgs.openmp}/lib:$LIBRARY_PATH"
    
    # For macOS, also set:
    export DYLD_LIBRARY_PATH="${llvmPkgs.openmp}/lib:$DYLD_LIBRARY_PATH"
    
    echo "Entered resume-matcher Nix development shell."
    echo "Rust toolchain, Typst, and dependencies (like OpenMP for XGBoost) are now available."
    echo "Compiler set to Clang from Nixpkgs."
  '';
}