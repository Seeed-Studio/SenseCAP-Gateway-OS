# SenseCAP Gateway OS

SenseCAP Gateway OS 是一款面向工业室内场景的多协议物联网网关操作系统。它基于 OpenWrt 构建，提供标准化的软件基础平台，涵盖三大核心能力：LoRaWAN 网关、工业总线数据采集和楼宇合规上报。

系统采用创新的轻量级架构，将 Debian 宿主机与单个 LXC 容器（OpenWrt）相结合。所有业务逻辑封装并运行在单个 OpenWrt 容器内，宿主机仅负责硬件抽象和容器管理。这种设计实现了最优的资源利用率、强安全隔离和灵活的运维能力。

[![license][license-badge]][license]
[![prs][prs-badge]][prs]
[![issues][issues-badge]][issues]
[![release][release-badge]][release]
[![contact][contact-badge]][contact]

[English](README.md) | [中文](README_zh-CN.md) | [日本語](README_ja.md) | [Français](README_fr.md) | [Português](README_pt.md) | [Español](README_es.md)

## 目录

- [特性](#特性)
- [推荐硬件](#推荐硬件)
- [功能与路线图](#功能与路线图)
- [目录结构](#目录结构)
- [快速上手](#快速上手)
  - [系统要求](#系统要求)
  - [安装依赖](#安装依赖)
  - [构建步骤](#构建步骤)
  - [自定义配置](#自定义配置)
- [部署](#部署)
- [功能模块](#功能模块)
  - [LoRaWAN 网关](#lorawan-网关)
  - [ChirpStack Concentrator](#chirpstack-concentrator)
  - [LTE/WWAN 支持](#ltewwan-支持)
  - [多 WAN 支持](#多-wan-支持)
  - [RS485 / Modbus](#rs485--modbus)
- [Feeds 说明](#feeds-说明)
- [常见问题](#常见问题)
- [相关链接](#相关链接)
- [许可证](#许可证)
- [贡献](#贡献)

## 特性

- **最小化宿主系统**：Debian 宿主机仅保留内核、LXC 工具链、硬件驱动和 UPS 监控，不运行任何应用层服务。
- **单容器架构**：所有服务（LoRaWAN 服务、网络、外设管理和 Web 服务）均在单个 LXC 容器内作为原生 OpenWrt 软件包运行。
- **简化运维**：单容器设计简化了配置管理、升级回滚和故障排查，降低运维复杂度。

![特性架构图](docs/images/features_architecture.png)

> **注意：** 上图仅展示了 LXC 容器（OpenWrt）内部运行的软件组件。宿主系统（Debian）层——包括 Linux 内核、硬件驱动、LXC 运行时、UPS 监控和用户数据存储——在图中顶部单独展示，不属于容器镜像的一部分。

## 推荐硬件

<p align="center">
  <img src="docs/images/recommended_hardware.png" width="700" />
</p>

| **设备** | **链接** |
| --- | --- |
| reComputer R1225 LoRaWAN 网关 & 工业控制器 (US915-4G) | [购买](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-4G-p-6721.html) |
| reComputer R1225 LoRaWAN 网关 & 工业控制器 (US915) | [购买](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-p-6722.html) |
| reComputer R1225 LoRaWAN 网关 & 工业控制器 (EU868-4G) | [购买](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-4G-p-6719.html) |
| reComputer R1225 LoRaWAN 网关 & 工业控制器 (EU868) | [购买](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-p-6720.html) |

reComputer R1225 Wiki：<https://wiki.seeedstudio.com/r1225_introduction/>

SenseCAP Gateway OS 不仅是 R1225 的专用系统，更是一套可移植的网关软件方案。它可以适配不同的硬件平台，帮助合作伙伴快速定制和扩展系统。

## 功能与路线图

![功能路线图](docs/images/capabilities_roadmap.gif)

![LuCI 菜单](docs/images/luci_menus.png)

🔜 **即将推出：**

- 添加 BACnet 协议的 Web 配置支持
- 优化串口和 Modbus 的 Web 配置逻辑
- 添加 4G 网络看门狗服务

## 目录结构

```
recomputer-gateway/
├── .config                    # OpenWrt 构建配置
├── .github/
│   └── workflows/
│       └── build.yml          # GitHub Actions 构建工作流
├── feeds.conf.default         # Feeds 配置
├── feeds/
│   ├── chirpstack/            # ChirpStack 相关软件包
│   ├── lorawan-gateway/       # LoRaWAN 网关后端服务
│   └── luci-lorawan-gateway/  # LuCI Web 界面扩展
│       ├── luci-app-gateway/      # 网关配置主应用
│       ├── luci-app-lora/         # LoRa 状态显示
│       ├── luci-app-lte/          # LTE 配置
│       ├── luci-app-ups/          # UPS 电源管理
│       ├── luci-app-rs485/        # RS485 配置
│       ├── luci-app-terminal/     # Web 终端
│       ├── luci-app-ota/          # OTA 升级
│       ├── luci-app-multiwan/     # 多 WAN 配置
│       ├── luci-app-routing/      # 路由配置
│       └── luci-theme-sensecap/   # SenseCap 主题
├── openwrt/                   # OpenWrt 源码（构建时下载）
└── README.md                  # 本文档
```

## 快速上手

### 系统要求

- **操作系统**：Ubuntu/Debian Linux
- **磁盘空间**：建议 > 50GB
- **内存**：建议 > 8GB

### 安装依赖

```bash
sudo apt-get update
sudo apt-get install build-essential clang flex bison g++ gawk \
  gcc-multilib g++-multilib gettext git libncurses5-dev \
  libssl-dev rsync unzip zlib1g-dev file wget
```

### 构建步骤

#### 1. 初始化子模块

```bash
git submodule update --init --recursive
```

#### 2. 克隆 OpenWrt 源码

```bash
git clone https://github.com/openwrt/openwrt.git -b openwrt-24.10
cd openwrt
rm -r feeds.conf.default
cp ../feeds.conf.default feeds.conf.default
```

#### 3. 更新并安装 Feeds

```bash
./scripts/feeds update -a
./scripts/feeds install -a
```

#### 4. 应用配置

```bash
cp ../.config .config
make defconfig
```

#### 5.（可选）禁用 Rust LLVM CI 下载以加速构建

```bash
sed -i 's/--set=llvm.download-ci-llvm=true/--set=llvm.download-ci-llvm=false/' \
  feeds/packages/lang/rust/Makefile
```

#### 6. 构建

```bash
unset CI GITHUB_ACTIONS CONTINUOUS_INTEGRATION
make -j$(nproc)
```

#### 7. 获取构建产物

构建完成后，固件位于：

```
openwrt/bin/targets/armsr/armv8/openwrt-armsr-armv8-generic-rootfs.tar.gz
```

### 自定义配置

如需自定义固件（如添加软件包、修改内核设置），在 openwrt 目录下运行 menuconfig：

```bash
cd openwrt
make menuconfig
```

## 部署

固件通过 LXC 容器部署到设备：

### 1. 停止现有容器

```bash
sudo lxc-stop -n SenseCAP
```

### 2. 清理并创建新的 rootfs

```bash
sudo rm -rf /var/lib/lxc/SenseCAP/rootfs
sudo mkdir -p /var/lib/lxc/SenseCAP/rootfs
```

### 3. 解压新固件

```bash
sudo tar -xzf /path/to/openwrt-armsr-armv8-generic-rootfs.tar.gz \
  -C /var/lib/lxc/SenseCAP/rootfs
```

### 4. 启动容器

```bash
sudo lxc-start -n SenseCAP
```

### 5. 进入 LXC 容器

```bash
sudo lxc-attach -n SenseCAP
```

### 6. 查看日志

```bash
# LoRa 数据包转发日志
logread | grep lora

# 系统日志
logread
```

### 7. Web 管理界面

访问 `http://[IP地址]/cgi-bin/luci`，可查看：

- **状态概览**：LoRa 状态、网络连接、数据包统计
- **服务配置**：LoRa、网络及其他配置项

## 功能模块

### LoRaWAN 网关

- **配置文件**：`/etc/config/lora_pkt_fwd`
- **服务**：`lorawan_gateway`
- **界面**：LuCI Gateway 应用

![LoRaWAN 架构](docs/images/lorawan_architecture.png)

### ChirpStack Concentrator

- **目标平台**：`seeed-gateway`
- **服务**：`chirpstack-concentratord`

### LTE/WWAN 支持

- **配置文件**：`/etc/config/network`
- **防火墙**：已为 LTE 和 WWAN 网络添加防火墙规则

### 多 WAN 支持

支持包括 LTE 和以太网在内的多种 WAN 配置，具备负载均衡和故障切换能力。

#### 网络接口架构

reComputer R1225 配备了**两个物理以太网口**（ETH0 和 ETH1）。基于宿主机-容器架构，这两个网口承担不同的角色：

| 网口 | 角色 | 说明 |
|------|------|------|
| **ETH0** | 容器（LXC）接口 | 该接口通过宿主机的 LXC 网络配置，**从硬件直接映射（passthrough）到 LXC 容器内部**。OpenWrt 容器对该接口拥有完全控制权，将其作为标准 WAN 或 LAN 口管理。所有应用层流量（LoRaWAN 上行、MQTT、Web 管理界面访问等）均通过此端口传输。 |
| **ETH1** | 宿主机（Debian）接口 | 该接口**由 Debian 宿主系统管理**。用于宿主机级别的管理任务，如 SSH 登录宿主机、容器管理操作、固件更新和 UPS 监控通信等。该接口与容器网络栈相互隔离。 |

这种分离设计确保即使容器网络配置错误或不可达，宿主机管理接口仍然可用于恢复和维护操作。

### RS485 / Modbus

- **配置文件**：`/etc/config/rs485-module`（串口、MQTT、协议）
- **服务**：`rs485-module`、`rs485-modbus`
- **界面**：LuCI RS485 应用（串口设置、协议设置、MQTT 设置、MQTT 日志）

RS485 模块支持的工业协议包括 **Modbus RTU** 和 **BACnet MS/TP**：

- **Modbus RTU**：通过 RS485 串口轮询和解析 Modbus 寄存器，数据通过 MQTT 上下行转发。
- **BACnet MS/TP**：基于 RS485 的 BACnet 协议支持，用于楼宇自动化集成（Web 配置开发中）。

![RS485 架构](docs/images/rs485_architecture.png)

## Feeds 说明

本项目使用三个自定义 OpenWrt feeds，定义在 `feeds.conf.default` 中，通过 `./scripts/feeds update && ./scripts/feeds install` 安装到 OpenWrt 构建系统。

### chirpstack

ChirpStack LoRaWAN 生态集成，包含网络服务器、集中器守护进程、数据包转发器及其 LuCI 前端。

| 软件包 | 说明 |
|--------|------|
| `chirpstack` | ChirpStack LoRaWAN 网络服务器 |
| `chirpstack-concentratord` | 集中器数据包转发守护进程（含各硬件平台适配构建） |
| `chirpstack-mqtt-forwarder` | 基于 MQTT 的数据包转发器（single / slot1 / slot2 / mesh 变体） |
| `chirpstack-udp-forwarder` | 基于 UDP 的数据包转发器（single / slot1 / slot2 变体） |
| `chirpstack-gateway-mesh` | LoRaWAN Mesh 网络扩展 |
| `chirpstack-rest-api` | ChirpStack REST API 服务 |
| `lorawan-devices` | LoRaWAN 设备配置文件和编解码定义 |
| `node-red` | Node-RED 可视化自动化平台 |
| `libloragw-sx1301 / sx1302 / 2g4` | Semtech LoRa 硬件抽象层库 |
| `luci-app-chirpstack-*` | 所有 ChirpStack 组件的 LuCI Web 界面 |
| `luci-theme-argon` | LuCI Argon 主题 |

### lorawan-gateway

网关硬件集成和后端系统服务。

| 软件包 | 说明 |
|--------|------|
| `lora` | LoRa 射频协议栈服务（Rust） |
| `packetforwarder` | LoRa 数据包转发器 |
| `chirpstack-concentratord-target-seeed-gateway` | Seeed 网关专用集中器构建 |
| `chirpstack-gateway-bridge` | ChirpStack 网关桥接服务（MQTT/UDP 后端） |
| `basicstation_ubus` | Basic Station 协议与 ubus RPC 服务 |
| `lte-serve` | LTE 蜂窝模块管理服务 |
| `rs485-module` | RS485 串口通信服务（Rust） |
| `rs485-modbus` | RS485 Modbus 协议实现（Rust） |
| `bacnet-stack` | BACnet 协议栈，用于楼宇自动化 |
| `ups-module` | UPS 电源管理服务（Rust） |
| `hardware-info` | EEPROM 读取工具，获取网关 SN、EUI 和硬件信息 |
| `ubus-serve` | ubus RPC 系统管理服务 |
| `wifi-module` | 通过 USB 设备自动配置 WiFi |

### luci-lorawan-gateway

网关管理的 LuCI Web 界面应用和主题。

| 软件包 | 说明 |
|--------|------|
| `luci-app-gateway` | 网关系统主配置界面 |
| `luci-app-lora` | LoRa 射频状态与配置 |
| `luci-app-chirpstack-concentratord-target-seeed-gateway` | Seeed 网关集中器配置界面 |
| `luci-app-lte` | LTE/4G 蜂窝网络配置 |
| `luci-app-multiwan` | 多 WAN 故障切换与负载均衡 |
| `luci-app-routing` | 网络路由配置 |
| `luci-app-rs485` | RS485/Modbus 接口配置 |
| `luci-app-bacnet` | BACnet 协议配置 |
| `luci-app-ups` | UPS 电源管理 |
| `luci-app-ota` | OTA 固件升级 |
| `luci-app-terminal` | Web 终端控制台 |
| `luci-theme-sensecap` | SenseCAP 自定义主题 |

## 常见问题

### 构建失败

**问题**：编译过程中出现错误

**解决方案**：
- 检查磁盘空间（建议 > 50GB）
- 确保子模块已更新：`git submodule update --init --recursive`
- Rust 编译较慢，可禁用 CI LLVM 下载以加速构建

### 部署后无法访问

**问题**：容器启动后无法访问 Web 管理界面

**解决方案**：
- 检查 LXC 容器状态：`sudo lxc-ls -f`
- 查看容器日志：`sudo lxc-info -n SenseCAP`
- 验证网络配置是否正确

### LoRa 数据不显示

**问题**：LoRa 状态页面无数据

**解决方案**：
- 检查 concentrator 服务状态
- 查看日志：`logread | grep -i lora`
- 验证网关配置是否正确

## 相关链接

- [OpenWrt](https://openwrt.org/)
- [ChirpStack](https://www.chirpstack.io/)
- [LuCI](https://github.com/openwrt/luci)
- [Seeed Studio](https://www.seeedstudio.com/)

## 许可证

本项目遵循 OpenWrt 项目的许可证要求。

## 贡献

欢迎提交 Issue 和 Pull Request！

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
