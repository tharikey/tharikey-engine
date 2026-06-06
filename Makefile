# ThariKey engine — build tasks. Designed to run locally and on CI.
#
# Docs are plain markdown under docs/ (docs-as-code) — the tharikey.com website renders them; there is
# no site build here. The wasm playground bundle is built by the website from bindings/wasm.

.PHONY: help test lint wasm c-header all clean

help:
	@echo "make test      — cargo test (core + bindings type-check)"
	@echo "make lint      — cargo clippy + fmt check"
	@echo "make wasm      — build the wasm bundle (bindings/wasm/pkg)"
	@echo "make c-header  — (re)generate the C ABI header (bindings/c/include/tharikey.h)"
	@echo "make all       — test + lint"
	@echo "make clean     — remove build outputs"

test:
	cargo test
	cargo check -p tharikey-c -p tharikey-py -p tharikey-wasm

lint:
	cargo clippy --workspace --all-targets -- -D warnings
	cargo fmt --check

# Requires wasm-pack (cargo install wasm-pack) and the wasm32-unknown-unknown target.
wasm:
	cd bindings/wasm && wasm-pack build --target web --out-dir pkg --no-typescript

c-header:
	cargo build -p tharikey-c

all: test lint

clean:
	cargo clean
	rm -rf bindings/wasm/pkg
