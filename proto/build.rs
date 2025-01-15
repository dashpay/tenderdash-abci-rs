use std::env;

use tenderdash_proto_compiler::GenerationMode;

fn main() {
    // default Tenderdash version to use if TENDERDASH_COMMITISH is not set
    const DEFAULT_VERSION: &str = "f89ed874f295784f3c10759d0612b64ab8ae82cb";

    // check if TENDERDASH_COMMITISH is already set; if not, set it to the current
    // version
    let commitish = env::var("TENDERDASH_COMMITISH").unwrap_or_default();
    if commitish.is_empty() {
        env::set_var("TENDERDASH_COMMITISH", DEFAULT_VERSION);
    }

    #[cfg(feature = "grpc")]
    tenderdash_proto_compiler::proto_compile(GenerationMode::Grpc);
    // we always build nostd version
    tenderdash_proto_compiler::proto_compile(GenerationMode::NoStd);

    println!("cargo:rerun-if-changed=../proto-compiler/src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
    println!("cargo:rerun-if-env-changed=TENDERDASH_COMMITISH");
}
