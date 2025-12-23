.PHONY: fmt clippy build check clean

fmt:
	cargo fmt

clippy:
	cargo clippy --all-targets -- -D warnings

build:
	cargo build --release

check:
	cargo check

clean:
	cargo clean
