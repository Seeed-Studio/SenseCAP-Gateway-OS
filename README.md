# SenseCAP Gateway OS

SenseCAP Gateway OS is a multi-protocol IoT gateway operating system designed for industrial indoor scenarios. It is based on OpenWrt and provides a standardized software foundation, covering three key capabilities: LoRaWAN gateway, industrial bus data acquisition, and building compliance reporting.

The system adopts an innovative lightweight architecture combining a Debian host with a single LXC container (OpenWrt). All business logic is encapsulated and runs within a single OpenWrt container, while the host system is responsible only for hardware abstraction and container management. This design achieves optimal resource utilization, strong security isolation, and operational flexibility.

[![license][license-badge]][license]
[![prs][prs-badge]][prs]
[![issues][issues-badge]][issues]
[![release][release-badge]][release]
[![contact][contact-badge]][contact]

[English](README.md) | [中文](README_zh-CN.md) | [日本語](README_ja.md) | [Français](README_fr.md) | [Português](README_pt.md) | [Español](README_es.md)

## Table of Contents

- [Features](#features)
- [Recommended Hardware](#recommended-hardware)
- [Capabilities \& Roadmap](#capabilities--roadmap)
- [Directory Structure](#directory-structure)
- [Getting Started](#getting-started)
  - [System Requirements](#system-requirements)
  - [Install Dependencies](#install-dependencies)
  - [Build Steps](#build-steps)
  - [Customization](#customization)
- [Deployment](#deployment)
- [Function Modules](#function-modules)
  - [LoRaWAN Gateway](#lorawan-gateway)
  - [ChirpStack Concentrator](#chirpstack-concentrator)
  - [LTE/WWAN Support](#ltewwan-support)
  - [Multi-WAN Support](#multi-wan-support)
- [Feeds Description](#feeds-description)
- [FAQ](#faq)
- [Related Links](#related-links)
- [License](#license)
- [Contributing](#contributing)

## Features

- **Minimal Host System**: The Debian host retains only the kernel, LXC toolchain, hardware drivers, and UPS monitoring, without running any application-level services.
- **Single-Container Architecture**: All services (LoRaWAN services, networking, peripheral management, and web services) run within a single LXC container as native OpenWrt packages.
- **Simplified Operations**: The single-container design simplifies configuration management, upgrade rollback, and troubleshooting, reducing operational complexity.

![Features Architecture](docs/images/features_architecture.png)

> **Note:** The diagram above only illustrates the software components running inside the LXC container (OpenWrt). The host system (Debian) layer — including the Linux kernel, hardware drivers, LXC runtime, UPS monitor, and user data storage — is shown separately at the top and is not part of the container image.

## Recommended Hardware

<p align="center">
  <img src="docs/images/recommended_hardware.png" width="700" />
</p>

| **Device** | **Link** |
| --- | --- |
| reComputer R1225 LoRaWAN Gateway & Industrial Controller (US915-4G) | [Get one](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-4G-p-6721.html) |
| reComputer R1225 LoRaWAN Gateway & Industrial Controller (US915) | [Get one](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-p-6722.html) |
| reComputer R1225 LoRaWAN Gateway & Industrial Controller (EU868-4G) | [Get one](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-4G-p-6719.html) |
| reComputer R1225 LoRaWAN Gateway & Industrial Controller (EU868) | [Get one](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-p-6720.html) |

Wiki of reComputer R1225: <https://wiki.seeedstudio.com/r1225_introduction/>

SenseCAP Gateway OS is not only the dedicated system for R1225, but also a portable gateway software solution. It can be adapted to different hardware platforms, enabling partners to rapidly customize and extend the system.

## Capabilities & Roadmap

![Capabilities Roadmap](docs/images/capabilities_roadmap.gif)

![LuCI Menus](docs/images/luci_menus.png)

🔜 **What's next:**

- Add BACnet protocol support for Web configuration
- Optimize Web configuration logic for serial ports and Modbus
- Add a 4G network watchdog service

## Directory Structure

```
recomputer-gateway/
├── .config                    # OpenWrt build configuration
├── .github/
│   └── workflows/
│       └── build.yml          # GitHub Actions build workflow
├── feeds.conf.default         # Feeds configuration
├── feeds/
│   ├── chirpstack/            # ChirpStack related packages
│   ├── lorawan-gateway/       # LoRaWAN Gateway backend services
│   └── luci-lorawan-gateway/  # LuCI Web interface extensions
│       ├── luci-app-gateway/      # Main gateway configuration app
│       ├── luci-app-lora/         # LoRa status display
│       ├── luci-app-lte/          # LTE configuration
│       ├── luci-app-ups/          # UPS power management
│       ├── luci-app-rs485/        # RS485 configuration
│       ├── luci-app-terminal/     # Web terminal
│       ├── luci-app-ota/          # OTA upgrade
│       ├── luci-app-multiwan/     # Multi WAN configuration
│       ├── luci-app-routing/      # Routing configuration
│       └── luci-theme-sensecap/   # SenseCap theme
├── openwrt/                   # OpenWrt source (downloaded during build)
└── README.md                  # This document
```

## Getting Started

### System Requirements

- **OS**: Ubuntu/Debian Linux
- **Disk Space**: > 50GB recommended
- **Memory**: > 8GB recommended

### Install Dependencies

```bash
sudo apt-get update
sudo apt-get install build-essential clang flex bison g++ gawk \
  gcc-multilib g++-multilib gettext git libncurses5-dev \
  libssl-dev rsync unzip zlib1g-dev file wget
```

### Build Steps

#### 1. Initialize Submodules

```bash
git submodule update --init --recursive
```

#### 2. Clone OpenWrt Source

```bash
git clone https://github.com/openwrt/openwrt.git -b openwrt-24.10
cd openwrt
rm -r feeds.conf.default
cp ../feeds.conf.default feeds.conf.default
```

#### 3. Update and Install Feeds

```bash
./scripts/feeds update -a
./scripts/feeds install -a
```

#### 4. Apply Configuration

```bash
cp ../.config .config
make defconfig
```

#### 5. (Optional) Disable Rust LLVM CI Download for Faster Build

```bash
sed -i 's/--set=llvm.download-ci-llvm=true/--set=llvm.download-ci-llvm=false/' \
  feeds/packages/lang/rust/Makefile
```

#### 6. Build

```bash
unset CI GITHUB_ACTIONS CONTINUOUS_INTEGRATION
make -j$(nproc)
```

#### 7. Get Build Output

After completion, the firmware is located at:

```
openwrt/bin/targets/armsr/armv8/openwrt-armsr-armv8-generic-rootfs.tar.gz
```

### Customization

To customize the firmware (e.g., add packages, modify kernel settings), run menuconfig in the openwrt directory:

```bash
cd openwrt
make menuconfig
```

## Deployment

The firmware is deployed to the device via LXC container:

### 1. Stop Existing Container

```bash
sudo lxc-stop -n SenseCAP
```

### 2. Clean and Create New rootfs

```bash
sudo rm -rf /var/lib/lxc/SenseCAP/rootfs
sudo mkdir -p /var/lib/lxc/SenseCAP/rootfs
```

### 3. Extract New Firmware

```bash
sudo tar -xzf /path/to/openwrt-armsr-armv8-generic-rootfs.tar.gz \
  -C /var/lib/lxc/SenseCAP/rootfs
```

### 4. Start Container

```bash
sudo lxc-start -n SenseCAP
```

### 5. SSH to LXC Container

```bash
sudo lxc-attach -n SenseCAP
```

### 6. View Logs

```bash
# LoRa packet forwarder logs
logread | grep lora

# System logs
logread
```

### 7. Web Interface

Access `http://[IP_ADDRESS]/cgi-bin/luci` for:

- **Status Overview**: LoRa status, network connections, packet statistics
- **Services**: LoRa, network and other configurations

## Function Modules

### LoRaWAN Gateway

- **Config File**: `/etc/config/lora_pkt_fwd`
- **Service**: `lorawan_gateway`
- **UI**: LuCI Gateway app

![LoRaWAN Architecture](docs/images/lorawan_architecture.png)

### ChirpStack Concentrator

- **Target**: `seeed-gateway`
- **Service**: `chirpstack-concentratord`

### LTE/WWAN Support

- **Config**: `/etc/config/network`
- **Firewall**: LTE and WWAN networks have firewall rules added

### Multi-WAN Support

Supports multiple WAN configurations including LTE and Ethernet, with load balancing and failover capabilities.

#### Network Interface Architecture

The reComputer R1225 is equipped with **two physical Ethernet ports** (ETH0 and ETH1). These two ports serve different roles based on the host-container architecture:

| Port | Role | Description |
|------|------|-------------|
| **ETH0** | Container (LXC) interface | This interface is **directly mapped (passthrough) from the hardware into the LXC container** via the host's LXC network configuration. The OpenWrt container has full control over this interface, managing it as a standard WAN or LAN port. All application-level traffic (LoRaWAN uplink, MQTT, Web UI access, etc.) flows through this port. |
| **ETH1** | Host (Debian) interface | This interface is **managed by the Debian host system**. It is used for host-level management tasks such as SSH access to the host, container management operations, firmware updates, and UPS monitoring communication. It remains isolated from the container network stack. |

This separation ensures that even if the container network is misconfigured or unreachable, the host management interface remains accessible for recovery and maintenance.

### RS485 / Modbus

- **Config Files**: `/etc/config/rs485-module` (serial, mqtt, protocol)
- **Services**: `rs485-module`, `rs485-modbus`
- **UI**: LuCI RS485 app (Serial Settings, Protocol Settings, MQTT Settings, MQTT Log)

The RS485 module supports industrial protocols including **Modbus RTU** and **BACnet MS/TP**:

- **Modbus RTU**: Polling and parsing of Modbus registers via the RS485 serial interface, with data forwarded through MQTT uplink/downlink.
- **BACnet MS/TP**: BACnet protocol support over RS485 for building automation integration (Web configuration in progress).

![RS485 Architecture](docs/images/rs485_architecture.png)

## Feeds Description

This project uses three custom OpenWrt feeds. They are defined in `feeds.conf.default` and installed into the OpenWrt build system via `./scripts/feeds update && ./scripts/feeds install`.

### chirpstack

ChirpStack LoRaWAN ecosystem integration, including the network server, concentrator daemon, packet forwarders, and their LuCI frontends.

| Package | Description |
|---------|-------------|
| `chirpstack` | ChirpStack LoRaWAN network server |
| `chirpstack-concentratord` | Concentrator packet-forwarder daemon (with per-hardware target builds) |
| `chirpstack-mqtt-forwarder` | MQTT-based packet forwarder (single / slot1 / slot2 / mesh variants) |
| `chirpstack-udp-forwarder` | UDP-based packet forwarder (single / slot1 / slot2 variants) |
| `chirpstack-gateway-mesh` | LoRaWAN mesh networking extension |
| `chirpstack-rest-api` | REST API service for ChirpStack |
| `lorawan-devices` | LoRaWAN device profiles and codec definitions |
| `node-red` | Node-RED visual automation platform |
| `libloragw-sx1301 / sx1302 / 2g4` | Semtech LoRa HAL libraries |
| `luci-app-chirpstack-*` | LuCI web interfaces for all ChirpStack components |
| `luci-theme-argon` | Argon theme for LuCI |

### lorawan-gateway

Gateway hardware integration and backend system services.

| Package | Description |
|---------|-------------|
| `lora` | LoRa radio stack service (Rust) |
| `packetforwarder` | LoRa packet forwarder |
| `chirpstack-concentratord-target-seeed-gateway` | Seeed gateway-specific concentrator build |
| `chirpstack-gateway-bridge` | ChirpStack gateway bridge (MQTT/UDP backend) |
| `basicstation_ubus` | Basic Station protocol with ubus RPC service |
| `lte-serve` | LTE cellular module management service |
| `rs485-module` | RS485 serial communication service (Rust) |
| `rs485-modbus` | RS485 Modbus protocol implementation (Rust) |
| `bacnet-stack` | BACnet protocol stack for building automation |
| `ups-module` | UPS power management service (Rust) |
| `hardware-info` | EEPROM reader for gateway SN, EUI, and hardware info |
| `ubus-serve` | ubus RPC service for system management |
| `wifi-module` | Auto WiFi configuration via USB drive detection |

### luci-lorawan-gateway

LuCI web interface applications and theme for gateway management.

| Package | Description |
|---------|-------------|
| `luci-app-gateway` | Main gateway system configuration |
| `luci-app-lora` | LoRa radio status and configuration |
| `luci-app-chirpstack-concentratord-target-seeed-gateway` | Seeed gateway concentrator configuration |
| `luci-app-lte` | LTE/4G cellular configuration |
| `luci-app-multiwan` | Multi-WAN failover and load balancing |
| `luci-app-routing` | Network routing configuration |
| `luci-app-rs485` | RS485/Modbus interface configuration |
| `luci-app-bacnet` | BACnet protocol configuration |
| `luci-app-ups` | UPS power management |
| `luci-app-ota` | OTA firmware upgrade |
| `luci-app-terminal` | Web-based terminal console |
| `luci-theme-sensecap` | SenseCAP custom theme |

## FAQ

### Build Fails

**Problem**: Errors during compilation

**Solution**:
- Check disk space (recommend > 50GB)
- Ensure submodules are updated: `git submodule update --init --recursive`
- Rust compilation is slow, disable CI LLVM download to speed up

### Cannot Access After Deployment

**Problem**: Cannot access web interface after container starts

**Solution**:
- Check LXC container status: `sudo lxc-ls -f`
- View container logs: `sudo lxc-info -n SenseCAP`
- Verify network configuration is correct

### LoRa Data Not Displaying

**Problem**: No data on LoRa status page

**Solution**:
- Check concentrator service status
- View logs: `logread | grep -i lora`
- Verify gateway configuration is correct

## Related Links

- [OpenWrt](https://openwrt.org/)
- [ChirpStack](https://www.chirpstack.io/)
- [LuCI](https://github.com/openwrt/luci)
- [Seeed Studio](https://www.seeedstudio.com/)

## License

This project follows the OpenWrt project license requirements.

## Contributing

Issues and Pull Requests are welcome!

<!-- Badge links -->
[license-badge]: https://img.shields.io/badge/license-Apache--2.0-green
[license]: LICENSE
[prs-badge]: https://img.shields.io/badge/PRs-welcome-brightgreen
[prs]: https://github.com/Seeed-Studio/recomputer-gateway/pulls
[issues-badge]: https://img.shields.io/badge/Issues-welcome-brightgreen
[issues]: https://github.com/Seeed-Studio/recomputer-gateway/issues
[release-badge]: https://img.shields.io/github/v/release/Seeed-Studio/recomputer-gateway
[release]: https://github.com/Seeed-Studio/recomputer-gateway/releases
[contact-badge]: https://img.shields.io/badge/Contact-sensecap%40seeed.cc-blue
[contact]: mailto:sensecap@seeed.cc
