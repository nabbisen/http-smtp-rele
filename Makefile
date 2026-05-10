# Makefile — http-smtp-rele
#
# Wraps cargo invocations with the correct rustc/rustdoc binaries.
#
# Usage:
#   make build         build debug
#   make release       build release binary
#   make test          run all tests
#   make check         cargo check (fast type-check)
#   make check-rfcs    verify RFC directory integrity
#   make clean         remove build artifacts
#   make gate          full pre-commit verification

RUSTC   := /usr/bin/rustc-1.91
RUSTDOC := /usr/bin/rustdoc-1.91
CARGO   := RUSTC=$(RUSTC) RUSTDOC=$(RUSTDOC) /usr/bin/cargo-1.91

.PHONY: build release test check check-rfcs clean gate

build:
	$(CARGO) build

release:
	$(CARGO) build --release

test:
	$(CARGO) test

check:
	$(CARGO) check

check-rfcs:
	@sh scripts/check-rfcs.sh

clean:
	$(CARGO) clean

# Full pre-commit gate (RFC 004)
gate: check test check-rfcs
	@echo "All gates passed."
