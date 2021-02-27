#!/usr/bin/env bash

RUSTFLAGS=--cfg=web_sys_unstable_apis wasm-pack build --target web $*
