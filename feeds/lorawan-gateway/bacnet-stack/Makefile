include $(TOPDIR)/rules.mk

PKG_NAME:=bacnet-stack
PKG_VERSION:=1.5.0
PKG_RELEASE:=1

PKG_SOURCE:=$(PKG_VERSION).tar.gz
PKG_SOURCE_URL:=https://github.com/bacnet-stack/bacnet-stack/archive/refs/tags/
PKG_HASH:=skip

PKG_BUILD_DIR:=$(BUILD_DIR)/bacnet-stack-$(PKG_VERSION)
PKG_BUILD_PARALLEL:=1

CMAKE_INSTALL:=1

include $(INCLUDE_DIR)/package.mk
include $(INCLUDE_DIR)/cmake.mk

CMAKE_OPTIONS += \
	-DBACNET_STACK_BUILD_APPS=ON \
	-DBAC_ROUTING=OFF \
	-DBACNET_BUILD_SERVER_BASIC_APP=ON \
	-DBACNET_BUILD_SERVER_MINI_APP=OFF \
	-DBACNET_PROPERTY_LISTS=ON

define Package/bacnet-stack
	SECTION:=gateway
	CATEGORY:=Gateway
	TITLE:=BACnet Protocol Stack
	URL:=https://github.com/bacnet-stack/bacnet-stack
	DEPENDS:=+libpthread
endef

define Package/bacnet-stack/description
	BACnet/IP protocol stack and utilities.
	Provides bacserv (BACnet server daemon) and CLI tools:
	  - bacserv : BACnet device server (server-basic)
	  - bacwhois : discover BACnet devices on the network
	  - bacepics : read all objects/properties from a device
	  - bacrp / bacwp : read / write a single property
endef

define Build/Prepare
	$(call Build/Prepare/Default)
endef

define Package/bacnet-stack/install
	$(INSTALL_DIR) $(1)/usr/bin

	# server daemon
	$(INSTALL_BIN) $(PKG_INSTALL_DIR)/usr/bin/bacserv       $(1)/usr/bin/ 2>/dev/null || \
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/build/apps/server-basic/bacserv $(1)/usr/bin/ 2>/dev/null || \
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/bin/bacserv              $(1)/usr/bin/ 2>/dev/null || true

	# CLI tools (install any that were built)
	for t in bacwhois bacepics bacrp bacwp; do \
		if [ -f $(PKG_INSTALL_DIR)/usr/bin/$$$$t ]; then \
			$(INSTALL_BIN) $(PKG_INSTALL_DIR)/usr/bin/$$$$t $(1)/usr/bin/; \
		fi; \
	done
endef

$(eval $(call BuildPackage,bacnet-stack))
