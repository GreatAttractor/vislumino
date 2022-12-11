# Vislumino - Astronomy Visualization Tools
Copyright (C) 2022 Filip Szczerek (ga.software@yahoo.com)

version 0.1.0 (2022-12-11)

*This program comes with ABSOLUTELY NO WARRANTY. This is free software, licensed under GNU General Public License v3 and you are welcome to redistribute it under certain conditions. See the LICENSE file for details.*

----------------------------------------

- 1\. Introduction
- 2\. Features
- 3\. Building
  - 3\.1\. Linux and alikes
  - 3\.2\. MS Windows

----------------------------------------

## 1. Introduction

Vislumino is a tool for creating visualizations of astronomical data.

Homepage: https://greatattractor.github.io/vislumino


## 2. Features

### 2.1. Planetary projection

Tutorial: [link](https://greatattractor.github.io/vislumino/tutorials/planetary_projection/index.html)

Demonstration video: [link](https://www.youtube.com/watch?v=w_k1WWCmGpw)

## 3. Building

Clone the repository:
```Bash
$ git clone --recurse-submodules https://github.com/GreatAttractor/vislumino.git
```


### 3.1. Linux and alikes

Install the [Rust toolchain](https://www.rust-lang.org/learn/get-started). To build Vislumino, run:
```Bash
$ cargo build --release
```

To build & launch:
```Bash
$ cargo run --release
```

To run tests:
```Bash
$ cargo test
```


### 3.2. MS Windows

Building under MS Windows has been tested in [MSYS2](https://www.msys2.org/) environment and the GNU variant of the [Rust toolchain](https://www.rust-lang.org/learn/get-started).

Download MSYS2 from http://www.msys2.org/ and follow its installation instructions. Then install the Rust toolchain: go to https://forge.rust-lang.org/infra/other-installation-methods.html and install the `x86_64-pc-windows-gnu` variant. The warnings about "Visual C++ prerequisites" being required and "Install the C++ build tools before proceeding" can be ignored. Note that you must customize "Current installation options" and change the "default host triple" to "x86_64-pc-windows-gnu".

Open the "MSYS2 MinGW 64-bit" shell (from the Start menu, or directly via `C:\msys64\msys2_shell.cmd -mingw64`), and install the build prerequisites:
```bash
$ pacman -S git base-devel mingw-w64-x86_64-toolchain
```

Pull Rust binaries into `$PATH`:
```bash
$ export PATH=$PATH:/c/Users/MY_USERNAME/.cargo/bin
```
then change to the Vislumino source directory and build it using commands as in sect. 3.1.

After a successful build, Vislumino can be run locally with:
```bash
$ target/release/vislumino.exe
```

*Upcoming: creating a binary distribution*.
