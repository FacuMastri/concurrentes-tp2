TARGETS = coffee_maker server common

all: fmt test clippy;

%:
	cargo $@