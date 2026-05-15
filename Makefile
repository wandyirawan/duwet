.PHONY: dev build release clean

dev:
	cargo run

build:
	cargo build --release
	@echo "Binary: target/release/duwet"

clean:
	cargo clean
