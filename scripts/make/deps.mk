# Necessary dependencies for the build system

# Tool to generate xconfig
#ifeq ($(shell xconfig --version 2>/dev/null),)
#  $(info Installing xconfig...)
#  $(shell cargo install --path xtask/xconfig)
#endif


# xconf tool (the correct binary name is xconf, not xconfig)
XCONF_BIN := $(shell command -v xconf 2>/dev/null)
XCONF_SRC := $(shell find xtask/xconfig/src -type f -name '*.rs' 2>/dev/null)
XCONF_CARGO := xtask/xconfig/Cargo.toml

# Only rebuild xconf if:
# 1. It doesn't exist, OR
# 2. Source files are newer than the binary
.PHONY: check-xconf
check-xconf:
	@if [ -z "$(XCONF_BIN)" ]; then \
		echo "🔧 Installing xconf..."; \
		cargo install --path xtask/xconfig --force; \
	elif [ "$(XCONF_SRC)" -nt "$(XCONF_BIN)" ] || [ "$(XCONF_CARGO)" -nt "$(XCONF_BIN)" ]; then \
		echo "🔄 Updating xconf (source changed)..."; \
		cargo install --path xtask/xconfig --force; \
	fi

# Cargo binutils
ifeq ($(shell cargo install --list | grep cargo-binutils),)
  $(info Installing cargo-binutils...)
  $(shell cargo install cargo-binutils)
endif
