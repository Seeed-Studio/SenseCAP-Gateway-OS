#!/bin/sh

MEDIA_DIR="/media"
CONFIG_FILE="WLAN.txt"
WIRELESS_CONFIG="/etc/config/wireless"
SN_FILE="/etc/deviceinfo/sn"
FREQ_PLAN_FILE="/etc/deviceinfo/freq_plan"
DEFAULT_SSID="R1225-0000"

get_wifi_country() {
    case "$(cat "$FREQ_PLAN_FILE" 2>/dev/null)" in
        US915) echo "US" ;;
        EU868) echo "FR" ;;
        CN470) echo "CN" ;;
        *)     echo "" ;;
    esac
}

get_default_ssid() {
    local sn=""
    local ssid="R1225-0000"
    
    if [ -f "$SN_FILE" ]; then
        sn=$(cat "$SN_FILE" | tr -d '[:space:]')
        if [ ${#sn} -ge 4 ]; then
            local last_four=${sn: -4}
            ssid="R1225-${last_four}"
        fi
    fi
    
    echo "$ssid"
}

LAST_CONFIG_MD5=""
LAST_INTERFACES=""
NO_CLIENT_COUNTER=0
NO_CLIENT_THRESHOLD=300

log_message() {
    logger -t wifi-module "$1"
}

apply_wifi_config() {
    local config_path="$1"
    
    log_message "Found WLAN.txt at: $config_path"
    
    local mode=""
    local ap_ssid=""
    local ap_password=""
    local sta_ssid=""
    local sta_password=""
    
    while IFS='=' read -r key value; do
        key=$(echo "$key" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
        value=$(echo "$value" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
        
        case "$key" in
            MODE|mode)
                mode="$value"
                ;;
            AP_SSID|ap_ssid)
                ap_ssid="$value"
                ;;
            AP_PASSWORD|ap_password)
                ap_password="$value"
                ;;
            STA_SSID|sta_ssid)
                sta_ssid="$value"
                ;;
            STA_PASSWORD|sta_password)
                sta_password="$value"
                ;;
        esac
    done < "$config_path"
    
    if [ -z "$mode" ]; then
        log_message "ERROR: MODE not specified in WLAN.txt"
        return 1
    fi
    
    log_message "Applying WiFi configuration: MODE=$mode"
    
    if [ "$mode" = "ap" ] || [ "$mode" = "AP" ]; then
        if [ -z "$ap_ssid" ]; then
            log_message "ERROR: AP_SSID not specified for AP mode"
            return 1
        fi

        # Remove all existing wifi-iface sections
        uci show wireless | grep "\.iface=" | while read -r line; do
            local section=$(echo "$line" | cut -d'.' -f2 | cut -d'=' -f1)
            uci -q delete "wireless.${section}"
        done

        uci set wireless.radio0.disabled='0'
        uci set wireless.radio0.band='2g'
        uci set wireless.radio0.channel='6'
        uci -q delete wireless.radio0.htmode

        uci set wireless.default_radio0='wifi-iface'
        uci set wireless.default_radio0.device='radio0'
        uci set wireless.default_radio0.network='wlan'
        uci set wireless.default_radio0.mode='ap'
        uci set wireless.default_radio0.ifname='wlan0'
        uci set wireless.default_radio0.ssid="$ap_ssid"

        if [ -n "$ap_password" ] && [ ${#ap_password} -ge 8 ]; then
            uci set wireless.default_radio0.encryption='psk2'
            uci set wireless.default_radio0.key="$ap_password"
        else
            uci set wireless.default_radio0.encryption='none'
        fi

        uci commit wireless
        log_message "Configured as AP: SSID=$ap_ssid"
        
    elif [ "$mode" = "sta" ] || [ "$mode" = "STA" ]; then
        if [ -z "$sta_ssid" ]; then
            log_message "ERROR: STA_SSID not specified for STA mode"
            return 1
        fi

        # Remove all existing wifi-iface sections
        uci show wireless | grep "\.iface=" | while read -r line; do
            local section=$(echo "$line" | cut -d'.' -f2 | cut -d'=' -f1)
            uci -q delete "wireless.${section}"
        done

        uci set wireless.radio0.disabled='0'
        uci set wireless.radio0.band='2g'
        uci set wireless.radio0.htmode='HT20'
        uci set wireless.radio0.channel='auto'
        # Keep the user's country if configured; otherwise default it from
        # the EEPROM freq plan instead of hardcoding CN.
        uci -q get wireless.radio0.country >/dev/null || {
            wifi_country=$(get_wifi_country)
            [ -n "$wifi_country" ] && uci set wireless.radio0.country="$wifi_country"
        }

        uci set wireless.default_radio0='wifi-iface'
        uci set wireless.default_radio0.device='radio0'
        uci set wireless.default_radio0.network='wwan'
        uci set wireless.default_radio0.mode='sta'
        uci set wireless.default_radio0.ifname='wlan0'
        uci set wireless.default_radio0.ssid="$sta_ssid"

        if [ -n "$sta_password" ]; then
            uci set wireless.default_radio0.encryption='psk2'
            uci set wireless.default_radio0.key="$sta_password"
        else
            uci set wireless.default_radio0.encryption='none'
        fi

        uci commit wireless
        log_message "Configured as STA: SSID=$sta_ssid"
        
    else
        log_message "ERROR: Invalid MODE: $mode (must be 'ap' or 'sta')"
        return 1
    fi
    
    wifi reload
    log_message "WiFi configuration applied and reloaded"
    
    return 0
}

check_usb_mounts() {
    [ -d "$MEDIA_DIR" ] || return
    
    local config_file=$(find "$MEDIA_DIR" -name "$CONFIG_FILE" -type f 2>/dev/null | head -n 1)
    
    if [ -n "$config_file" ] && [ -f "$config_file" ]; then
        local current_md5=$(md5sum "$config_file" 2>/dev/null | awk '{print $1}')
        
        if [ "$current_md5" != "$LAST_CONFIG_MD5" ]; then
            apply_wifi_config "$config_file"
            if [ $? -eq 0 ]; then
                LAST_CONFIG_MD5="$current_md5"
            fi
        fi
    fi
}

check_new_interfaces() {
    # wwan* comes and goes with modem re-enumeration; not a reason to restart WiFi
    local current_interfaces=$(ip -o link show | awk -F': ' '{print $2}' | grep -v '^wwan' | sort | tr '\n' ' ')
    
    if [ -n "$LAST_INTERFACES" ] && [ "$current_interfaces" != "$LAST_INTERFACES" ]; then
        log_message "Network interface change detected, restarting WiFi..."
        wifi reload && wifi down radio0 && wifi up radio0
        log_message "WiFi restarted"
    fi
    
    LAST_INTERFACES="$current_interfaces"
}

check_r1225_hotspot() {
    local default_ssid=$(get_default_ssid)
    local r1225_section=""
    local section_list=$(uci show wireless | grep "\.ssid=" | grep "^wireless.*\.ssid='${default_ssid}'$")
    
    if [ -z "$section_list" ]; then
        NO_CLIENT_COUNTER=0
        return
    fi
    
    r1225_section=$(echo "$section_list" | head -1 | cut -d'.' -f2 | cut -d'=' -f1)
    
    if [ -z "$r1225_section" ]; then
        NO_CLIENT_COUNTER=0
        return
    fi
    
    local is_disabled=$(uci -q get wireless.$r1225_section.disabled 2>/dev/null)
    
    if [ "$is_disabled" = "1" ]; then
        NO_CLIENT_COUNTER=0
        return
    fi
    
    
    local ap_interface=$(iw dev 2>/dev/null | awk '/Interface/{iface=$2} /type AP/{print iface; exit}')
    
    if [ -z "$ap_interface" ]; then
        NO_CLIENT_COUNTER=0
        return
    fi
    local has_clients=$(iw dev "$ap_interface" station dump 2>/dev/null | grep -c "^Station")
    
    if [ "$has_clients" -eq 0 ]; then
        NO_CLIENT_COUNTER=$((NO_CLIENT_COUNTER + 1))
        
        if [ $NO_CLIENT_COUNTER -ge $NO_CLIENT_THRESHOLD ]; then
            log_message "Default hotspot ($default_ssid) has no clients for 5 minutes, disabling..."
            uci set wireless.$r1225_section.disabled='1'
            uci commit wireless
            wifi reload
            log_message "Default hotspot disabled (section: $r1225_section, SSID: $default_ssid)"
            NO_CLIENT_COUNTER=0
        fi
    else
        NO_CLIENT_COUNTER=0
    fi
}

init_r1225_hotspot() {
    local default_ssid=$(get_default_ssid)
    log_message "Checking R1225 hotspot initialization for SSID: $default_ssid..."
    
    local r1225_section=""
    local section_list=$(uci show wireless | grep "\.ssid=" | grep "^wireless.*\.ssid='${default_ssid}'$")
    
    if [ -z "$section_list" ]; then
        log_message "No default hotspot ($default_ssid) found, skipping initialization"
        return
    fi
    
    r1225_section=$(echo "$section_list" | head -1 | cut -d'.' -f2 | cut -d'=' -f1)
    
    if [ -z "$r1225_section" ]; then
        return
    fi
    local r1225_disabled=$(uci -q get wireless.$r1225_section.disabled 2>/dev/null)
    
    if [ "$r1225_disabled" != "1" ]; then
        log_message "Default hotspot ($default_ssid) is already enabled"
        return
    fi
    
    log_message "Default hotspot ($default_ssid) is disabled, checking for other active WiFi..."
    
    # Pipe runs the loop in a subshell; capture the result via stdout instead
    local has_active_wifi

    has_active_wifi=$(uci show wireless | grep "^wireless\." | while read -r cfg; do
        local section=$(echo "$cfg" | cut -d'.' -f2 | cut -d'=' -f1)
        local is_iface=$(uci -q get wireless.$section 2>/dev/null | grep "wifi-iface")

        if [ -n "$is_iface" ] && [ "$section" != "$r1225_section" ]; then
            local disabled=$(uci -q get wireless.$section.disabled 2>/dev/null)
            if [ "$disabled" != "1" ]; then
                echo 1
                break
            fi
        fi
    done)

    if [ -z "$has_active_wifi" ]; then
        log_message "No other active WiFi found, enabling default hotspot ($default_ssid)..."
        uci set wireless.$r1225_section.disabled='0'
        uci commit wireless
        wifi reload
        log_message "Default hotspot enabled (section: $r1225_section, SSID: $default_ssid)"
    else
        log_message "Other active WiFi detected, keeping default hotspot disabled"
    fi
}

log_message "WiFi Module Monitor started"

init_r1225_hotspot

while true; do
    check_new_interfaces
    check_usb_mounts
    check_r1225_hotspot
    sleep 1
done
