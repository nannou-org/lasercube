# lasercube-core

Core types and constants for the LaserCube network protocol.

## Overview

`lasercube-core` provides the fundamental data structures and protocol
definitions for communicating with LaserCube devices.

## Features

- Core protocol definitions
- Type-safe command and data structures
- Buffer management utilities
- Ready for use with standard network libraries

## Example

```rust
use lasercube_core::{
    Command, SampleData, Point,
    consts::MAX_POINTS_PER_MESSAGE,
};

// Create a point
let point = Point::from_normalized(0.0, 0.0, 1.0, 0.5, 0.0);

// Create a sample data packet
let sample_data = SampleData::new(0, 0, vec![point]);

// Create a command to send the sample data
let command = Command::SampleData(sample_data);
```

## Safety Notes

This crate provides the basic building blocks for communicating with LaserCube
devices, which are physical laser devices that can present safety risks if
misused. When implementing network communication using this crate, please
ensure appropriate safety measures are in place.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
