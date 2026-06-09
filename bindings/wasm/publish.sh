#!/usr/bin/env bash
# Build + publish the wasm binding to npm as @tharikey/engine.
#
# wasm-pack regenerates pkg/package.json from Cargo.toml on every build, so the scoped npm name (which
# a crate name can't express) and the npm keywords are re-applied here. README, version, description,
# repository and homepage all come from the crate (README.md + Cargo.toml) automatically.
#
# Bump the version in Cargo.toml before publishing a new release (npm won't overwrite an existing one).
# Run from the engine repo root:  bash bindings/wasm/publish.sh
set -euo pipefail

wasm-pack build bindings/wasm --release --target bundler --out-dir pkg --out-name tharikey

cd bindings/wasm/pkg
npm pkg set name='@tharikey/engine'
npm pkg set keywords[0]=thaana keywords[1]=dhivehi keywords[2]=transliteration \
            keywords[3]=keyboard keywords[4]=ime keywords[5]=maldives

# Idempotent: skip if this version is already on npm (so a release tag that doesn't bump the wasm
# version is a no-op instead of a hard failure).
VER=$(npm pkg get version | tr -d '"')
if npm view "@tharikey/engine@${VER}" version >/dev/null 2>&1; then
  echo "@tharikey/engine@${VER} already on npm — nothing to publish."
  exit 0
fi

npm publish --access public
