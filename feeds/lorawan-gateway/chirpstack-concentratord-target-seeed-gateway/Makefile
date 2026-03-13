include $(TOPDIR)/rules.mk

PKG_NAME:=chirpstack-concentratord-target-seeed-gateway
PKG_VERSION:=1.0
PKG_RELEASE:=1

include $(INCLUDE_DIR)/package.mk

define Package/chirpstack-concentratord-target-seeed-gateway
	SECTION:=chirpstack
	CATEGORY:=ChirpStack
	TITLE:=ChirpStack Concentratord (Seeed Gateway)
	URL:=https://github.com/chirpstack/chirpstack-concentratord
	DEPENDS:=+chirpstack-concentratord-sx1302
endef

define Package/chirpstack-concentratord-target-seeed-gateway/description
	ChirpStack Concentratord (seeed-gateway)
endef

define Package/chirpstack-concentratord-target-seeed-gateway/conffiles
	/etc/config/chirpstack-concentratord
endef

define Package/chirpstack-concentratord-target-seeed-gateway/install
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/etc/uci-defaults/75_chirpstack-concentratord $(1)/etc/uci-defaults/75_chirpstack-concentratord

	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/chirpstack-concentratord.init $(1)/etc/init.d/chirpstack-concentratord
endef

define Build/Compile
	# Nothing to do
endef

$(eval $(call BuildPackage,chirpstack-concentratord-target-seeed-gateway))
