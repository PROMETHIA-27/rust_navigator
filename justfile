build:
	cargo build
	rm -r editors/code/server/
	mkdir editors/code/server/
	mv target/debug/rust_navigator.exe editors/code/server/
