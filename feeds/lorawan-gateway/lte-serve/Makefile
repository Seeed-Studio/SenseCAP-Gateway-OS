include $(TOPDIR)/rules.mk

PKG_NAME:=lte-serve
PKG_VERSION:=1.0
PKG_RELEASE:=1

include $(INCLUDE_DIR)/package.mk

define Package/lte-serve
  SECTION:=gateway
  CATEGORY:=Gateway
  TITLE:=LTE Module Initialization Service
  DEPENDS:=+gpiod-tools +kmod-usb-serial +kmod-usb-serial-option +coreutils-timeout
  PKGARCH:=all
endef

define Package/lte-serve/description
  LTE module initialization service with GPIO control and AT command support
endef

define Build/Compile
endef

define Package/lte-serve/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) ./files/lte-serve.sh $(1)/usr/bin/
	$(INSTALL_BIN) ./files/uqmi $(1)/usr/bin/

	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/lte-serve.init $(1)/etc/init.d/lte-serve
endef

$(eval $(call BuildPackage,lte-serve))
