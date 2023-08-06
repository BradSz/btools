.PHONY: all
all: test

.PHONY: fix
fix:
	@cargo fix
	@cargo clippy --fix --allow-staged
	@cargo fmt

.PHONY: install
install:
	@cargo install --path ./binspect
	@cargo install --path ./chop

.PHONY: test
test:
	@cargo test
	@cargo clippy
