# Top level makefile, the real shit is at src/Makefile

default: all

.DEFAULT:
	cd deps/redis-2.8.19 && $(MAKE)
	cd src && $(MAKE) $@
