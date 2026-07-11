.PHONY: check rust swift
rust:
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
swift:
	cd Packages/App && swift build
check: rust swift
