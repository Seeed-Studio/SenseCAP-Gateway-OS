# WiFi Module - Automatic Configuration

This package enables automatic WiFi configuration of OpenWrt devices via USB flash drive.

## Architecture

| File | Purpose |
|------|---------|
| `wifi-module.init` | OpenWrt init service, starts the monitor script |
| `wifi-module.sh` | Main monitor script, runs continuously to detect configuration changes |
| `etc/uci-defaults/66_wifi_init` | One-time WiFi initialization on first boot |

## Workflow

```
┌─────────────────────────────────────────────────────────┐
│                    Main Loop (every 1 second)            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────────┐  ┌──────────────────┐           │
│  │ check_usb_mounts │  │check_r1225_hotspot│           │
│  │  Detect WLAN.txt │  │  Default hotspot  │           │
│  └──────────────────┘  │     management    │           │
│            │           └──────────────────┘           │
│            ▼                      │                     │
│     File changed?          5 min no clients?           │
│     (MD5 check)            (auto-disable)              │
│            │                      │                     │
│            ▼                      ▼                     │
│    Apply new config      Disable default hotspot       │
│    to UCI & reload                                  │
└─────────────────────────────────────────────────────────┘
```

## First Boot Initialization

On system first boot, `66_wifi_init` executes once:

1. Clear `/etc/config/wireless`
2. Read `/etc/deviceinfo/sn` for device serial number
3. Generate default SSID: `R1225-{last 4 digits of SN}`
4. Create default AP hotspot:
   - Band: 2.4GHz
   - Channel: 6
   - Password: `1234567890`

## USB Configuration via WLAN.txt

The monitor script checks `/media` directory for `WLAN.txt` every second.

### Configuration File Format

Create a file named `WLAN.txt` in the root directory of the USB drive:

#### AP Mode (Creating a Hotspot)

```ini
MODE=ap
AP_SSID=MyHotspot
AP_PASSWORD=12345678
```

**Applied UCI settings:**
```bash
uci set wireless.radio0.disabled='0'
uci set wireless.radio0.band='2g'
uci set wireless.radio0.channel='6'
uci set wireless.default_radio0.mode='ap'
uci set wireless.default_radio0.network='wlan'
```

#### STA Mode (Connect to WiFi)

```ini
MODE=sta
STA_SSID=HomeWiFi
STA_PASSWORD=mypassword
```

**Applied UCI settings:**
```bash
uci set wireless.radio0.disabled='0'
uci set wireless.radio0.band='2g'
uci set wireless.radio0.channel='auto'
uci set wireless.radio0.htmode='HT20'
# country: kept if already configured, else defaulted from EEPROM freq plan
uci set wireless.default_radio0.mode='sta'
uci set wireless.default_radio0.network='wwan'
```

### Wi-Fi Country

The country is never overwritten once configured. When
`wireless.radio0.country` is unset, STA setup defaults it from
`/etc/deviceinfo/freq_plan` (US915 -> US, EU868 -> FR, CN470 -> CN).
Changes made in LuCI always win.

### Configuration Parameters

| Parameter | AP Mode | STA Mode | Required |
|-----------|---------|----------|----------|
| `MODE` | `ap` | `sta` | Yes |
| `AP_SSID` | Hotspot name | - | Required for AP |
| `AP_PASSWORD` | Hotspot password (min 8 chars) | - | Optional |
| `STA_SSID` | - | Target network name | Required for STA |
| `STA_PASSWORD` | - | Network password | Optional |

**Note:** If password is omitted or less than 8 characters, encryption is set to `none`.

## Smart Default Hotspot Management

The module includes intelligent power-saving features:

| Scenario | Behavior |
|----------|----------|
| No other WiFi on boot | Auto-enable default hotspot |
| Default hotspot idle for 5 min | Auto-disable to save power |
| WLAN.txt detected | Apply configuration and switch mode |

### Functions

- `init_r1225_hotspot()`: Enables default hotspot if no other active WiFi exists
- `check_r1225_hotspot()`: Monitors client activity, disables after 5 minutes idle

## Change Detection Mechanisms

### MD5 Deduplication
Prevents reapplying identical configurations by comparing file MD5 hash:

```bash
if [ "$current_md5" != "$LAST_CONFIG_MD5" ]; then
    apply_wifi_config "$config_file"
fi
```

### Network Interface Detection
`check_new_interfaces()` detects network interface changes and restarts WiFi:

```bash
local current_interfaces=$(ip -o link show | awk -F': ' '{print $2}' | sort)
if [ "$current_interfaces" != "$LAST_INTERFACES" ]; then
    wifi reload && wifi down radio0 && wifi up radio0
fi
```

## Service Management

```bash
# Start service
/etc/init.d/wifi-module start

# Stop service
/etc/init.d/wifi-module stop

# Enable on boot
/etc/init.d/wifi-module enable

# View logs
logread | grep wifi-module
```

## Bug Fixes

### AP to AP Reconfiguration Issue

**Problem**: When configuring AP mode via USB for the second time (changing SSID or password), the configuration would not apply correctly. A workaround was to first switch to STA mode, then back to AP.

**Root Cause**: The original code only deleted fixed-named sections (`default_radio0` and `wifinet0`), leaving residual UCI configuration that prevented proper updates.

**Solution**: Before applying new configuration, the script now removes all existing `wifi-iface` sections:

```bash
uci show wireless | grep "\.iface=" | while read -r line; do
    local section=$(echo "$line" | cut -d'.' -f2 | cut -d'=' -f1)
    uci -q delete "wireless.${section}"
done
```

This ensures a clean state before creating new WiFi configuration, allowing seamless AP-to-AP reconfiguration.

## Default SSID Generation

The SSID is generated from the device serial number stored in `/etc/deviceinfo/sn`:

```bash
get_default_ssid() {
    local sn=$(cat /etc/deviceinfo/sn | tr -d '[:space:]')
    local last_four=${sn: -4}
    echo "R1225-${last_four}"
}
```

If SN file doesn't exist, it defaults to `R1225-0000`.
