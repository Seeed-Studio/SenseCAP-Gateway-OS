include $(TOPDIR)/rules.mk

PKG_NAME:=ups-module
PKG_VERSION:=1.0.0
PKG_RELEASE:=1

include $(INCLUDE_DIR)/package.mk

define Package/ups-module
	SECTION:=gateway
	CATEGORY:=Gateway
	TITLE:=UPS Power Management Module
	URL:=https://github.com/seeed
	DEPENDS:=+python3 +python3-gpiod
endef

define Package/ups-module/description
	UPS power management module for monitoring power status
	and executing commands during power outages.
endef

define Build/Compile
endef

define Package/ups-module/conffiles
	/etc/config/ups-module
endef

define Package/ups-module/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) ./src/ups-module.py $(1)/usr/bin/ups-module

	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/ups-module.init $(1)/etc/init.d/ups-module

	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/etc/uci-defaults/72_ups-module $(1)/etc/uci-defaults/72_ups-module

	$(INSTALL_DIR) $(1)/etc/ups-uploads
endef

define Package/ups-module/postinst
	#!/bin/sh
	[ -n "$$IPKG_INSTROOT" ] || {
		/etc/init.d/ups-module enable
		/etc/init.d/ups-module start
	}
	exit 0
endef

define Package/ups-module/prerm
	#!/bin/sh
	[ -n "$$IPKG_INSTROOT" ] || {
		/etc/init.d/ups-module stop
		/etc/init.d/ups-module disable
	}
	exit 0
endef

$(eval $(call BuildPackage,ups-module))
