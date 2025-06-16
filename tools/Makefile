# VexFS Tools Makefile

CC = gcc
CFLAGS = -Wall -Wextra -std=c99 -O2
LDFLAGS = 

# Targets
TARGETS = mkfs.vexfs

# Default target
all: $(TARGETS)

# Build mkfs.vexfs
mkfs.vexfs: mkfs.vexfs.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

# Clean build artifacts
clean:
	rm -f $(TARGETS) *.o

# Install tools (optional)
install: $(TARGETS)
	install -d $(DESTDIR)/sbin
	install -m 755 mkfs.vexfs $(DESTDIR)/sbin/

# Uninstall tools
uninstall:
	rm -f $(DESTDIR)/sbin/mkfs.vexfs

.PHONY: all clean install uninstall