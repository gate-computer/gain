CARGO		:= cargo +nightly
GATERUN		:= ../gate/bin/gate-run

TARGET		:= wasm32-unknown-unknown

FUNCTION	:= main

-include config.mk

export RUSTFLAGS

debug:
	$(CARGO) build --target=$(TARGET) --examples

release:
	$(CARGO) build --target=$(TARGET) --examples --release

all: debug release

check: debug
	$(GATERUN) -c function=$(FUNCTION) target/$(TARGET)/debug/examples/hello.wasm

check-release: release
	$(GATERUN) -c function=$(FUNCTION) target/$(TARGET)/release/examples/hello.wasm

check-all: check check-release

clean:
	rm -rf target Cargo.lock

.PHONY: debug release all check check-release check-all clean
