CARGO		:= cargo
FLATC		:= flatc
GATE		:= gate

TARGET		:= wasm32-wasi

FUNCTION	:=

-include config.mk

.PHONY: debug release
debug release:
	$(CARGO) build --target=$(TARGET) $(patsubst --debug,,--$@)
	$(CARGO) build --target=$(TARGET) --examples $(patsubst --debug,,--$@)

.PHONY: all
all: debug release

.PHONY: check
check: check-debug check-release
check-%: %
	$(GATE) call -d target/$(TARGET)/$*/examples/hello.wasm $(FUNCTION)
	$(GATE) call -d target/$(TARGET)/$*/examples/catalog.wasm
	/bin/echo -e "+ 1 2\n+ 2 3\ncatalog\nidentity/principal\nidentity/instance" | $(GATE) call -d target/$(TARGET)/$*/examples/lep.wasm
	set -e; $(GATE) call -d target/$(TARGET)/$*/examples/peer.wasm & \
		$(GATE) call -d target/$(TARGET)/$*/examples/peer.wasm & \
		wait

.PHONY: generate
generate:
	$(FLATC) --rust -o gain-localhost/src ../gate-localhost/localhost.fbs

.PHONY: clean
clean:
	rm -rf Cargo.lock target
