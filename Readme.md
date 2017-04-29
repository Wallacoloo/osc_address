## OSC\_address

Provides tools to represent an Open Sound Control (OSC) message in a typesafe
enum-based format that provides for easy message routing and efficient
serialization when coupled with [serde\_osc](https://crates.io/crates/serde_osc).


## Usage

Usage examples are yet to be written. Routing an `OscMessage` type should be a fairly
self-explanatory process based on enum matching. To derive `OscMessage` over a type,
consult the `tests/` directory within [osc\_address\_derive](https://github.com/Wallacoloo/osc_address_derive).


## Documentation

Documentation can be found at [docs.rs/osc\_address](https://docs.rs/osc_address).


## License

OSC\_address is licensed under either of

   * Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0)
   * MIT license (http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in OSC\_address by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
