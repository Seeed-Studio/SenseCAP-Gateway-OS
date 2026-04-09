# SenseCAP Gateway OS

O SenseCAP Gateway OS é um sistema operacional de gateway IoT multiprotocolo projetado para cenários industriais internos. Baseado no OpenWrt, fornece uma base de software padronizada que abrange três capacidades principais: gateway LoRaWAN, aquisição de dados de barramento industrial e relatórios de conformidade predial.

O sistema adota uma arquitetura leve inovadora que combina um host Debian com um único contêiner LXC (OpenWrt). Toda a lógica de negócios é encapsulada e executada dentro de um único contêiner OpenWrt, enquanto o sistema host é responsável apenas pela abstração de hardware e gerenciamento de contêineres. Este design alcança utilização otimizada de recursos, forte isolamento de segurança e flexibilidade operacional.

[![license][license-badge]][license]
[![prs][prs-badge]][prs]
[![issues][issues-badge]][issues]
[![release][release-badge]][release]
[![contact][contact-badge]][contact]

[English](README.md) | [中文](README_zh-CN.md) | [日本語](README_ja.md) | [Français](README_fr.md) | [Português](README_pt.md) | [Español](README_es.md)

## Índice

- [Características](#características)
- [Hardware recomendado](#hardware-recomendado)
- [Capacidades e roteiro](#capacidades-e-roteiro)
- [Estrutura de diretórios](#estrutura-de-diretórios)
- [Primeiros passos](#primeiros-passos)
  - [Requisitos do sistema](#requisitos-do-sistema)
  - [Instalação de dependências](#instalação-de-dependências)
  - [Etapas de compilação](#etapas-de-compilação)
  - [Personalização](#personalização)
- [Implantação](#implantação)
- [Módulos funcionais](#módulos-funcionais)
  - [Gateway LoRaWAN](#gateway-lorawan)
  - [Concentrador ChirpStack](#concentrador-chirpstack)
  - [Suporte LTE/WWAN](#suporte-ltewwan)
  - [Suporte Multi-WAN](#suporte-multi-wan)
- [Descrição dos feeds](#descrição-dos-feeds)
- [FAQ](#faq)
- [Links relacionados](#links-relacionados)
- [Licença](#licença)
- [Contribuição](#contribuição)

## Características

- **Sistema host mínimo**: O host Debian mantém apenas o kernel, a cadeia de ferramentas LXC, drivers de hardware e monitoramento UPS, sem executar serviços de nível aplicativo.
- **Arquitetura de contêiner único**: Todos os serviços (serviços LoRaWAN, rede, gerenciamento de periféricos e serviços Web) executam dentro de um único contêiner LXC como pacotes nativos OpenWrt.
- **Operações simplificadas**: O design de contêiner único simplifica o gerenciamento de configuração, rollback de atualizações e resolução de problemas, reduzindo a complexidade operacional.

![Arquitetura de recursos](docs/images/features_architecture.png)

> **Nota:** O diagrama acima ilustra apenas os componentes de software executados dentro do contêiner LXC (OpenWrt). A camada do sistema host (Debian) — incluindo o kernel Linux, drivers de hardware, runtime LXC, monitor UPS e armazenamento de dados do usuário — é mostrada separadamente no topo e não faz parte da imagem do contêiner.

## Hardware recomendado

<p align="center">
  <img src="docs/images/recommended_hardware.png" width="700" />
</p>

| **Dispositivo** | **Link** |
| --- | --- |
| reComputer R1225 Gateway LoRaWAN & Controlador Industrial (US915-4G) | [Comprar](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-4G-p-6721.html) |
| reComputer R1225 Gateway LoRaWAN & Controlador Industrial (US915) | [Comprar](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-p-6722.html) |
| reComputer R1225 Gateway LoRaWAN & Controlador Industrial (EU868-4G) | [Comprar](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-4G-p-6719.html) |
| reComputer R1225 Gateway LoRaWAN & Controlador Industrial (EU868) | [Comprar](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-p-6720.html) |

Wiki do reComputer R1225: <https://wiki.seeedstudio.com/r1225_introduction/>

O SenseCAP Gateway OS não é apenas o sistema dedicado para o R1225, mas também uma solução de software de gateway portátil. Pode ser adaptado a diferentes plataformas de hardware, permitindo que parceiros personalizem e estendam rapidamente o sistema.

## Capacidades e roteiro

![Roteiro de capacidades](docs/images/capabilities_roadmap.gif)

![Menus LuCI](docs/images/luci_menus.png)

🔜 **Próximos passos:**

- Adicionar suporte de configuração Web para o protocolo BACnet
- Otimizar a lógica de configuração Web para portas seriais e Modbus
- Adicionar serviço de watchdog de rede 4G

## Estrutura de diretórios

```
recomputer-gateway/
├── .config                    # Configuração de compilação OpenWrt
├── .github/
│   └── workflows/
│       └── build.yml          # Workflow de compilação GitHub Actions
├── feeds.conf.default         # Configuração de feeds
├── feeds/
│   ├── chirpstack/            # Pacotes relacionados ao ChirpStack
│   ├── lorawan-gateway/       # Serviços backend do gateway LoRaWAN
│   └── luci-lorawan-gateway/  # Extensões da interface Web LuCI
│       ├── luci-app-gateway/      # Aplicativo de configuração principal
│       ├── luci-app-lora/         # Exibição de status LoRa
│       ├── luci-app-lte/          # Configuração LTE
│       ├── luci-app-ups/          # Gerenciamento de energia UPS
│       ├── luci-app-rs485/        # Configuração RS485
│       ├── luci-app-terminal/     # Terminal Web
│       ├── luci-app-ota/          # Atualização OTA
│       ├── luci-app-multiwan/     # Configuração Multi WAN
│       ├── luci-app-routing/      # Configuração de roteamento
│       └── luci-theme-sensecap/   # Tema SenseCap
├── openwrt/                   # Fonte OpenWrt (baixado durante a compilação)
└── README.md                  # Este documento
```

## Primeiros passos

### Requisitos do sistema

- **SO**: Ubuntu/Debian Linux
- **Espaço em disco**: > 50GB recomendado
- **Memória**: > 8GB recomendado

### Instalação de dependências

```bash
sudo apt-get update
sudo apt-get install build-essential clang flex bison g++ gawk \
  gcc-multilib g++-multilib gettext git libncurses5-dev \
  libssl-dev rsync unzip zlib1g-dev file wget
```

### Etapas de compilação

#### 1. Inicializar submódulos

```bash
git submodule update --init --recursive
```

#### 2. Clonar o fonte OpenWrt

```bash
git clone https://github.com/openwrt/openwrt.git -b openwrt-24.10
cd openwrt
rm -r feeds.conf.default
cp ../feeds.conf.default feeds.conf.default
```

#### 3. Atualizar e instalar feeds

```bash
./scripts/feeds update -a
./scripts/feeds install -a
```

#### 4. Aplicar configuração

```bash
cp ../.config .config
make defconfig
```

#### 5. (Opcional) Desativar download CI LLVM do Rust para compilação mais rápida

```bash
sed -i 's/--set=llvm.download-ci-llvm=true/--set=llvm.download-ci-llvm=false/' \
  feeds/packages/lang/rust/Makefile
```

#### 6. Compilar

```bash
unset CI GITHUB_ACTIONS CONTINUOUS_INTEGRATION
make -j$(nproc)
```

#### 7. Obter resultado da compilação

Após a conclusão, o firmware está localizado em:

```
openwrt/bin/targets/armsr/armv8/openwrt-armsr-armv8-generic-rootfs.tar.gz
```

### Personalização

Para personalizar o firmware (adicionar pacotes, modificar configurações do kernel), execute menuconfig no diretório openwrt:

```bash
cd openwrt
make menuconfig
```

## Implantação

O firmware é implantado no dispositivo via contêiner LXC:

### 1. Parar o contêiner existente

```bash
sudo lxc-stop -n SenseCAP
```

### 2. Limpar e criar novo rootfs

```bash
sudo rm -rf /var/lib/lxc/SenseCAP/rootfs
sudo mkdir -p /var/lib/lxc/SenseCAP/rootfs
```

### 3. Extrair novo firmware

```bash
sudo tar -xzf /path/to/openwrt-armsr-armv8-generic-rootfs.tar.gz \
  -C /var/lib/lxc/SenseCAP/rootfs
```

### 4. Iniciar o contêiner

```bash
sudo lxc-start -n SenseCAP
```

### 5. SSH para o contêiner LXC

```bash
sudo lxc-attach -n SenseCAP
```

### 6. Visualizar logs

```bash
# Logs do packet forwarder LoRa
logread | grep lora

# Logs do sistema
logread
```

### 7. Interface Web

Acesse `http://[IP_ADDRESS]/cgi-bin/luci` para:

- **Visão geral do status**: Status LoRa, conexões de rede, estatísticas de pacotes
- **Serviços**: Configurações de LoRa, rede e outros

## Módulos funcionais

### Gateway LoRaWAN

- **Arquivo de configuração**: `/etc/config/lora_pkt_fwd`
- **Serviço**: `lorawan_gateway`
- **UI**: Aplicativo LuCI Gateway

![Arquitetura LoRaWAN](docs/images/lorawan_architecture.png)

### Concentrador ChirpStack

- **Alvo**: `seeed-gateway`
- **Serviço**: `chirpstack-concentratord`

### Suporte LTE/WWAN

- **Configuração**: `/etc/config/network`
- **Firewall**: Regras de firewall adicionadas para redes LTE e WWAN

### Suporte Multi-WAN

Suporta múltiplas configurações WAN incluindo LTE e Ethernet, com balanceamento de carga e capacidades de failover.

#### Arquitetura de interfaces de rede

O reComputer R1225 é equipado com **duas portas Ethernet físicas** (ETH0 e ETH1). Essas portas desempenham papéis diferentes com base na arquitetura host-contêiner:

| Porta | Função | Descrição |
|------|------|-------------|
| **ETH0** | Interface do contêiner (LXC) | Esta interface é **mapeada diretamente (passthrough) do hardware para o contêiner LXC** através da configuração de rede LXC do host. O contêiner OpenWrt tem controle total sobre esta interface, gerenciando-a como uma porta WAN ou LAN padrão. Todo o tráfego de nível aplicativo (uplink LoRaWAN, MQTT, acesso Web UI, etc.) flui por esta porta. |
| **ETH1** | Interface do host (Debian) | Esta interface é **gerenciada pelo sistema host Debian**. É utilizada para tarefas de gerenciamento ao nível do host, como acesso SSH, operações de gerenciamento de contêineres, atualizações de firmware e comunicação de monitoramento UPS. Permanece isolada da pilha de rede do contêiner. |

Esta separação garante que, mesmo se a rede do contêiner estiver mal configurada ou inacessível, a interface de gerenciamento do host permanece acessível para recuperação e manutenção.

### RS485 / Modbus

- **Arquivos de configuração**: `/etc/config/rs485-module` (serial, MQTT, protocolo)
- **Serviços**: `rs485-module`, `rs485-modbus`
- **UI**: Aplicativo LuCI RS485 (Configurações seriais, Configurações de protocolo, Configurações MQTT, Log MQTT)

O módulo RS485 suporta protocolos industriais incluindo **Modbus RTU** e **BACnet MS/TP**:

- **Modbus RTU**: Polling e análise de registradores Modbus via interface serial RS485, com encaminhamento de dados através de uplink/downlink MQTT.
- **BACnet MS/TP**: Suporte ao protocolo BACnet sobre RS485 para integração de automação predial (configuração Web em andamento).

![Arquitetura RS485](docs/images/rs485_architecture.png)

## Descrição dos feeds

Este projeto utiliza três feeds OpenWrt personalizados. Eles são definidos em `feeds.conf.default` e instalados no sistema de compilação OpenWrt via `./scripts/feeds update && ./scripts/feeds install`.

### chirpstack

Integração do ecossistema ChirpStack LoRaWAN, incluindo o servidor de rede, daemon concentrador, packet forwarders e seus frontends LuCI.

| Pacote | Descrição |
|---------|-------------|
| `chirpstack` | Servidor de rede ChirpStack LoRaWAN |
| `chirpstack-concentratord` | Daemon packet forwarder do concentrador (com builds por hardware) |
| `chirpstack-mqtt-forwarder` | Packet forwarder baseado em MQTT (variantes single / slot1 / slot2 / mesh) |
| `chirpstack-udp-forwarder` | Packet forwarder baseado em UDP (variantes single / slot1 / slot2) |
| `chirpstack-gateway-mesh` | Extensão de rede mesh LoRaWAN |
| `chirpstack-rest-api` | Serviço de API REST para ChirpStack |
| `lorawan-devices` | Perfis de dispositivos LoRaWAN e definições de codecs |
| `node-red` | Plataforma de automação visual Node-RED |
| `libloragw-sx1301 / sx1302 / 2g4` | Bibliotecas HAL LoRa Semtech |
| `luci-app-chirpstack-*` | Interfaces Web LuCI para todos os componentes ChirpStack |
| `luci-theme-argon` | Tema Argon para LuCI |

### lorawan-gateway

Integração de hardware do gateway e serviços de sistema backend.

| Pacote | Descrição |
|---------|-------------|
| `lora` | Serviço de pilha de rádio LoRa (Rust) |
| `packetforwarder` | Packet forwarder LoRa |
| `chirpstack-concentratord-target-seeed-gateway` | Build de concentrador específico para Seeed gateway |
| `chirpstack-gateway-bridge` | Bridge de gateway ChirpStack (backend MQTT/UDP) |
| `basicstation_ubus` | Protocolo Basic Station com serviço RPC ubus |
| `lte-serve` | Serviço de gerenciamento de módulo celular LTE |
| `rs485-module` | Serviço de comunicação serial RS485 (Rust) |
| `rs485-modbus` | Implementação do protocolo Modbus RS485 (Rust) |
| `bacnet-stack` | Pilha de protocolo BACnet para automação predial |
| `ups-module` | Serviço de gerenciamento de energia UPS (Rust) |
| `hardware-info` | Leitor EEPROM para SN, EUI e informações de hardware do gateway |
| `ubus-serve` | Serviço RPC ubus para gerenciamento de sistema |
| `wifi-module` | Configuração WiFi automática via detecção de drive USB |

### luci-lorawan-gateway

Aplicativos de interface Web LuCI e tema para gerenciamento do gateway.

| Pacote | Descrição |
|---------|-------------|
| `luci-app-gateway` | Configuração principal do sistema de gateway |
| `luci-app-lora` | Status e configuração de rádio LoRa |
| `luci-app-chirpstack-concentratord-target-seeed-gateway` | Configuração do concentrador Seeed gateway |
| `luci-app-lte` | Configuração celular LTE/4G |
| `luci-app-multiwan` | Failover Multi-WAN e balanceamento de carga |
| `luci-app-routing` | Configuração de roteamento de rede |
| `luci-app-rs485` | Configuração de interface RS485/Modbus |
| `luci-app-bacnet` | Configuração de protocolo BACnet |
| `luci-app-ups` | Gerenciamento de energia UPS |
| `luci-app-ota` | Atualização OTA de firmware |
| `luci-app-terminal` | Console terminal baseado na Web |
| `luci-theme-sensecap` | Tema personalizado SenseCAP |

## FAQ

### Falha na compilação

**Problema**: Erros durante a compilação

**Solução**:
- Verificar espaço em disco (> 50GB recomendado)
- Garantir que os submódulos estão atualizados: `git submodule update --init --recursive`
- A compilação Rust é lenta, desative o download CI LLVM para acelerar

### Não é possível acessar após implantação

**Problema**: Não é possível acessar a interface Web após o início do contêiner

**Solução**:
- Verificar o status do contêiner LXC: `sudo lxc-ls -f`
- Visualizar logs do contêiner: `sudo lxc-info -n SenseCAP`
- Verificar se a configuração de rede está correta

### Dados LoRa não exibidos

**Problema**: Sem dados na página de status LoRa

**Solução**:
- Verificar o status do serviço concentrador
- Visualizar logs: `logread | grep -i lora`
- Verificar se a configuração do gateway está correta

## Links relacionados

- [OpenWrt](https://openwrt.org/)
- [ChirpStack](https://www.chirpstack.io/)
- [LuCI](https://github.com/openwrt/luci)
- [Seeed Studio](https://www.seeedstudio.com/)

## Licença

Este projeto segue os requisitos de licença do projeto OpenWrt.

## Contribuição

Issues e Pull Requests são bem-vindos!

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
