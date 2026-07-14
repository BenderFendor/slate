PREFIX ?= /usr/local
DESTDIR ?=
BINDIR := $(DESTDIR)$(PREFIX)/bin
DATADIR := $(DESTDIR)$(PREFIX)/share

.PHONY: build release test check install uninstall package clean

build:
	cargo build --locked

release:
	cargo build --release --locked

test:
	cargo test --all-targets --all-features --locked

check:
	cargo fmt --check
	cargo test --all-targets --all-features --locked
	cargo clippy --all-targets --all-features --locked -- -D warnings

install: release
	install -Dm755 target/release/slate $(BINDIR)/slate
	install -Dm644 data/com.slate.editor.desktop $(DATADIR)/applications/com.slate.editor.desktop
	install -Dm644 data/com.slate.editor.metainfo.xml $(DATADIR)/metainfo/com.slate.editor.metainfo.xml
	install -Dm644 data/com.slate.editor.xml $(DATADIR)/mime/packages/com.slate.editor.xml
	install -Dm644 data/icons/hicolor/scalable/apps/com.slate.editor.svg $(DATADIR)/icons/hicolor/scalable/apps/com.slate.editor.svg
	install -d $(DATADIR)/icons/hicolor/scalable/actions
	cp -a assets/icons/hicolor/scalable/actions/. $(DATADIR)/icons/hicolor/scalable/actions/

uninstall:
	rm -f $(BINDIR)/slate
	rm -f $(DATADIR)/applications/com.slate.editor.desktop
	rm -f $(DATADIR)/metainfo/com.slate.editor.metainfo.xml
	rm -f $(DATADIR)/mime/packages/com.slate.editor.xml
	rm -f $(DATADIR)/icons/hicolor/scalable/apps/com.slate.editor.svg
	find assets/icons/hicolor/scalable/actions -maxdepth 1 -type f -printf '%f\n' | while read icon; do rm -f "$(DATADIR)/icons/hicolor/scalable/actions/$$icon"; done

package:
	./scripts/package-linux.sh

clean:
	cargo clean
	rm -rf dist
