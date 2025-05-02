
# A makefile is necessary for OpenBench, but this project uses cargo.
# Therefore, all that this makefile does is to execute `cargo build --release`

EXE = caps
CC = cargo
export EXE
export CC
TARGET_TUPLE := $(shell rustc --print host-tuple)

.PHONY: caps release debug all clean

#the first target is the default target and only builds `caps`, because that's all that is necessary for openbench
default: caps-pgo

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

caps-pgo:
	cargo pgo build
	echo "position startpos moves e2e4 g8f6\ngo wtime 500 btime 500 winc 5 binc 5\nwait\nquit" | cargo pgo run
	cargo pgo run -- bench
	cargo pgo optimize
	mv "target/$(TARGET_TUPLE)/release/motors" "$(EXE)"

release:
	cargo build --release --package motors --bin motors

debug:
	cargo build --package motors --bin motors

clean:
	rm -rf target/
