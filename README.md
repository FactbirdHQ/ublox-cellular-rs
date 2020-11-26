# ublox-cellular

> no_std driver crate for interfacing with the ublox cellular family over serial

![Test][test]
[![Code coverage][codecov-badge]][codecov]
![No Std][no-std-badge]
<!--
[![Crates.io Version][crates-io-badge]][crates-io]
[![Crates.io Downloads][crates-io-download-badge]][crates-io-download]
-->

---

A driver crate for AT-command based serial ublox cellular modules, built on top of [atat].

[atat]: https://crates.io/crates/atat


## [Documentation](https://docs.rs/ublox-cellular-rs/latest)

Relevant docs:
- https://www.u-blox.com/en/docs/UBX-20015573
- https://www.u-blox.com/en/docs/UBX-13001820

Relevant repos:
- https://github.com/u-blox/cellular
- https://github.com/ARMmbed/mbed-os/blob/master/connectivity/drivers/cellular


## Tests

> The crate is covered by tests. These tests can be run by `cargo test --tests --all-features`, and are run by the CI on every push.


## Examples
The crate has examples for running it on a linux platform.

The samples can be built using `cargo build -p linux_example --target x86_64-unknown-linux-gnu`, and similarly run using `cargo run`


## Features

- device selection (must select one, and only one!):
  - `topy_l4`
  - `mpci_l2`
  - `lisa_u2`
  - `sara_g3`
  - `sara_g4`
  - `sara_u2`
  - `sara_u1`
  - `toby_l2`
  - `toby_r2`
  - `lara_r2`
  - `leon_g1`
- `socket-tcp`: Enabled by default. Adds TCP socket capabilities, and implements [`TcpStack`] trait.
- `socket-udp`: Enabled by default. Adds UDP socket capabilities, and implements [`UdpStack`] trait.
- `defmt-default`: Disabled by default. Add log statements on trace (dev) or info (release) log levels to aid debugging.
- `defmt-trace`: Disabled by default. Add log statements on trace log levels to aid debugging.
- `defmt-debug`: Disabled by default. Add log statements on debug log levels to aid debugging.
- `defmt-info`: Disabled by default. Add log statements on info log levels to aid debugging.
- `defmt-warn`: Disabled by default. Add log statements on warn log levels to aid debugging.
- `defmt-error`: Disabled by default. Add log statements on error log levels to aid debugging.


## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
 http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.


<!-- Badges -->
[test]: https://github.com/BlackbirdHQ/ublox-cellular-rs/workflows/Test/badge.svg
[no-std-badge]: https://img.shields.io/badge/no__std-yes-blue
[codecov-badge]: https://codecov.io/gh/BlackbirdHQ/ublox-cellular-rs/branch/master/graph/badge.svg
[codecov]: https://codecov.io/gh/BlackbirdHQ/ublox-cellular-rs
<!--
[crates-io]: https://crates.io/crates/ublox-cellular-rs
[crates-io-badge]: https://img.shields.io/crates/v/ublox-cellular-rs.svg?maxAge=3600
[crates-io-download]: https://crates.io/crates/ublox-cellular-rs
[crates-io-download-badge]: https://img.shields.io/crates/d/ublox-cellular-rs.svg?maxAge=3600
-->
