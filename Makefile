# SPDX-License-Identifier: LGPL-3.0-or-later
CC ?= cc
CARGO ?= cargo
PYTHON ?= python3
CFLAGS ?= -O2 -Wall -Wextra -Werror
BUILD := build
C_SOURCES := fprint/*.c pam/pam_probe.c guest/legacy_load.c kernel/goodix_bios_read.c crates/backends/native/bridge.c

.PHONY: all rust fprint native-test test fmt fmt-check lint audit check clean kernel
all: rust fprint $(BUILD)/pam_probe $(BUILD)/pam_sddm_probe

$(BUILD):
	mkdir -p $@

rust: | $(BUILD)
	$(CARGO) build --workspace --release --locked
	cp target/release/gxfp51b7 $(BUILD)/gxfp51b7

$(BUILD)/pam_probe: pam/pam_probe.c | $(BUILD)
	$(CC) $(CFLAGS) $< -lpam -o $@

$(BUILD)/pam_sddm_probe: pam/pam_probe.c | $(BUILD)
	$(CC) $(CFLAGS) -DGXFP_PAM_SERVICE='"sddm"' $< -lpam -o $@

fprint: $(BUILD)/libfprint-gxfp51b7.so $(BUILD)/fprint_probe

$(BUILD)/libfprint-gxfp51b7.so: fprint/gxfp51b7.c | $(BUILD)
	$(CC) $(CFLAGS) -fPIC -shared $< $$(pkg-config --cflags --libs libfprint-2-tod-1 libfprint-2 gio-2.0) -o $@

$(BUILD)/fprint_probe: fprint/probe.c | $(BUILD)
	$(CC) $(CFLAGS) $< $$(pkg-config --cflags --libs libfprint-2 gio-2.0) -o $@

native-test:
	$(PYTHON) tools/test_native.py

$(BUILD)/test_worker: fprint/test_worker.c fprint/gxfp51b7.c | $(BUILD)
	$(CC) $(CFLAGS) $< $$(pkg-config --cflags --libs libfprint-2-tod-1 libfprint-2 gio-2.0) -o $@

test: all $(BUILD)/test_worker native-test
	$(BUILD)/test_worker
	$(CARGO) test --workspace --locked
	$(CARGO) build --release --example oracle -p gxfp-core --locked
	PYTHONPATH=tests/reference $(PYTHON) -m unittest discover -s tests -p 'test_*.py' -v
	$(PYTHON) tests/parity.py

fmt:
	ruff format tools tests
	$(CARGO) fmt --all
	clang-format -i $(C_SOURCES)

fmt-check:
	ruff format --check tools tests
	$(CARGO) fmt --all -- --check
	clang-format --dry-run --Werror $(C_SOURCES)

lint:
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings
	$(CARGO) machete
	ruff check tools tests

audit:
	$(CARGO) audit --deny warnings

check: test fmt-check lint
	RUSTDOCFLAGS="-D warnings" $(CARGO) doc --workspace --no-deps --locked
	$(PYTHON) -m compileall -q tools tests
	bash -n guest/bootstrap.sh
	$(PYTHON) tools/check_release.py

kernel:
	$(MAKE) -C /lib/modules/$$(uname -r)/build M=$$(pwd)/kernel modules

clean:
	rm -rf $(BUILD)
	$(CARGO) clean
	find tools tests -type d -name __pycache__ -exec rm -rf {} +
	$(MAKE) -C guest clean
	find kernel -maxdepth 1 -type f \( -name '*.o' -o -name '*.ko' -o -name '*.mod' -o -name '*.mod.c' -o -name '.*.cmd' -o -name Module.symvers -o -name modules.order \) -delete
