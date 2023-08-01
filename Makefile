.PHONY: check run dev clippy wasm

check:
	cargo check
	cargo check --features dyn

run:
	cargo run

dev:
	cargo run --features dyn

clippy:
	cargo clippy --features dyn

wasm:
	cargo build --target wasm32-unknown-unknown
	wasm-bindgen target/wasm32-unknown-unknown/debug/space_tennis.wasm --out-dir wasm --target web
