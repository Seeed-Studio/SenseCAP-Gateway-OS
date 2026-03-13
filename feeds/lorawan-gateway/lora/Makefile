include $(TOPDIR)/rules.mk

PKG_NAME:=lora
PKG_VERSION:=1.0
PKG_RELEASE:=1

PKG_BUILD_DEPENDS:=rust/host

PKG_MAINTAINER:=ruiqian tang <ruiqian.tang@seeed.cc>
PKG_LICENSE:=Apache-2.0

include $(INCLUDE_DIR)/package.mk
include $(TOPDIR)/feeds/packages/lang/rust/rust-package.mk

define Package/lora
	SECTION:=gateway
	CATEGORY:=Gateway
	TITLE:=LoRa Configuration
	DEPENDS:=+packetforwarder +basicstation_ubus +libc +libstdcpp
endef

define Package/lora/description
	Provides configuration files for LoRa Configuration.
endef

define Package/lora/conffiles
	/etc/config/lora
endef

define Package/lora/install
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/etc/uci-defaults/64_lora_config $(1)/etc/uci-defaults/64_lora_config

	$(INSTALL_DIR) $(1)/etc/lora
	$(INSTALL_DATA) ./files/etc/lora/freq_plan.json $(1)/etc/lora/freq_plan.json

	$(INSTALL_DIR) $(1)/usr/bin/
	$(INSTALL_BIN) $(PKG_INSTALL_DIR)/bin/radio_init $(1)/usr/bin/radio_init
endef

RUST_PKG_LOCKED:=0

define Build/Prepare
	rm -rf $(PKG_BUILD_DIR)
	mkdir -p $(PKG_BUILD_DIR)
	$(CP) ./Cargo.toml $(PKG_BUILD_DIR)/
	$(CP) ./src $(PKG_BUILD_DIR)/
endef

$(eval $(call BuildPackage,lora))