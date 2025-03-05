# lasercube

A Rust implementation of the LaserCube network protocol for controlling
LaserCube devices.

## Overview

This workspace provides a set of crates for communicating with LaserCube
devices over the network. LaserCube is a portable laser projector created by
LaserOS (https://www.laseros.com). This library allows you to control LaserCube
devices programmatically using Rust.

## Crates

- **lasercube-core**: Core types, constants, encodings and decodings for the
  LaserCube protocol.
- **lasercube**: Main library providing high-level interfaces for discovering
  and controlling LaserCube devices.

## Usage

See the `examples/` directory for a demonstration.

## Protocol Documentation

The LaserCube protocol was derived by reading [this python
implementation][python-impl].

See [SPECIFICATION.md](./SPECIFICATION.md).

## Acknowledgments

- Original python implementation by Sidney San Martín [here][python-impl].

[python-impl]: https://gist.github.com/s4y/0675595c2ff5734e927d68caf652e3af
