#! /bin/bash

PLATFORM_DIR="$(realpath "$(dirname "$0")/../../platform")"

function help() {
    cat <<EOF
This script is used to release a new version of the project.
It will do the following:
- update the version in the Cargo.toml files
- if $PLATFORM_DIR exists, it will update the references to tenderdash-abci / tenderdash-proto in the Cargo.toml files
to use the new version

Usage:
   ./scripts/release.sh [--help|-h] [-t|--tenderdash] <tenderdash_version> [-a|--abci] <library_version>

Arguments:
   <tenderdash_version> - the version of Tenderdash to use
   <library_version> - the version of this library (rs-tenderdash-abci)

Examples:

   ./scripts/release.sh -t 0.14.0-dev.2 -a 0.14.0-dev.6
   ./scripts/release.sh -t 0.14.5 -a 0.14.12


EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    key="$1"
    case $key in
    -h | --help)
        help
        exit 0
        ;;
    -t | --tenderdash)
        shift
        if [ -z "$1" ]; then
            echo "Please specify the version of Tenderdash."
            exit 1
        fi
        td_version=$1
        shift
        ;;
    -a | --abci)
        shift
        if [ -z "$1" ]; then
            echo "Please specify the version of the library."
            exit 1
        fi
        rs_tenderdash_abci_version=$1
        shift
        ;;
    *)
        break
        ;;
    esac
done

# Check if the versions are passed.
if [ -z "$td_version" ]; then
    echo "Please specify the version of Tenderdash."
    exit 1
fi
td_version=${td_version#v} # remove 'v' if it exists

if [ -z "$rs_tenderdash_abci_version" ]; then
    echo "Please specify the version of the library."
    exit 1
fi

rs_tenderdash_abci_version=${rs_tenderdash_abci_version#v} # remove 'v' if it exists

set -ex
# Update the version in the Cargo.toml files.
sed -i "s/^version = .*/version = \"$rs_tenderdash_abci_version\"/" ./*/Cargo.toml
sed -i "s/^\s*const DEFAULT_VERSION: &str = \".*\";/const DEFAULT_VERSION: \&str = \"v$td_version\";/" ./proto/build.rs
cargo fmt -- ./proto/build.rs 2>/dev/null

if [ -d "$PLATFORM_DIR" ]; then
    rs_tenderdash="git = \"https:\/\/github.com\/dashpay\/rs-tenderdash-abci\", version = \"$rs_tenderdash_abci_version\" "
    echo "INFO: Updating references to tenderdash-abci / tenderdash-proto in $PLATFORM_DIR"

    sed -i "s/^tenderdash-abci = { git = .* }/tenderdash-abci = { $rs_tenderdash }/" "${PLATFORM_DIR}"/packages/*/Cargo.toml
    sed -i "s/^tenderdash-proto = { git = .* }/tenderdash-proto = { $rs_tenderdash }/" "${PLATFORM_DIR}"/packages/*/Cargo.toml
else
    echo "WARN: Dash Platform not found in $PLATFORM_DIR, skipping"
fi
