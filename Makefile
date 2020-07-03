CARGO		:= cargo
GATE		:= gate

TARGET		:= wasm32-wasi

FUNCTION	:=

-include config.mk

.PHONY: debug
debug:
	$(CARGO) build --target=$(TARGET) --examples

.PHONY: release
release:
	$(CARGO) build --target=$(TARGET) --examples --release

.PHONY: all
all: debug release

.PHONY: check
check: check-debug check-release
check-%: %
	$(GATE) call -d target/$(TARGET)/$*/examples/hello.wasm $(FUNCTION)
	/bin/echo -e "+ 1 2\n+ 2 3\ncatalog\nidentity/principal\nidentity/instance" | $(GATE) call -d target/$(TARGET)/$*/examples/lep.wasm
	set -e; $(GATE) call -d target/$(TARGET)/$*/examples/peer.wasm & \
		$(GATE) call -d target/$(TARGET)/$*/examples/peer.wasm & \
		wait

.PHONY: clean
clean:
	rm -rf target Cargo.lock
