
# A makefile is necessary for OpenBench, but this project uses cargo.
# Therefore, all that this makefile does is to execute `cargo build --release`

EXE = motors
CC = cargo

.PHONY: all clean

all: motors

motors: release

bench: release
	./motors bench

release:
	cargo rustc --release --bin motors -- --emit link=${EXE}

debug:
	rustc -- --emit link=${EXE}

clean:
	rm -rf target/
