//! A crate designed for communicating with LaserCube lasers.

pub use client::Client;
pub use lasercube_core as core;

pub mod client;
pub mod discover;
