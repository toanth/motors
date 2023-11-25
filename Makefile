
# A makefile is necessary for OpenBench, but this project uses cargo.
# Therefore, all that this makefile does is to execute `cargo build --release`

.PHONY: all clean

all: motors

motors:
	cargo build --release

debug:
	cargo build

clean:
	rm -rf target/
