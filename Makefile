 
.PHONY: release, test, dev

release:
	cargo update
	cargo test
	cargo build --release
	strip target/release/cash_microservice

build:
	cargo update
	cargo test
	cargo build

dev:
	# . ./ENV.sh; backper
	cargo run;

test:
	cargo test