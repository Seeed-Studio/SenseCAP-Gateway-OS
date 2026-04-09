# SenseCAP Gateway OS

SenseCAP Gateway OS est un système d'exploitation de passerelle IoT multi-protocole conçu pour les scénarios industriels intérieurs. Basé sur OpenWrt, il fournit une base logicielle standardisée couvrant trois capacités clés : passerelle LoRaWAN, acquisition de données de bus industriel et rapports de conformité des bâtiments.

Le système adopte une architecture légère innovante combinant un hôte Debian avec un seul conteneur LXC (OpenWrt). Toute la logique métier est encapsulée et s'exécute dans un seul conteneur OpenWrt, tandis que le système hôte est uniquement responsable de l'abstraction matérielle et de la gestion des conteneurs. Cette conception permet une utilisation optimale des ressources, une forte isolation de sécurité et une flexibilité opérationnelle.

[![license][license-badge]][license]
[![prs][prs-badge]][prs]
[![issues][issues-badge]][issues]
[![release][release-badge]][release]
[![contact][contact-badge]][contact]

[English](README.md) | [中文](README_zh-CN.md) | [日本語](README_ja.md) | [Français](README_fr.md) | [Português](README_pt.md) | [Español](README_es.md)

## Table des matières

- [Caractéristiques](#caractéristiques)
- [Matériel recommandé](#matériel-recommandé)
- [Capacités et feuille de route](#capacités-et-feuille-de-route)
- [Structure des répertoires](#structure-des-répertoires)
- [Démarrage](#démarrage)
  - [Configuration requise](#configuration-requise)
  - [Installation des dépendances](#installation-des-dépendances)
  - [Étapes de compilation](#étapes-de-compilation)
  - [Personnalisation](#personnalisation)
- [Déploiement](#déploiement)
- [Modules fonctionnels](#modules-fonctionnels)
  - [Passerelle LoRaWAN](#passerelle-lorawan)
  - [Concentrateur ChirpStack](#concentrateur-chirpstack)
  - [Support LTE/WWAN](#support-ltewwan)
  - [Support Multi-WAN](#support-multi-wan)
- [Description des feeds](#description-des-feeds)
- [FAQ](#faq)
- [Liens associés](#liens-associés)
- [Licence](#licence)
- [Contribution](#contribution)

## Caractéristiques

- **Système hôte minimal** : L'hôte Debian ne conserve que le noyau, la chaîne d'outils LXC, les pilotes matériels et la surveillance UPS, sans exécuter de services applicatifs.
- **Architecture à conteneur unique** : Tous les services (services LoRaWAN, réseau, gestion des périphériques et services Web) s'exécutent dans un seul conteneur LXC en tant que packages OpenWrt natifs.
- **Opérations simplifiées** : La conception à conteneur unique simplifie la gestion de la configuration, le rollback des mises à jour et le dépannage, réduisant la complexité opérationnelle.

![Architecture des fonctionnalités](docs/images/features_architecture.png)

> **Note :** Le diagramme ci-dessus illustre uniquement les composants logiciels s'exécutant à l'intérieur du conteneur LXC (OpenWrt). La couche du système hôte (Debian) — incluant le noyau Linux, les pilotes matériels, le runtime LXC, le moniteur UPS et le stockage des données utilisateur — est affichée séparément en haut et ne fait pas partie de l'image du conteneur.

## Matériel recommandé

<p align="center">
  <img src="docs/images/recommended_hardware.png" width="700" />
</p>

| **Appareil** | **Lien** |
| --- | --- |
| reComputer R1225 Passerelle LoRaWAN & Contrôleur industriel (US915-4G) | [Acheter](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-4G-p-6721.html) |
| reComputer R1225 Passerelle LoRaWAN & Contrôleur industriel (US915) | [Acheter](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-US915-p-6722.html) |
| reComputer R1225 Passerelle LoRaWAN & Contrôleur industriel (EU868-4G) | [Acheter](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-4G-p-6719.html) |
| reComputer R1225 Passerelle LoRaWAN & Contrôleur industriel (EU868) | [Acheter](https://www.seeedstudio.com/reComputer-R1225-LoRaWAN-Gateway-Industrial-Controller-EU868-p-6720.html) |

Wiki du reComputer R1225 : <https://wiki.seeedstudio.com/r1225_introduction/>

SenseCAP Gateway OS n'est pas seulement le système dédié au R1225, mais aussi une solution logicielle de passerelle portable. Il peut être adapté à différentes plateformes matérielles, permettant aux partenaires de personnaliser et d'étendre rapidement le système.

## Capacités et feuille de route

![Feuille de route des capacités](docs/images/capabilities_roadmap.gif)

![Menus LuCI](docs/images/luci_menus.png)

🔜 **Prochaines étapes :**

- Ajout du support de configuration Web pour le protocole BACnet
- Optimisation de la logique de configuration Web pour les ports série et Modbus
- Ajout d'un service de watchdog réseau 4G

## Structure des répertoires

```
recomputer-gateway/
├── .config                    # Configuration de compilation OpenWrt
├── .github/
│   └── workflows/
│       └── build.yml          # Workflow de compilation GitHub Actions
├── feeds.conf.default         # Configuration des feeds
├── feeds/
│   ├── chirpstack/            # Packages liés à ChirpStack
│   ├── lorawan-gateway/       # Services backend de la passerelle LoRaWAN
│   └── luci-lorawan-gateway/  # Extensions de l'interface Web LuCI
│       ├── luci-app-gateway/      # Application de configuration principale
│       ├── luci-app-lora/         # Affichage du statut LoRa
│       ├── luci-app-lte/          # Configuration LTE
│       ├── luci-app-ups/          # Gestion de l'alimentation UPS
│       ├── luci-app-rs485/        # Configuration RS485
│       ├── luci-app-terminal/     # Terminal Web
│       ├── luci-app-ota/          # Mise à jour OTA
│       ├── luci-app-multiwan/     # Configuration Multi WAN
│       ├── luci-app-routing/      # Configuration du routage
│       └── luci-theme-sensecap/   # Thème SenseCap
├── openwrt/                   # Source OpenWrt (téléchargé lors de la compilation)
└── README.md                  # Ce document
```

## Démarrage

### Configuration requise

- **OS** : Ubuntu/Debian Linux
- **Espace disque** : > 50 Go recommandé
- **Mémoire** : > 8 Go recommandé

### Installation des dépendances

```bash
sudo apt-get update
sudo apt-get install build-essential clang flex bison g++ gawk \
  gcc-multilib g++-multilib gettext git libncurses5-dev \
  libssl-dev rsync unzip zlib1g-dev file wget
```

### Étapes de compilation

#### 1. Initialisation des sous-modules

```bash
git submodule update --init --recursive
```

#### 2. Cloner le source OpenWrt

```bash
git clone https://github.com/openwrt/openwrt.git -b openwrt-24.10
cd openwrt
rm -r feeds.conf.default
cp ../feeds.conf.default feeds.conf.default
```

#### 3. Mise à jour et installation des feeds

```bash
./scripts/feeds update -a
./scripts/feeds install -a
```

#### 4. Appliquer la configuration

```bash
cp ../.config .config
make defconfig
```

#### 5. (Optionnel) Désactiver le téléchargement CI LLVM de Rust pour accélérer la compilation

```bash
sed -i 's/--set=llvm.download-ci-llvm=true/--set=llvm.download-ci-llvm=false/' \
  feeds/packages/lang/rust/Makefile
```

#### 6. Compiler

```bash
unset CI GITHUB_ACTIONS CONTINUOUS_INTEGRATION
make -j$(nproc)
```

#### 7. Récupérer le résultat de compilation

Après la compilation, le firmware se trouve à :

```
openwrt/bin/targets/armsr/armv8/openwrt-armsr-armv8-generic-rootfs.tar.gz
```

### Personnalisation

Pour personnaliser le firmware (ajouter des packages, modifier les paramètres du noyau), exécutez menuconfig dans le répertoire openwrt :

```bash
cd openwrt
make menuconfig
```

## Déploiement

Le firmware est déployé sur l'appareil via un conteneur LXC :

### 1. Arrêter le conteneur existant

```bash
sudo lxc-stop -n SenseCAP
```

### 2. Nettoyer et créer un nouveau rootfs

```bash
sudo rm -rf /var/lib/lxc/SenseCAP/rootfs
sudo mkdir -p /var/lib/lxc/SenseCAP/rootfs
```

### 3. Extraire le nouveau firmware

```bash
sudo tar -xzf /path/to/openwrt-armsr-armv8-generic-rootfs.tar.gz \
  -C /var/lib/lxc/SenseCAP/rootfs
```

### 4. Démarrer le conteneur

```bash
sudo lxc-start -n SenseCAP
```

### 5. SSH vers le conteneur LXC

```bash
sudo lxc-attach -n SenseCAP
```

### 6. Afficher les logs

```bash
# Logs du packet forwarder LoRa
logread | grep lora

# Logs système
logread
```

### 7. Interface Web

Accédez à `http://[IP_ADDRESS]/cgi-bin/luci` pour :

- **Vue d'ensemble** : Statut LoRa, connexions réseau, statistiques de paquets
- **Services** : Configuration LoRa, réseau et autres

## Modules fonctionnels

### Passerelle LoRaWAN

- **Fichier de configuration** : `/etc/config/lora_pkt_fwd`
- **Service** : `lorawan_gateway`
- **UI** : Application LuCI Gateway

![Architecture LoRaWAN](docs/images/lorawan_architecture.png)

### Concentrateur ChirpStack

- **Cible** : `seeed-gateway`
- **Service** : `chirpstack-concentratord`

### Support LTE/WWAN

- **Configuration** : `/etc/config/network`
- **Pare-feu** : Règles de pare-feu ajoutées pour les réseaux LTE et WWAN

### Support Multi-WAN

Supporte plusieurs configurations WAN incluant LTE et Ethernet, avec équilibrage de charge et capacités de basculement.

#### Architecture des interfaces réseau

Le reComputer R1225 est équipé de **deux ports Ethernet physiques** (ETH0 et ETH1). Ces ports remplissent des rôles différents selon l'architecture hôte-conteneur :

| Port | Rôle | Description |
|------|------|-------------|
| **ETH0** | Interface conteneur (LXC) | Cette interface est **directement mappée (passthrough) du matériel vers le conteneur LXC** via la configuration réseau LXC de l'hôte. Le conteneur OpenWrt a un contrôle total sur cette interface, la gérant comme un port WAN ou LAN standard. Tout le trafic applicatif (uplink LoRaWAN, MQTT, accès Web UI, etc.) passe par ce port. |
| **ETH1** | Interface hôte (Debian) | Cette interface est **gérée par le système hôte Debian**. Elle est utilisée pour les tâches d'administration au niveau de l'hôte telles que l'accès SSH, les opérations de gestion des conteneurs, les mises à jour du firmware et la communication de surveillance UPS. Elle reste isolée de la pile réseau du conteneur. |

Cette séparation garantit que même si le réseau du conteneur est mal configuré ou inaccessible, l'interface de gestion de l'hôte reste accessible pour la récupération et la maintenance.

### RS485 / Modbus

- **Fichiers de configuration** : `/etc/config/rs485-module` (série, MQTT, protocole)
- **Services** : `rs485-module`, `rs485-modbus`
- **UI** : Application LuCI RS485 (Paramètres série, Paramètres de protocole, Paramètres MQTT, Journal MQTT)

Le module RS485 supporte les protocoles industriels incluant **Modbus RTU** et **BACnet MS/TP** :

- **Modbus RTU** : Interrogation et analyse des registres Modbus via l'interface série RS485, avec transfert de données via uplink/downlink MQTT.
- **BACnet MS/TP** : Support du protocole BACnet sur RS485 pour l'intégration de l'automatisation des bâtiments (configuration Web en cours).

![Architecture RS485](docs/images/rs485_architecture.png)

## Description des feeds

Ce projet utilise trois feeds OpenWrt personnalisés. Ils sont définis dans `feeds.conf.default` et installés dans le système de compilation OpenWrt via `./scripts/feeds update && ./scripts/feeds install`.

### chirpstack

Intégration de l'écosystème ChirpStack LoRaWAN, incluant le serveur réseau, le démon concentrateur, les packet forwarders et leurs frontends LuCI.

| Package | Description |
|---------|-------------|
| `chirpstack` | Serveur réseau ChirpStack LoRaWAN |
| `chirpstack-concentratord` | Démon packet forwarder du concentrateur (avec builds ciblés par matériel) |
| `chirpstack-mqtt-forwarder` | Packet forwarder basé sur MQTT (variantes single / slot1 / slot2 / mesh) |
| `chirpstack-udp-forwarder` | Packet forwarder basé sur UDP (variantes single / slot1 / slot2) |
| `chirpstack-gateway-mesh` | Extension de réseau mesh LoRaWAN |
| `chirpstack-rest-api` | Service API REST pour ChirpStack |
| `lorawan-devices` | Profils de dispositifs LoRaWAN et définitions de codecs |
| `node-red` | Plateforme d'automatisation visuelle Node-RED |
| `libloragw-sx1301 / sx1302 / 2g4` | Bibliothèques HAL LoRa Semtech |
| `luci-app-chirpstack-*` | Interfaces Web LuCI pour tous les composants ChirpStack |
| `luci-theme-argon` | Thème Argon pour LuCI |

### lorawan-gateway

Intégration matérielle de la passerelle et services système backend.

| Package | Description |
|---------|-------------|
| `lora` | Service de pile radio LoRa (Rust) |
| `packetforwarder` | Packet forwarder LoRa |
| `chirpstack-concentratord-target-seeed-gateway` | Build concentrateur spécifique Seeed gateway |
| `chirpstack-gateway-bridge` | Bridge passerelle ChirpStack (backend MQTT/UDP) |
| `basicstation_ubus` | Protocole Basic Station avec service RPC ubus |
| `lte-serve` | Service de gestion du module cellulaire LTE |
| `rs485-module` | Service de communication série RS485 (Rust) |
| `rs485-modbus` | Implémentation du protocole Modbus RS485 (Rust) |
| `bacnet-stack` | Pile protocole BACnet pour l'automatisation des bâtiments |
| `ups-module` | Service de gestion de l'alimentation UPS (Rust) |
| `hardware-info` | Lecteur EEPROM pour SN, EUI et infos matérielles de la passerelle |
| `ubus-serve` | Service RPC ubus pour la gestion système |
| `wifi-module` | Configuration WiFi automatique via détection de clé USB |

### luci-lorawan-gateway

Applications d'interface Web LuCI et thème pour la gestion de la passerelle.

| Package | Description |
|---------|-------------|
| `luci-app-gateway` | Configuration principale du système de passerelle |
| `luci-app-lora` | Statut et configuration radio LoRa |
| `luci-app-chirpstack-concentratord-target-seeed-gateway` | Configuration du concentrateur Seeed gateway |
| `luci-app-lte` | Configuration cellulaire LTE/4G |
| `luci-app-multiwan` | Basculement Multi-WAN et équilibrage de charge |
| `luci-app-routing` | Configuration du routage réseau |
| `luci-app-rs485` | Configuration de l'interface RS485/Modbus |
| `luci-app-bacnet` | Configuration du protocole BACnet |
| `luci-app-ups` | Gestion de l'alimentation UPS |
| `luci-app-ota` | Mise à jour OTA du firmware |
| `luci-app-terminal` | Console terminal basée sur le Web |
| `luci-theme-sensecap` | Thème personnalisé SenseCAP |

## FAQ

### Échec de la compilation

**Problème** : Erreurs pendant la compilation

**Solution** :
- Vérifier l'espace disque (> 50 Go recommandé)
- S'assurer que les sous-modules sont à jour : `git submodule update --init --recursive`
- La compilation Rust est lente, désactiver le téléchargement CI LLVM pour accélérer

### Impossible d'accéder après le déploiement

**Problème** : Impossible d'accéder à l'interface Web après le démarrage du conteneur

**Solution** :
- Vérifier le statut du conteneur LXC : `sudo lxc-ls -f`
- Afficher les logs du conteneur : `sudo lxc-info -n SenseCAP`
- Vérifier que la configuration réseau est correcte

### Données LoRa non affichées

**Problème** : Pas de données sur la page de statut LoRa

**Solution** :
- Vérifier le statut du service concentrateur
- Afficher les logs : `logread | grep -i lora`
- Vérifier que la configuration de la passerelle est correcte

## Liens associés

- [OpenWrt](https://openwrt.org/)
- [ChirpStack](https://www.chirpstack.io/)
- [LuCI](https://github.com/openwrt/luci)
- [Seeed Studio](https://www.seeedstudio.com/)

## Licence

Ce projet suit les exigences de licence du projet OpenWrt.

## Contribution

Les Issues et Pull Requests sont les bienvenus !

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
