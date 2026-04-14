include $(TOPDIR)/rules.mk

PKG_NAME:=bacnet-stack
PKG_VERSION:=1.4.2
PKG_RELEASE:=1

PKG_SOURCE:=bacnet-stack-$(PKG_VERSION).tar.gz
PKG_SOURCE_URL:=https://github.com/bacnet-stack/bacnet-stack/archive/refs/tags/
PKG_HASH:=skip

PKG_BUILD_DIR:=$(BUILD_DIR)/bacnet-stack-bacnet-stack-$(PKG_VERSION)
PKG_BUILD_PARALLEL:=1

CMAKE_INSTALL:=1

include $(INCLUDE_DIR)/package.mk
include $(INCLUDE_DIR)/cmake.mk

CMAKE_OPTIONS += \
	-DBACNET_STACK_BUILD_APPS=ON \
	-DBAC_ROUTING=OFF \
	-DBACNET_BUILD_SERVER_BASIC_APP=ON \
	-DBACNET_BUILD_SERVER_MINI_APP=OFF \
	-DBACNET_PROPERTY_LISTS=ON \
	-DBACDL_MSTP=ON \
	-DBACDL_BIP=OFF \
	-DBACDL_BIP6=OFF

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

# OpenWrt cmake.mk builds in-tree: binaries land in $(PKG_BUILD_DIR) directly
CMAKE_BUILD_DIR ?= $(PKG_BUILD_DIR)

define Package/bacnet-stack/install
	$(INSTALL_DIR) $(1)/usr/bin

	# server daemon: cmake names the binary 'bacbasic', install as bacserv
	$(INSTALL_BIN) $(CMAKE_BUILD_DIR)/bacbasic $(1)/usr/bin/bacserv

	# CLI tools: cmake name -> installed name
	-$(INSTALL_BIN) $(CMAKE_BUILD_DIR)/whois     $(1)/usr/bin/bacwhois
	-$(INSTALL_BIN) $(CMAKE_BUILD_DIR)/epics     $(1)/usr/bin/bacepics
	-$(INSTALL_BIN) $(CMAKE_BUILD_DIR)/readprop  $(1)/usr/bin/bacrp
	-$(INSTALL_BIN) $(CMAKE_BUILD_DIR)/writeprop $(1)/usr/bin/bacwp
endef

$(eval $(call BuildPackage,bacnet-stack))
