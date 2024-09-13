# Makefile

miri_tests:
	set MIRIFLAGS=-Zmiri-tree-borrows && cargo +nightly miri test $(filter-out $@,$(MAKECMDGOALS))

# Prevent make from treating the argument as a target
%:
	@:
