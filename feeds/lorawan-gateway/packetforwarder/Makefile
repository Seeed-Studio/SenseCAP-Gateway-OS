#
# Copyright (C) 2022 TDT AG <development@tdt.de>
#
# This is free software, licensed under the GNU General Public License v2.
# See /LICENSE for more information.
#
#
include $(TOPDIR)/rules.mk

PKG_NAME:=packetforwarder
PKG_VERSION:=2.1.0
PKG_RELEASE:=1

PKG_SOURCE:=$(PKG_NAME)-$(PKG_VERSION).tar.gz
PKG_SOURCE_URL:=https://codeload.github.com/Lora-net/sx1302_hal/tar.gz/V$(PKG_VERSION)?
PKG_HASH:=4b62aa6a83ad449c68fdd844fc35024586c9faabca3c5a90a2544735b380de5b
PKG_LICENSE_FILES:=LICENSE.TXT
PKG_BUILD_DEPENDS:=sx1302_hal

include $(INCLUDE_DIR)/package.mk

define Package/packetforwarder/Default
  SECTION:=gateway
  CATEGORY:=Gateway
  TITLE:=SX1302/SX1303 packet forwarder(add ubus message support)
endef

define Package/packetforwarder
  $(call Package/packetforwarder/Default)
  TITLE+=(Packet Forwarder)
  DEPENDS:=+kmod-usb-acm +kmod-spi-dev +librt +libpthread
endef

define Package/packetforwarder/description
  Packet Forwarders for SX1302/SX1303 Hardware Abstraction Layer
endef

define Package/packetforwarder/config
	select BUSYBOX_DEFAULT_USLEEP
endef

TARGET_CFLAGS = -O2 -Wall -Wextra -std=c99 -Iinc -I. -lm  -I../libtools/inc

define Package/packetforwarder/conffiles
/etc/config/packetforwarder
endef

define Build/Prepare
	mkdir -p $(PKG_BUILD_DIR)
	$(TAR) -C $(PKG_BUILD_DIR) --strip-components=1 -xf $(DL_DIR)/$(PKG_SOURCE)
	[ ! -d ./src/ ] || cp -fpR ./src/. $(PKG_BUILD_DIR)
	$(call Build/Patch/Default)
endef

# replace /libloragw/inc/*.h and /libloragw/*.a with the ones from sx1302_hal
define Build/InstallDev
	$(INSTALL_DIR) $(1)/usr/include/lgw
	$(CP) $(PKG_BUILD_DIR)/libloragw/inc/*.h $(1)/usr/include/lgw

	$(INSTALL_DIR) $(1)/usr/lib
	$(CP) $(PKG_BUILD_DIR)/libloragw/*.a $(1)/usr/lib
endef

define Package/packetforwarder/install
	$(INSTALL_DIR) $(1)/usr/bin/packetforwarder
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/packet_forwarder/lora_pkt_fwd $(1)/usr/bin/packetforwarder/

	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/etc/uci-defaults/70_packetforwarder $(1)/etc/uci-defaults/70_packetforwarder

	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/etc/init.d/lora_pkt_fwd.init $(1)/etc/init.d/lora_pkt_fwd

	$(INSTALL_DIR) $(1)/etc/packetforwarder
	$(INSTALL_DATA) ./files/etc/packetforwarder/*.* $(1)/etc/packetforwarder/
endef

$(eval $(call BuildPackage,packetforwarder))