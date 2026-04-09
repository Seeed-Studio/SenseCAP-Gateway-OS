# SenseCAP Gateway OS

SenseCAP Gateway OS es un sistema operativo de gateway IoT multiprotocolo diseñado para escenarios industriales interiores. Basado en OpenWrt, proporciona una base de software estandarizada que abarca tres capacidades clave: gateway LoRaWAN, adquisición de datos de bus industrial e informes de cumplimiento de edificios.

El sistema adopta una arquitectura ligera innovadora que combina un host Debian con un único contenedor LXC (OpenWrt). Toda la lógica de negocio está encapsulada y se ejecuta dentro de un único contenedor OpenWrt, mientras que el sistema host es responsable únicamente de la abstracción de hardware y la gestión de contenedores. Este diseño logra una utilización óptima de recursos, fuerte aislamiento de seguridad y flexibilidad operativa.

[![license][license-badge]][license]
[![prs][prs-badge]][prs]
[![issues][issues-badge]][issues]
[![release][release-badge]][release]
[![contact][contact-badge]][contact]

[English](README.md) | [中文](README_zh-CN.md) | [日本語](README_ja.md) | [Français](README_fr.md) | [Português](README_pt.md) | [Español](README_es.md)

## Tabla de contenidos

- [Características](#características)
- [Hardware recomendado](#hardware-recomendado)
- [Capacidades y hoja de ruta](#capacidades-y-hoja-de-ruta)
- [Estructura de directorios](#estructura-de-directorios)
- [Primeros pasos](#primeros-pasos)
  - [Requisitos del sistema](#requisitos-del-sistema)
  - [Instalación de dependencias](#instalación-de-dependencias)
  - [Pasos de compilación](#pasos-de-compilación)
  - [Personalización](#personalización)
- [Despliegue](#despliegue)
- [Módulos funcionales](#módulos-funcionales)
  - [Gateway LoRaWAN](#gateway-lorawan)
  - [Concentrador ChirpStack](#concentrador-chirpstack)
  - [Soporte LTE/WWAN](#soporte-ltewwan)
  - [Soporte Multi-WAN](#soporte-multi-wan)
- [Descripción de los feeds](#descripción-de-los-feeds)
- [FAQ](#faq)
- [Enlaces relacionados](#enlaces-relacionados)
- [Licencia](#licencia)
- [Contribución](#contribución)

## Características

- **Sistema host mínimo**: El host Debian conserva únicamente el kernel, la cadena de herramientas LXC, los controladores de hardware y la monitorización UPS, sin ejecutar servicios a nivel de aplicación.
- **Arquitectura de contenedor único**: Todos los servicios (servicios LoRaWAN, red, gestión de periféricos y servicios Web) se ejecutan dentro de un único contenedor LXC como paquetes nativos de OpenWrt.
- **Operaciones simplificadas**: El diseño de contenedor único simplifica la gestión de configuración, la reversión de actualizaciones y la resolución de problemas, reduciendo la complejidad operativa.

![Arquitectura de características](docs/images/features_architecture.png)

> **Nota:** El diagrama anterior solo ilustra los componentes de software que se ejecutan dentro del contenedor LXC (OpenWrt). La capa del sistema host (Debian) — incluyendo el kernel Linux, los controladores de hardware, el runtime LXC, el monitor UPS y el almacenamiento de datos de usuario — se muestra por separado en la parte superior y no forma parte de la imagen del contenedor.

## Hardware recomendado

<p align="center">
  <img src="docs/images/recommended_hardware.png" width="700" />
</p>

| **Dispositivo** | **Enlace** |
| --- | --- |
| reComputer R1225 Gateway LoRaWAN & Controlador Industrial (US915-4G) | [Comprar](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-4G-p-6721.html) |
| reComputer R1225 Gateway LoRaWAN & Controlador Industrial (US915) | [Comprar](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-p-6722.html) |
| reComputer R1225 Gateway LoRaWAN & Controlador Industrial (EU868-4G) | [Comprar](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-4G-p-6719.html) |
| reComputer R1225 Gateway LoRaWAN & Controlador Industrial (EU868) | [Comprar](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-p-6720.html) |

Wiki del reComputer R1225: <https://wiki.seeedstudio.com/r1225_introduction/>

SenseCAP Gateway OS no es solo el sistema dedicado para el R1225, sino también una solución de software de gateway portátil. Puede adaptarse a diferentes plataformas de hardware, permitiendo a los socios personalizar y extender rápidamente el sistema.

## Capacidades y hoja de ruta

![Hoja de ruta de capacidades](docs/images/capabilities_roadmap.gif)

![Menús LuCI](docs/images/luci_menus.png)

🔜 **Próximamente:**

- Añadir soporte de configuración Web para el protocolo BACnet
- Optimizar la lógica de configuración Web para puertos serie y Modbus
- Añadir servicio de watchdog de red 4G

## Estructura de directorios

```
recomputer-gateway/
├── .config                    # Configuración de compilación OpenWrt
├── .github/
│   └── workflows/
│       └── build.yml          # Workflow de compilación GitHub Actions
├── feeds.conf.default         # Configuración de feeds
├── feeds/
│   ├── chirpstack/            # Paquetes relacionados con ChirpStack
│   ├── lorawan-gateway/       # Servicios backend del gateway LoRaWAN
│   └── luci-lorawan-gateway/  # Extensiones de interfaz Web LuCI
│       ├── luci-app-gateway/      # Aplicación de configuración principal
│       ├── luci-app-lora/         # Visualización de estado LoRa
│       ├── luci-app-lte/          # Configuración LTE
│       ├── luci-app-ups/          # Gestión de energía UPS
│       ├── luci-app-rs485/        # Configuración RS485
│       ├── luci-app-terminal/     # Terminal Web
│       ├── luci-app-ota/          # Actualización OTA
│       ├── luci-app-multiwan/     # Configuración Multi WAN
│       ├── luci-app-routing/      # Configuración de enrutamiento
│       └── luci-theme-sensecap/   # Tema SenseCap
├── openwrt/                   # Fuente OpenWrt (descargado durante la compilación)
└── README.md                  # Este documento
```

## Primeros pasos

### Requisitos del sistema

- **SO**: Ubuntu/Debian Linux
- **Espacio en disco**: > 50GB recomendado
- **Memoria**: > 8GB recomendado

### Instalación de dependencias

```bash
sudo apt-get update
sudo apt-get install build-essential clang flex bison g++ gawk \
  gcc-multilib g++-multilib gettext git libncurses5-dev \
  libssl-dev rsync unzip zlib1g-dev file wget
```

### Pasos de compilación

#### 1. Inicializar submódulos

```bash
git submodule update --init --recursive
```

#### 2. Clonar el código fuente de OpenWrt

```bash
git clone https://github.com/openwrt/openwrt.git -b openwrt-24.10
cd openwrt
rm -r feeds.conf.default
cp ../feeds.conf.default feeds.conf.default
```

#### 3. Actualizar e instalar feeds

```bash
./scripts/feeds update -a
./scripts/feeds install -a
```

#### 4. Aplicar configuración

```bash
cp ../.config .config
make defconfig
```

#### 5. (Opcional) Desactivar la descarga CI LLVM de Rust para compilación más rápida

```bash
sed -i 's/--set=llvm.download-ci-llvm=true/--set=llvm.download-ci-llvm=false/' \
  feeds/packages/lang/rust/Makefile
```

#### 6. Compilar

```bash
unset CI GITHUB_ACTIONS CONTINUOUS_INTEGRATION
make -j$(nproc)
```

#### 7. Obtener resultado de compilación

Después de completar, el firmware se encuentra en:

```
openwrt/bin/targets/armsr/armv8/openwrt-armsr-armv8-generic-rootfs.tar.gz
```

### Personalización

Para personalizar el firmware (añadir paquetes, modificar configuraciones del kernel), ejecute menuconfig en el directorio openwrt:

```bash
cd openwrt
make menuconfig
```

## Despliegue

El firmware se despliega en el dispositivo a través de un contenedor LXC:

### 1. Detener el contenedor existente

```bash
sudo lxc-stop -n SenseCAP
```

### 2. Limpiar y crear nuevo rootfs

```bash
sudo rm -rf /var/lib/lxc/SenseCAP/rootfs
sudo mkdir -p /var/lib/lxc/SenseCAP/rootfs
```

### 3. Extraer nuevo firmware

```bash
sudo tar -xzf /path/to/openwrt-armsr-armv8-generic-rootfs.tar.gz \
  -C /var/lib/lxc/SenseCAP/rootfs
```

### 4. Iniciar el contenedor

```bash
sudo lxc-start -n SenseCAP
```

### 5. SSH al contenedor LXC

```bash
sudo lxc-attach -n SenseCAP
```

### 6. Ver logs

```bash
# Logs del packet forwarder LoRa
logread | grep lora

# Logs del sistema
logread
```

### 7. Interfaz Web

Acceda a `http://[IP_ADDRESS]/cgi-bin/luci` para:

- **Resumen de estado**: Estado LoRa, conexiones de red, estadísticas de paquetes
- **Servicios**: Configuración de LoRa, red y otros

## Módulos funcionales

### Gateway LoRaWAN

- **Archivo de configuración**: `/etc/config/lora_pkt_fwd`
- **Servicio**: `lorawan_gateway`
- **UI**: Aplicación LuCI Gateway

![Arquitectura LoRaWAN](docs/images/lorawan_architecture.png)

### Concentrador ChirpStack

- **Objetivo**: `seeed-gateway`
- **Servicio**: `chirpstack-concentratord`

### Soporte LTE/WWAN

- **Configuración**: `/etc/config/network`
- **Firewall**: Reglas de firewall añadidas para redes LTE y WWAN

### Soporte Multi-WAN

Soporta múltiples configuraciones WAN incluyendo LTE y Ethernet, con balanceo de carga y capacidades de failover.

#### Arquitectura de interfaces de red

El reComputer R1225 está equipado con **dos puertos Ethernet físicos** (ETH0 y ETH1). Estos puertos desempeñan diferentes roles basados en la arquitectura host-contenedor:

| Puerto | Rol | Descripción |
|------|------|-------------|
| **ETH0** | Interfaz del contenedor (LXC) | Esta interfaz está **mapeada directamente (passthrough) desde el hardware al contenedor LXC** a través de la configuración de red LXC del host. El contenedor OpenWrt tiene control total sobre esta interfaz, gestionándola como un puerto WAN o LAN estándar. Todo el tráfico a nivel de aplicación (uplink LoRaWAN, MQTT, acceso Web UI, etc.) fluye a través de este puerto. |
| **ETH1** | Interfaz del host (Debian) | Esta interfaz es **gestionada por el sistema host Debian**. Se utiliza para tareas de administración a nivel de host como acceso SSH, operaciones de gestión de contenedores, actualizaciones de firmware y comunicación de monitorización UPS. Permanece aislada de la pila de red del contenedor. |

Esta separación garantiza que incluso si la red del contenedor está mal configurada o es inaccesible, la interfaz de gestión del host permanece accesible para recuperación y mantenimiento.

### RS485 / Modbus

- **Archivos de configuración**: `/etc/config/rs485-module` (serial, MQTT, protocolo)
- **Servicios**: `rs485-module`, `rs485-modbus`
- **UI**: Aplicación LuCI RS485 (Configuración serial, Configuración de protocolo, Configuración MQTT, Log MQTT)

El módulo RS485 soporta protocolos industriales incluyendo **Modbus RTU** y **BACnet MS/TP**:

- **Modbus RTU**: Sondeo y análisis de registros Modbus a través de la interfaz serial RS485, con reenvío de datos mediante uplink/downlink MQTT.
- **BACnet MS/TP**: Soporte del protocolo BACnet sobre RS485 para integración de automatización de edificios (configuración Web en progreso).

![Arquitectura RS485](docs/images/rs485_architecture.png)

## Descripción de los feeds

Este proyecto utiliza tres feeds OpenWrt personalizados. Están definidos en `feeds.conf.default` e instalados en el sistema de compilación OpenWrt mediante `./scripts/feeds update && ./scripts/feeds install`.

### chirpstack

Integración del ecosistema ChirpStack LoRaWAN, incluyendo el servidor de red, daemon concentrador, packet forwarders y sus frontends LuCI.

| Paquete | Descripción |
|---------|-------------|
| `chirpstack` | Servidor de red ChirpStack LoRaWAN |
| `chirpstack-concentratord` | Daemon packet forwarder del concentrador (con builds por hardware) |
| `chirpstack-mqtt-forwarder` | Packet forwarder basado en MQTT (variantes single / slot1 / slot2 / mesh) |
| `chirpstack-udp-forwarder` | Packet forwarder basado en UDP (variantes single / slot1 / slot2) |
| `chirpstack-gateway-mesh` | Extensión de red mesh LoRaWAN |
| `chirpstack-rest-api` | Servicio de API REST para ChirpStack |
| `lorawan-devices` | Perfiles de dispositivos LoRaWAN y definiciones de codecs |
| `node-red` | Plataforma de automatización visual Node-RED |
| `libloragw-sx1301 / sx1302 / 2g4` | Bibliotecas HAL LoRa Semtech |
| `luci-app-chirpstack-*` | Interfaces Web LuCI para todos los componentes ChirpStack |
| `luci-theme-argon` | Tema Argon para LuCI |

### lorawan-gateway

Integración de hardware del gateway y servicios de sistema backend.

| Paquete | Descripción |
|---------|-------------|
| `lora` | Servicio de pila de radio LoRa (Rust) |
| `packetforwarder` | Packet forwarder LoRa |
| `chirpstack-concentratord-target-seeed-gateway` | Build de concentrador específico para Seeed gateway |
| `chirpstack-gateway-bridge` | Bridge de gateway ChirpStack (backend MQTT/UDP) |
| `basicstation_ubus` | Protocolo Basic Station con servicio RPC ubus |
| `lte-serve` | Servicio de gestión de módulo celular LTE |
| `rs485-module` | Servicio de comunicación serial RS485 (Rust) |
| `rs485-modbus` | Implementación del protocolo Modbus RS485 (Rust) |
| `bacnet-stack` | Pila de protocolo BACnet para automatización de edificios |
| `ups-module` | Servicio de gestión de energía UPS (Rust) |
| `hardware-info` | Lector EEPROM para SN, EUI e información de hardware del gateway |
| `ubus-serve` | Servicio RPC ubus para gestión del sistema |
| `wifi-module` | Configuración WiFi automática mediante detección de unidad USB |

### luci-lorawan-gateway

Aplicaciones de interfaz Web LuCI y tema para gestión del gateway.

| Paquete | Descripción |
|---------|-------------|
| `luci-app-gateway` | Configuración principal del sistema de gateway |
| `luci-app-lora` | Estado y configuración de radio LoRa |
| `luci-app-chirpstack-concentratord-target-seeed-gateway` | Configuración del concentrador Seeed gateway |
| `luci-app-lte` | Configuración celular LTE/4G |
| `luci-app-multiwan` | Failover Multi-WAN y balanceo de carga |
| `luci-app-routing` | Configuración de enrutamiento de red |
| `luci-app-rs485` | Configuración de interfaz RS485/Modbus |
| `luci-app-bacnet` | Configuración de protocolo BACnet |
| `luci-app-ups` | Gestión de energía UPS |
| `luci-app-ota` | Actualización OTA de firmware |
| `luci-app-terminal` | Consola terminal basada en Web |
| `luci-theme-sensecap` | Tema personalizado SenseCAP |

## FAQ

### Fallo en la compilación

**Problema**: Errores durante la compilación

**Solución**:
- Verificar espacio en disco (> 50GB recomendado)
- Asegurar que los submódulos están actualizados: `git submodule update --init --recursive`
- La compilación de Rust es lenta, desactive la descarga CI LLVM para acelerar

### No se puede acceder después del despliegue

**Problema**: No se puede acceder a la interfaz Web después de iniciar el contenedor

**Solución**:
- Verificar el estado del contenedor LXC: `sudo lxc-ls -f`
- Ver logs del contenedor: `sudo lxc-info -n SenseCAP`
- Verificar que la configuración de red es correcta

### Datos LoRa no se muestran

**Problema**: Sin datos en la página de estado LoRa

**Solución**:
- Verificar el estado del servicio concentrador
- Ver logs: `logread | grep -i lora`
- Verificar que la configuración del gateway es correcta

## Enlaces relacionados

- [OpenWrt](https://openwrt.org/)
- [ChirpStack](https://www.chirpstack.io/)
- [LuCI](https://github.com/openwrt/luci)
- [Seeed Studio](https://www.seeedstudio.com/)

## Licencia

Este proyecto sigue los requisitos de licencia del proyecto OpenWrt.

## Contribución

¡Se aceptan Issues y Pull Requests!

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
