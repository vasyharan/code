all: test

test:
	cargo test --workspace --verbose

coverage:
	@rm -rf ./target/coverage
	CARGO_INCREMENTAL=0 \
		RUSTFLAGS='-Cinstrument-coverage' \
		LLVM_PROFILE_FILE='./target/coverage/raw/cargo-test-%p-%m.profraw' \
		cargo test --workspace --verbose
	grcov \
		--source-dir ./ \
		--binary-path ./target/debug/deps/ \
		--output-types lcov --branch \
		--output-path target/coverage/lcov.info \
		--ignore-not-existing --ignore '../*' --ignore "/*" \
		./target/coverage/raw/