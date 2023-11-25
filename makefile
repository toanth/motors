
.PHONY: all clean

all: motors

motors:
	cargo build --release

debug:
	cargo build

clean:
	rm -rf target/
