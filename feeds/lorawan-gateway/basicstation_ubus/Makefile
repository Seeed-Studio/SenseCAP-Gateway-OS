#
# Copyright (C) 2022 TDT AG <development@tdt.de>
#
# This is free software, licensed under the GNU General Public License v2.
# See /LICENSE for more information.
#
#
include $(TOPDIR)/rules.mk

PKG_NAME:=basicstation_ubus
PKG_VERSION:=2.0.6
PKG_RELEASE:=2

PKG_SOURCE:=$(PKG_NAME)-$(PKG_VERSION).tar.gz
PKG_SOURCE_URL:=https://codeload.github.com/lorabasics/basicstation/tar.gz/v$(PKG_VERSION)?
PKG_HASH:=7e285de94bae1174b4c3496fc3ab15fe67c72f92c0693d2320bafc654a9dfb43
PKG_LICENSE:=BSD-3-Clause
PKG_LICENSE_FILES:=LICENSE

PKG_BUILD_DIR:=$(BUILD_DIR)/$(PKG_NAME)-$(PKG_VERSION)
PKG_BUILD_DEPENDS:=packetforwarder mbedtls

include $(INCLUDE_DIR)/package.mk
include $(INCLUDE_DIR)/cmake.mk

define Package/basicstation_ubus
  SECTION:=gateway
  CATEGORY:=Gateway
  TITLE:=LoRa Basic Station(ubus message support)
  DEPENDS:=+kmod-usb-acm +kmod-spi-dev
endef

define Package/basicstation_ubus/description
  LoRa Basic Station. The LoRaWAN Gateway Software.
endef

define Package/basicstation_ubus/conffiles
/etc/config/basicstation
endef

define Build/Prepare
	mkdir -p $(PKG_BUILD_DIR)
	$(TAR) -C $(PKG_BUILD_DIR) --strip-components=1 -xf $(DL_DIR)/$(PKG_SOURCE)
	[ ! -d ./src/ ] || cp -fpR ./src/. $(PKG_BUILD_DIR)
	$(call Build/Patch/Default)
	rm -rf $(PKG_BUILD_DIR)/deps/
	rm -f $(PKG_BUILD_DIR)/makefile
	rm -f $(PKG_BUILD_DIR)/makefile.s2core
	rm -f $(PKG_BUILD_DIR)/setup.gmk
endef

define Package/basicstation_ubus/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/station $(1)/usr/bin
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/etc/uci-defaults/71_basicstation $(1)/etc/uci-defaults/71_basicstation
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/etc/init.d/basicstation $(1)/etc/init.d/basicstation
	$(INSTALL_DIR) $(1)/etc/basicstation
	$(INSTALL_DATA) ./files/etc/basicstation/* $(1)/etc/basicstation/
	$(INSTALL_DIR) $(1)/etc/basicstation/certs/key
	$(INSTALL_DIR) $(1)/etc/basicstation/certs/client
	$(INSTALL_DIR) $(1)/etc/basicstation/certs/ca
	$(INSTALL_DIR) $(1)/etc/basicstation/token
endef

$(eval $(call BuildPackage,basicstation_ubus))
