CARGO ?= cargo

all: nyancat

nyancat:
	cd .. && $(CARGO) build --release
	cp ../target/release/nyancat nyancat

clean:
	cd .. && $(CARGO) clean
	-rm -f nyancat

check:
	cd .. && $(CARGO) test

.PHONY: all clean check
