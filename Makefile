SHELL = /bin/sh
CARGO ?= cargo
RUSTC ?= rustc
JOBS ?= 4
TEST_THREADS ?= 4
INSTALL_DIR ?= /data/scripts
BUILD_PATH = $(HOME)/.cargo/bin:$(PATH):/opt/homebrew/bin:/usr/local/bin
# The native target, resolved when a recipe needs it.
STATIC_TARGET ?= $$(PATH="$(BUILD_PATH)" $(RUSTC) -vV | sed -n 's/^host: //p')
# GMP's build needs m4. The Command Line Tools 15.3 of macOS have it only as gm4, and their
# /usr/bin/m4 fails and asks to install the tools; xcrun tells which one there is without that.
M4_ENV = if [ -z "$${M4:-}" ] && [ "$$(uname -s)" = Darwin ] && \
	! xcrun --find m4 >/dev/null 2>&1 && xcrun --find gm4 >/dev/null 2>&1; \
	then M4=gm4; export M4; fi;

all: test build

build: release

release:
	@$(M4_ENV) PATH="$(BUILD_PATH)" $(CARGO) build --locked --release -j $(JOBS)

debug:
	@$(M4_ENV) PATH="$(BUILD_PATH)" $(CARGO) build --locked -j $(JOBS)

# macOS has no static executables: there the release is built for the native target and
# must need nothing but the libraries of the system.
static:
	@$(M4_ENV) target="$(STATIC_TARGET)"; \
		flags="$(RUSTFLAGS) -C target-feature=+crt-static"; \
		case "$$target" in \
			*-freebsd) flags="$$flags -C link-arg=-Wno-unused-command-line-argument" ;; \
			*-apple-darwin) flags="$(RUSTFLAGS)" ;; \
		esac; \
		PATH="$(BUILD_PATH)" RUSTFLAGS="$$flags" $(CARGO) build --locked --release -j $(JOBS) --target "$$target"
	@target="$(STATIC_TARGET)"; binary="target/$$target/release/symbolic-math"; \
		case "$$target" in \
			*-apple-darwin) \
				otool -L "$$binary" | awk 'NR > 1 && $$1 !~ /^\/(usr\/lib|System\/Library)\// { bad = 1 } END { exit bad || NR < 2 }' && exit 0; \
				kind="it needs more than the libraries of macOS: $$(otool -L "$$binary" 2>&1 | tr -s '\n\t' '  ')" ;; \
			*) kind="$$(LC_ALL=C file -b "$$binary")" ;; \
		esac; \
		case "$$kind" in \
			*"statically linked"*|*"static-pie linked"*) ;; \
			*) printf '%s\n' "error: not a static executable: $$kind" >&2; exit 1 ;; \
		esac

install: release static
	install -m 755 target/release/symbolic-math symbolic-math
	install -m 755 "target/$(STATIC_TARGET)/release/symbolic-math" symbolic-math.static
	mkdir -p "$(INSTALL_DIR)"
	install -m 755 symbolic-math.static "$(INSTALL_DIR)/symbolic-math"

test:
	@$(M4_ENV) PATH="$(BUILD_PATH)" RUST_TEST_THREADS=$(TEST_THREADS) $(CARGO) test --locked -j $(JOBS)

fmt:
	PATH="$(BUILD_PATH)" $(CARGO) fmt --all

lint:
	PATH="$(BUILD_PATH)" $(CARGO) fmt --all -- --check
	@$(M4_ENV) PATH="$(BUILD_PATH)" $(CARGO) clippy --locked --all-targets -j $(JOBS) -- -D warnings

clean:
	PATH="$(BUILD_PATH)" $(CARGO) clean

.PHONY: all build release debug static install test fmt lint clean
