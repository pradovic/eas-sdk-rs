# How to contribute

The contributions are welcome! 

There are just a few small guidelines you need to follow.

1. Code should be `cargo fmt` formatted.
2. Code should pass `cargo clippy`.
3. Exported types, constants, variables and functions should be documented.
5. All tests must pass constantly `cargo test`.

## Versioning

Ethereum Attestation Service Rust client follows semantic versioning. New functionality should be accompanied by increment to the minor version number.

## Releasing

Any code which is complete, tested, reviewed, and merged to master can be released.

1. Make a pull request with changes.
2. Once the pull request has been merged, visit [https://github.com/pradovic/eas-sdk-rs/releases](https://github.com/pradovic/eas-sdk-rs/releases) and click `Draft a new release`.
3. Update the `Tag version` and `Release title` field with the new Ethereum Attestation Service Rust client version. Be sure the version has a `v` prefixed in both places, e.g. `v0.25.0`.
4. Publish the release.
