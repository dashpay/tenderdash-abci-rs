# Rust bindings and server for Tenderdash ABCI++ protocol

## Introduction

![example workflow](https://github.com/dashpay/rs-tenderdash-abci/actions/workflows/test.yml/badge.svg?branch=master)

This repository contains Rust bindings for Tenderdash. It includes:

* data types, requests and responses required on [Tenderdash]
* ABCI++ protocol server, supporting **Unix sockets** and **TCP** connections
* [tonic](https://docs.rs/tonic/latest/tonic/)-based ABCI++ protocol client/server, supporting grpc connections

## Structure

The repository contains the following crates:

* [tenderdash-abci](./abci/) - main crate, including ABCI++ socket and tcp server implementation, `Application` trait and re-exporting `tenderdash-proto` crate
* [tenderdash-proto](./proto/) - ABCI++ messages and data types definitions, and gRPC client/server implementation, generated based on Tenderdash protobuf specifications
* [tenderdash-proto-compiler](./proto-compiler/) - an internal tool that fetches tenderdash repository and converts protobuf files to Rust

## Version Compatibility

Versioning of this library follows the Semantic Versioning 2.0.0 specification. Specifically, it consists of
`MAJOR.MINOR.PATCH+BUILD`, where `BUILD` denotes minimum version of [Tenderdash] required.

For instance, if you're working with `Tenderdash 1.3.0`, you should use `rs-tenderdash-abci 1.2.0+1.3.0`.

This library also includes built-in support for ABCI protocol version verification. The ABCI protocol version, as defined in Tenderdash's [version.go](https://github.com/dashpay/tenderdash/blob/HEAD/version/version.go) under the `ABCISemVer` constant, must align with the ABCI protocol version of this library. You can find the library's ABCI protocol version in [proto/src/tenderdash.rs](proto/src/tenderdash.rs) under the `ABCI_VERSION` constant.

## Quick start

1. Install dependencies. You can find a current list of dependencies in the [Dockerfile](Dockerfile-debian).
   * Ensure to install `protoc` from <https://github.com/protocolbuffers/protobuf/releases> and place it in your `$PATH`, as we
   **don't support protoc shipped with your distribution** (like Ubuntu).
2. Add tenderdash-abci crate to your project:

    ```bash
    cargo add --git https://github.com/dashpay/rs-tenderdash-abci tenderdash-abci
    ```

3. Implement the [Application](abci/src/application.rs) trait with your custom logic. You can check [kvstore](abci/tests/kvstore.rs) as a minimal example.

## Using custom protocol buffers definitions

If you want to build `rs-tenderdash-abci` using protocol buffers definitions from a custom Tenderdash version, you can do so by setting the `TENDERDASH_COMMITISH` environment variable to the desired Tenderdash commit ID before initiating the build process.

For instance, if you want to build the library with support for Tenderdash `v0.14-dev.3`, which corresponds to the commit ID `688ee3e3f2624e6ebb20f5d74e0812109b7b9a27`, you can use the following command:

```bash
export TENDERDASH_COMMITISH=688ee3e3f2624e6ebb20f5d74e0812109b7b9a27
cargo build
```

## Credits

This project is a partial fork of [tendermint-rs] project.

## License

[MIT](LICENSE.md)

[Tenderdash]: https://github.com/dashpay/tenderdash
[tendermint-rs]: https://github.com/informalsystems/tendermint-rs
