include $(TOPDIR)/rules.mk

PKG_NAME:=hardware-info
PKG_VERSION:=1.0
PKG_RELEASE:=1

PKG_BUILD_DIR:=$(BUILD_DIR)/$(PKG_NAME)-$(PKG_VERSION)

include $(INCLUDE_DIR)/package.mk

define Package/hardware-info
  SECTION:=gateway
  CATEGORY:=Gateway
  TITLE:=Hardware Info Reader
endef

define Package/hardware-info/description
  Reads hardware info from EEPROM and populates /etc/deviceinfo
endef

define Build/Prepare
	mkdir -p $(PKG_BUILD_DIR)
	$(CP) ./src/* $(PKG_BUILD_DIR)/
endef

define Package/hardware-info/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/hardware_info $(1)/usr/bin/
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/hardware-info.init $(1)/etc/init.d/hardware-info
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/10_hardware_info $(1)/etc/uci-defaults/10_hardware_info
	$(INSTALL_BIN) ./files/99-fix-distfeeds $(1)/etc/uci-defaults/99-fix-distfeeds
	$(INSTALL_BIN) ./files/etc/uci-defaults/74_hardware $(1)/etc/uci-defaults/74_hardware
endef

define Package/hardware-info/postinst
	#!/bin/sh
	[ -n "$$IPKG_INSTROOT" ] || {
		/etc/init.d/hardware-info enable
		/etc/init.d/hardware-info start
	}
	exit 0
endef

$(eval $(call BuildPackage,hardware-info))
