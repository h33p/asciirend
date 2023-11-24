#!/bin/sh

wasm-pack build --target web --no-typescript --no-pack --features bindings,wasm
cargo build --target wasm32-wasi --release --features=bindings
