-include Makefile.options
RUST_LOG?=DEBUG
#####################################################################################
## print usage information
help:
	@echo 'Usage:'
	@cat ${MAKEFILE_LIST} | grep -e "^## " -A 1 | grep -v '\-\-' | sed 's/^##//' | cut -f1 -d":" | \
		awk '{info=$$0; getline; print "  " $$0 ": " info;}' | column -t -s ':' | sort 
.PHONY: help
#####################################################################################
## run the server
run:
	cargo run --bin cache-rs 
.PHONY: run
###############################################################################
run/build: build/local
	target/release/cache-rs --
.PHONY: run/build
###############################################################################
## build the server
build/local: 
	cargo build --release  -vv
.PHONY: build/local
###############################################################################
## run unit tests
test/unit:
	RUST_LOG=DEBUG cargo test --no-fail-fast
.PHONY: test/unit
###############################################################################
## run lint
test/lint:
	@cargo clippy -V
	cargo clippy --all-targets --all-features -- -D warnings
.PHONY: test/lint	
###############################################################################
## clean all
clean:
	cargo clean
.PHONY: clean

.EXPORT_ALL_VARIABLES:
