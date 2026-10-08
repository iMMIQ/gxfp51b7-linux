# SPDX-License-Identifier: LGPL-3.0-or-later
# Build only in the isolated guest. SGX_DRIVER is your upstream driver checkout.
CC ?= cc
SGX_DRIVER ?= /home/ubuntu/legacy-driver
CFLAGS ?= -O2 -Wall -Wextra

legacy_load: legacy_load.c enter.S
	$(CC) $(CFLAGS) -I$(SGX_DRIVER) legacy_load.c enter.S -o $@

.PHONY: clean
clean:
	rm -f legacy_load
