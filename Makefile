
# A makefile is necessary for OpenBench, but this project uses cargo.
# Therefore, all that this makefile does is to execute `cargo build --release`

EXE = caps
CC = cargo
export EXE
export CC

.PHONY: caps release debug all clean

#the first target is the default target and only builds `caps`, because that's all that is necessary for openbench
default: caps

all: motors monitors pliers

monitors:
	cargo build --release --package monitors

pliers:
	cargo build --release --package pliers

motors: release

caps:
	cargo rustc --release --package motors --bin motors --no-default-features --features=caps -- --emit link=${EXE}

bench: release
	./caps bench

release:
	cargo build --release --package motors --bin motors

debug:
	cargo build --package motors --bin motors

clean:
	rm -rf target/
