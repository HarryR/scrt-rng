DEVSH=docker run -ti --rm -v /rust/.cargo/registry -v `pwd`:/src rng-dev
CARGO=$(DEVSH) /rust/.cargo/bin/cargo

all:
	@echo computer says no

sh-build: Dockerfile.dev
	docker build -t rng-dev -f Dockerfile.dev .

sh:
	$(DEVSH)

.PHONY: cargo-%
cargo-%:
	$(CARGO) $(shell echo $@ | cut -f 2- -d '-')

test: cargo-unit-test
clippy: cargo-clippy
check: cargo-check
build: cargo-build

release:
	RUSTFLAGS='-C link-arg=-s' $(CARGO) build --target wasm32-unknown-unknown --release 
