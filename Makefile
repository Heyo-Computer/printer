# Build and install the printer + computer + codegraph CLIs.
#
#   make                  # build all (release)
#   make install          # install all to $(PREFIX)/bin (default: ~/.local)
#   make install-printer  # build + install just one
#   make uninstall        # remove installed binaries
#   make clean            # cargo clean
#
# Override the install prefix:
#   make install PREFIX=/usr/local        # may need sudo
#   make install PREFIX=/opt/printer

PREFIX ?= $(HOME)/.local
BINDIR := $(PREFIX)/bin

CARGO ?= cargo
CARGO_FLAGS ?= --release

CRATES := printer computer codegraph

.PHONY: all build install uninstall clean check test help \
	$(addprefix build-,$(CRATES)) \
	$(addprefix install-,$(CRATES))

all: build

help:
	@echo "Targets:"
	@echo "  build               build all crates ($(CARGO_FLAGS))"
	@echo "  build-printer       build just printer"
	@echo "  build-computer      build just computer"
	@echo "  build-codegraph     build just codegraph"
	@echo "  install             install all binaries to \$$(BINDIR)"
	@echo "  install-printer     install just printer"
	@echo "  install-computer    install just computer"
	@echo "  install-codegraph   install just codegraph"
	@echo "  uninstall           remove all binaries from \$$(BINDIR)"
	@echo "  check               cargo check all crates"
	@echo "  test                cargo test all crates"
	@echo "  clean               cargo clean all crates"
	@echo ""
	@echo "Variables:"
	@echo "  PREFIX=$(PREFIX)"
	@echo "  BINDIR=$(BINDIR)"
	@echo "  CARGO=$(CARGO)"
	@echo "  CARGO_FLAGS=$(CARGO_FLAGS)"

build: $(addprefix build-,$(CRATES))

# Static pattern rules — implicit (`%:`) rules don't fire for .PHONY targets.
$(addprefix build-,$(CRATES)): build-%:
	$(CARGO) build --manifest-path $*/Cargo.toml $(CARGO_FLAGS)

install: $(addprefix install-,$(CRATES))

$(addprefix install-,$(CRATES)): install-%: build-%
	@mkdir -p $(BINDIR)
	install -m 0755 $*/target/release/$* $(BINDIR)/$*
	@echo "installed $* -> $(BINDIR)/$*"

uninstall:
	@for c in $(CRATES); do \
		if [ -f "$(BINDIR)/$$c" ]; then \
			rm -f "$(BINDIR)/$$c" && echo "removed $(BINDIR)/$$c"; \
		else \
			echo "$(BINDIR)/$$c not installed"; \
		fi; \
	done

check:
	@for c in $(CRATES); do \
		$(CARGO) check --manifest-path $$c/Cargo.toml; \
	done

test:
	@for c in $(CRATES); do \
		$(CARGO) test --manifest-path $$c/Cargo.toml; \
	done

clean:
	@for c in $(CRATES); do \
		$(CARGO) clean --manifest-path $$c/Cargo.toml; \
	done
