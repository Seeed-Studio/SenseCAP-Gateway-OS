#!/bin/sh

# LTE Module Initialization & Info Caching Script
# Handles GPIO control, AT command sending, and LTE info caching

LOG_TAG="[LTE-Serve]"
AT_RETRY_INTERVAL=180
LTE_INFO_CACHE="/var/run/lte-info.json"
LTE_INFO_UPDATE_INTERVAL=10

# Load JSON library
. /usr/share/libubox/jshn.sh

log() {
    logger -t "$LOG_TAG" "$1"
}

# Send AT command to 4G module
send_at_command() {
    local cmd="$1"
    local device="$2"

    if [ -e "$device" ]; then
        log "Sending AT command: $cmd to $device"
        echo -e "${cmd}\r" > "$device" 2>/dev/null
        return 0
    else
        return 1
    fi
}

# Check if wwan0 interface exists
wwan0_exists() {
    ip link show wwan0 &>/dev/null
}

# Get LTE info and cache it
update_lte_info_cache() {
    local lte_device imei iccid rssi lte_status is_connected

    # Get LTE device from UCI config
    lte_device=$(uci -q get network.LTE.device)
    [ -z "$lte_device" ] && lte_device="/dev/cdc-wdm0"

    # Check if device is accessible
    if [ ! -e "$lte_device" ]; then
        echo '{"connected":false}' > "$LTE_INFO_CACHE"
        return
    fi

    # Initialize JSON
    json_init

    # Check LTE connection status via ubus
    lte_status=$(ubus -S call network.interface.LTE status 2>/dev/null)
    is_connected="false"

    if [ -n "$lte_status" ]; then
        if echo "$lte_status" | jsonfilter -e '@.up' | grep -q true; then
            is_connected="true"
        fi
    fi

    # Fallback to operstate check
    if [ "$is_connected" = "false" ]; then
        operstate=$(cat /sys/class/net/wwan0/operstate 2>/dev/null)
        if [ "$operstate" = "up" ] || [ "$operstate" = "unknown" ]; then
            is_connected="true"
        fi
    fi

    # Add connected status
    json_add_string connected "$is_connected"

    # Get IMEI
    imei=$(uqmi -s -d "$lte_device" --get-imei 2>/dev/null | tr -d '"\n')
    if [ -n "$imei" ] && [ "$imei" != "Not supported" ] && [ "$imei" != "not supported" ]; then
        json_add_string imei "$imei"
    fi

    # Get ICCID
    iccid=$(uqmi -s -d "$lte_device" --get-iccid 2>/dev/null | tr -d '"\n')
    if [ -n "$iccid" ] && [ "$iccid" != "Not supported" ] && [ "$iccid" != "not supported" ]; then
        json_add_string iccid "$iccid"
    fi

    # Get RSSI
    rssi=$(uqmi -s -d "$lte_device" --get-signal-info 2>/dev/null | jsonfilter -e '@.rssi' 2>/dev/null)
    if [ -n "$rssi" ]; then
        json_add_string rssi "$rssi"
    fi

    # Add timestamp
    json_add_int timestamp $(date +%s)

    # Write to cache file
    json_dump > "$LTE_INFO_CACHE"
}

# Main loop
main() {
    log "Starting LTE Module Initialization Service"

    # Load GPIO configuration
    LTE_RST_CHIP=$(uci get hardware.hardware.lte_rst_chip)
    LTE_RST_LINE=$(uci get hardware.hardware.lte_rst_line)
    LTE_USB_PORT=$(uci get hardware.hardware.lte_usb_port)

    # Check and set GPIO if low
    GPIO_VALUE=$(gpioget -c "$LTE_RST_CHIP" "$LTE_RST_LINE" 2>/dev/null)
    if ! echo "$GPIO_VALUE" | grep -q "=active"; then
        log "LTE RST is LOW, setting to HIGH"
        gpioset -z -c "$LTE_RST_CHIP" "${LTE_RST_LINE}=1" 2>/dev/null
    fi

    local last_at_time=0
    local last_lte_info_update=0

    # Main initialization loop
    while true; do
        current_time=$(date +%s)

        # Update LTE info cache at intervals
        if [ $((current_time - last_lte_info_update)) -ge $LTE_INFO_UPDATE_INTERVAL ]; then
            update_lte_info_cache
            last_lte_info_update=$current_time
        fi

        # Check USB port and wwan0
        if [ -e "$LTE_USB_PORT" ]; then
            if ! wwan0_exists; then
                if [ $((current_time - last_at_time)) -ge $AT_RETRY_INTERVAL ]; then
                    log "wwan0 not found, sending AT commands to initialize LTE module"
                    send_at_command 'AT+QCFG="usbnet",0' "$LTE_USB_PORT"
                    sleep 2
                    send_at_command 'AT+CFUN=1,1' "$LTE_USB_PORT"
                    last_at_time=$current_time
                fi
            fi
        fi

        sleep 10
    done
}

# Run main loop
main
