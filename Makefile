# SPDX-License-Identifier: LGPL-3.0-or-later
CC ?= cc
PYTHON ?= python3
CFLAGS ?= -O2 -Wall -Wextra -Werror
BUILD := build

.PHONY: all test check clean kernel
all: $(BUILD)/pam_gxfp51b7.so $(BUILD)/pam_probe $(BUILD)/pam_sddm_probe

$(BUILD):
	mkdir -p $@

$(BUILD)/pam_gxfp51b7.so: pam/pam_gxfp51b7.c | $(BUILD)
	$(CC) $(CFLAGS) -fPIC -shared $< -lpam -o $@

$(BUILD)/pam_probe: pam/pam_probe.c | $(BUILD)
	$(CC) $(CFLAGS) $< -lpam -o $@

$(BUILD)/pam_sddm_probe: pam/pam_probe.c | $(BUILD)
	$(CC) $(CFLAGS) -DGXFP_PAM_SERVICE='"sddm"' $< -lpam -o $@

test:
	PYTHONPATH=src $(PYTHON) -m unittest discover -s tests -v

check: all test
	$(PYTHON) -m compileall -q src tools tests
	bash -n guest/bootstrap.sh
	$(PYTHON) tools/check_release.py

kernel:
	$(MAKE) -C /lib/modules/$$(uname -r)/build M=$$(pwd)/kernel modules

clean:
	rm -rf $(BUILD)
	find src tools tests -type d -name __pycache__ -exec rm -rf {} +
	$(MAKE) -C guest clean
	find kernel -maxdepth 1 -type f \( -name '*.o' -o -name '*.ko' -o -name '*.mod' -o -name '*.mod.c' -o -name '.*.cmd' -o -name Module.symvers -o -name modules.order \) -delete
