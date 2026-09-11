.PHONY: check test test-full run start

check:
	cargo check

test:
	cargo test --workspace --exclude shell

test-full:
	cargo test --workspace

run:
	cargo run -p app

start:
	watchexec -e rs -r cargo run -p app
