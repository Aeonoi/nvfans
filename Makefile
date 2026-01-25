DESTDIR:=/
PREFIX:=/usr/local
SYSTEMD_DIR:=/usr/local/lib/systemd/system/
NVFANS_SERVICE:=nvfans.service
NVFANS_SLEEP_SERVICE:=nvfans-sleep.service
NVFANS_RESUME_SERVICE:=nvfans-resume.service

.PHONY: build-release
build-release:
	cargo build -p nvfans --release
	
install: 
	install -Dm755 target/release/nvfans $(DESTDIR)$(PREFIX)/bin/nvfans
	# Services
	install -Dm644 services/nvfans.service 	      $(SYSTEMD_DIR)
	install -Dm644 services/nvfans-sleep.service  $(SYSTEMD_DIR)
	install -Dm644 services/nvfans-resume.service $(SYSTEMD_DIR)

.PHONY: uninstall
uninstall:
	rm $(DESTDIR)$(PREFIX)/bin/nvfans
	rm $(SYSTEMD_DIR)$(NVFANS_SERVICE)
	rm $(SYSTEMD_DIR)$(NVFANS_SLEEP_SERVICE)
	rm $(SYSTEMD_DIR)$(NVFANS_RESUME_SERVICE)

