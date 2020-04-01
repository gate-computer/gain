CARGO		:= cargo
GATE		:= gate

TARGET		:= wasm32-wasi

FUNCTION	:=

-include config.mk

debug:
	$(CARGO) build --target=$(TARGET) --examples

release:
	$(CARGO) build --target=$(TARGET) --examples --release

all: debug release

check: debug
	$(GATE) call -d target/$(TARGET)/debug/examples/hello.wasm $(FUNCTION)
	/bin/echo -e "+ 1 2\n+ 2 3\ncatalog" | $(GATE) call -d target/$(TARGET)/debug/examples/lep.wasm

check-release: release
	$(GATE) call target/$(TARGET)/release/examples/hello.wasm $(FUNCTION)
	/bin/echo -e "+ 1 2\n+ 2 3\ncatalog" | $(GATE) call target/$(TARGET)/release/examples/lep.wasm

check-all: check check-release

clean:
	rm -rf target Cargo.lock

.PHONY: debug release all check check-release check-all clean
