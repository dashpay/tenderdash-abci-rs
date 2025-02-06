use std::env;

use tenderdash_proto_compiler::GenerationMode;

fn main() {
    // default Tenderdash version to use if TENDERDASH_COMMITISH is not set
    const DEFAULT_VERSION: &str = "a4de86a0008ad817e087cbea99cafd4b51089cdd";

    // check if TENDERDASH_COMMITISH is already set; if not, set it to the current
    // version
    let commitish = env::var("TENDERDASH_COMMITISH").unwrap_or_default();
    if commitish.is_empty() {
        env::set_var("TENDERDASH_COMMITISH", DEFAULT_VERSION);
    }

    // build gRPC server and client
    // note it should be safe to build both server and client; we will just not use
    // them in the lib.rs

    #[cfg(feature = "server")]
    // build gRPC server (includes client)
    tenderdash_proto_compiler::proto_compile(GenerationMode::GrpcServer);
    #[cfg(feature = "client")]
    // build gRPC client only
    tenderdash_proto_compiler::proto_compile(GenerationMode::GrpcClient);
    // we always build nostd version
    tenderdash_proto_compiler::proto_compile(GenerationMode::NoStd);

    println!("cargo:rerun-if-changed=../proto-compiler/src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
    println!("cargo:rerun-if-env-changed=TENDERDASH_COMMITISH");
}
