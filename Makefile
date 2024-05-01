
# A makefile is necessary for OpenBench, but this project uses cargo.
# Therefore, all that this makefile does is to execute `cargo build --release`

EXE = caps
CC = cargo
export EXE
export CC

.PHONY: all clean

#the first target is the default target and only builds `motors`, because that's all that is necessary for openbench
default: motors

all: motors monitors

monitors:
	cargo build --release --bin monitors

motors: release

bench: release
	./motors bench

release:
	cd motors && make release

debug:
	cd motors && make debug

clean:
	rm -rf target/
