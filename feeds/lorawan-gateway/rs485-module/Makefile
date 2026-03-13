include $(TOPDIR)/rules.mk

PKG_NAME:=rs485-module
PKG_VERSION:=1.0.0
PKG_RELEASE:=1

PKG_BUILD_DEPENDS:=rust/host

include $(INCLUDE_DIR)/package.mk
include $(TOPDIR)/feeds/packages/lang/rust/rust-package.mk

define Package/rs485-module
	SECTION:=gateway
	CATEGORY:=Gateway
	TITLE:=RS485 to MQTT Bridge (Rust)
	URL:=https://github.com/seeed
	DEPENDS:=+libc +libstdcpp +libuci +libopenssl
endef

define Package/rs485-module/description
	Bidirectional bridge between RS485 serial port and MQTT broker.
	Written in Rust with rumqttc for async MQTT communication.
	Uplink: RS485 data wrapped in JSON {"data":"..."}.
	Downlink: MQTT JSON {"data":"..."} converted to raw bytes.
	Reads configuration from UCI (/etc/config/rs485-module).
endef

define Build/Prepare
	$(call Build/Prepare/Default)
	$(CP) ./Cargo.toml $(PKG_BUILD_DIR)/
	mkdir -p $(PKG_BUILD_DIR)/src
	$(CP) ./src/main.rs $(PKG_BUILD_DIR)/src/
endef

define Package/rs485-module/conffiles
	/etc/config/rs485-module
endef

define Package/rs485-module/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_INSTALL_DIR)/bin/rs485-module $(1)/usr/bin/

	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/rs485-module.init $(1)/etc/init.d/rs485-module

	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/etc/uci-defaults/73_rs485-module $(1)/etc/uci-defaults/73_rs485-module
endef

define Package/rs485-module/postinst
	#!/bin/sh
	[ -n "$$IPKG_INSTROOT" ] || {
	/etc/init.d/rs485-module enable
	/etc/init.d/rs485-module start
	}
	exit 0
endef

define Package/rs485-module/prerm
	#!/bin/sh
	[ -n "$$IPKG_INSTROOT" ] || {
	/etc/init.d/rs485-module stop
	/etc/init.d/rs485-module disable
	}
	exit 0
endef

$(eval $(call RustBinPackage,rs485-module))
$(eval $(call BuildPackage,rs485-module))
