# Necessary dependencies for the build system

# Tool to generate xconfig
ifeq ($(shell xconfig --version 2>/dev/null),)
  $(info Installing xconfig...)
  $(shell cargo install --path xtask/xconfig)
endif

ifeq ($(shell cargo kbuild --version 2>/dev/null),)
  $(info Installing cargo-kbuild...)
  $(shell cargo install --path xtask/cargo-kbuild)
endif

# Tool to parse information about the target package
ifeq ($(shell cargo platconfig --version 2>/dev/null),)
  $(info Installing cargo-platconfig...)
  $(shell cargo install --path xtask/cargo-platconfig)
endif

# Tool to generate platform configuration files
ifeq ($(shell kconfig-gen --version 2>/dev/null),)
  $(info Installing kconfig-gen...)
  $(shell cargo install --path xtask/kconfig-gen)
endif

# Cargo binutils
ifeq ($(shell cargo install --list | grep cargo-binutils),)
  $(info Installing cargo-binutils...)
  $(shell cargo install cargo-binutils)
endif
