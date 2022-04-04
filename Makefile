DEVSH=docker run -ti --rm -v /rust/.cargo/registry -v `pwd`:/src rng-dev
CARGO=$(DEVSH) /rust/.cargo/bin/cargo
SECRETDEV=docker exec -ti secretdev
SECRETCLI=$(SECRETDEV) secretd

OUT_WASM=target/wasm32-unknown-unknown/release/scrt_rng.wasm

all:
	@echo computer says no

sh-build:
	docker build -t rng-dev -f Dockerfile.dev .

sh:
	$(DEVSH)

# ---------------------------------

# Pass single-word commands through to cargo
.PHONY: cargo-%
cargo-%:
	$(CARGO) $(shell echo $@ | cut -f 2- -d '-')

test: cargo-unit-test
clippy: cargo-clippy
check: cargo-check
build: cargo-build
clean: cargo-clean

# ---------------------------------

# Repo contains no generated code
.PHONY: schema
schema:
	$(CARGO) run --example schema

release: schema cargo-wasm
	gzip -fk $(OUT_WASM)
	@du -hs $(OUT_WASM)*

$(OUT_WASM): release

# ---------------------------------

secretdev-start:
	docker run -it --rm -v `pwd`:/src -p 26657:26657 -p 26656:26656 -p 1337:1337 --name secretdev enigmampc/secret-network-sw-dev

secretdev-shell:
	$(SECRETDEV) /bin/bash

contract-deploy: $(OUT_WASM)
	$(SECRETCLI) tx compute store -y --from a --gas 1000000 /src/$(OUT_WASM).gz
