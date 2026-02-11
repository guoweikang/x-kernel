# Config generation
ifneq ($(MEM),)
  config_args += -w 'plat.phys-memory-size=$(shell ./scripts/make/strtosz.py $(MEM))'
else
  MEM := $(shell kconfig-gen $(PLAT_CONFIG) -r plat.phys-memory-size 2>/dev/null | tr -d _ | xargs printf "%dB")
endif

SMP := $(shell kconfig-gen $(PLAT_CONFIG) -r plat.cpu-num 2>/dev/null)
ifeq ($(SMP),)
  $(error "`plat.cpu-num` is not defined in the platform configuration file")
endif
