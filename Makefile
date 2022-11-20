TARGETS = .

all: fmt test clippy;

%:
	cargo $@