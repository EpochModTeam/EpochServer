# Top level makefile, the real shit is at src/Makefile

default: all

.DEFAULT:
	#cd deps/redis_nix && $(MAKE)
	cd src && $(MAKE) $@
