#!/bin/sh

# LTE Module Initialization & Info Caching Script
# Handles GPIO control, AT command sending, and LTE info caching
#
# Flow:
# 1. GPIO reset LTE module
# 2. Wait for USB2 port, get IMEI and ICCID via AT commands
# 3. Check wwan0, configure with AT commands if not present
# 4. Set lte_disable=0 to allow system dialing
# 5. Loop to read RSSI

LOG_TAG="[LTE-Serve]"
LTE_INFO_CACHE="/var/run/lte-info.json"
LTE_INFO_UPDATE_INTERVAL=60

# if $1 is not empty, set LTE_INFO_CACHE to $1
if [ -n "$1" ]; then
    LTE_INFO_CACHE="$1"
fi
# Load JSON library
. /usr/share/libubox/jshn.sh

# Caches for static info
CACHED_IMEI=""
CACHED_ICCID=""

log() {
    logger -t "$LOG_TAG" "$1"
}

# Send AT command and get response (using temp file)
send_at_and_read() {
    local cmd="$1"
    local device="$2"
    local wait_time="${3:-2}"
    local resp_file="/tmp/at_resp_$$.txt"

    if [ ! -e "$device" ]; then
        return 1
    fi

    # Clear buffer
    cat "$device" > /dev/null 2>&1 &
    CLEAR_PID=$!
    sleep 1
    kill $CLEAR_PID 2>/dev/null
    wait $CLEAR_PID 2>/dev/null

    # Send AT command
    echo -e "${cmd}\r" > "$device" 2>/dev/null

    # Read response to file with timeout
    cat "$device" > "$resp_file" 2>/dev/null &
    CAT_PID=$!
    sleep $wait_time
    kill $CAT_PID 2>/dev/null
    wait $CAT_PID 2>/dev/null

    # Return response
    if [ -s "$resp_file" ]; then
        cat "$resp_file"
        rm -f "$resp_file"
    else
        rm -f "$resp_file"
        return 1
    fi
}

# Get IMEI via AT command
get_imei_at() {
    local device="$1"
    local response

    response=$(send_at_and_read "AT+GSN" "$device" 2)
    echo "$response" | grep -oE '[0-9]{15}' | head -1
}

# Get ICCID via AT command
get_iccid_at() {
    local device="$1"
    local response

    response=$(send_at_and_read "AT+QCCID" "$device" 2)
    echo "$response" | grep -oE '[0-9]{19,20}' | head -1
}

# Check if wwan0 interface exists
wwan0_exists() {
    ip link show wwan0 &>/dev/null
}

# GPIO reset LTE module
reset_lte_module() {
    log "Resetting LTE module via GPIO..."

    # Set GPIO low to reset (use timeout to avoid blocking)
    timeout 1 gpioset -c "$LTE_RST_CHIP" "${LTE_RST_LINE}=0" 2>/dev/null || true

    # Set GPIO high to enable
    timeout 1 gpioset -c "$LTE_RST_CHIP" "${LTE_RST_LINE}=1" 2>/dev/null || true
    log "LTE module GPIO reset completed"
}

# Wait for USB port to appear
wait_for_usb_port() {
    local max_wait=60
    local count=0

    log "Waiting for $LTE_USB_PORT to appear..."
    while [ ! -e "$LTE_USB_PORT" ] && [ $count -lt $max_wait ]; do
        sleep 1
        count=$((count + 1))
    done

    if [ -e "$LTE_USB_PORT" ]; then
        log "USB port $LTE_USB_PORT is ready"
        return 0
    else
        log "Timeout waiting for $LTE_USB_PORT"
        return 1
    fi
}

# Get IMEI and ICCID via AT commands after reset
get_lte_info_at() {
    if [ -e "$LTE_USB_PORT" ]; then
        log "Getting IMEI and ICCID via AT commands..."

        # Get ICCID
        if [ -z "$CACHED_ICCID" ]; then
            CACHED_ICCID=$(get_iccid_at "$LTE_USB_PORT")
            [ -n "$CACHED_ICCID" ] && log "Got ICCID: $CACHED_ICCID"
        fi

        # Get IMEI
        if [ -z "$CACHED_IMEI" ]; then
            CACHED_IMEI=$(get_imei_at "$LTE_USB_PORT")
            [ -n "$CACHED_IMEI" ] && log "Got IMEI: $CACHED_IMEI"
        fi
    fi
}

# Configure wwan0 via AT commands if not present
configure_wwan0() {
    local max_wait=30
    local count=0

    # Wait for wwan0 to appear
    while ! wwan0_exists && [ $count -lt $max_wait ]; do
        log "wwan0 not found, configuring LTE module..."
        send_at_and_read 'AT+QCFG="usbnet",0' "$LTE_USB_PORT" 2
        send_at_and_read 'AT+CFUN=1,1' "$LTE_USB_PORT" 2
        log "AT commands sent, waiting for module to re-enumerate..."

        # Wait for re-enumeration
        count=0
        while ! wwan0_exists && [ $count -lt $max_wait ]; do
            sleep 1
            count=$((count + 1))
        done

        if wwan0_exists; then
            log "wwan0 interface is now available"
            return 0
        fi
    done

    if wwan0_exists; then
        return 0
    else
        log "Warning: wwan0 still not available after configuration"
        return 1
    fi
}

# Enable LTE by setting disabled=0
enable_lte_dialing() {
    local current_disabled
    current_disabled=$(uci -q get network.LTE.disabled)

    if [ "$current_disabled" != "0" ]; then
        log "Enabling LTE dialing (setting disabled=0)"
        uci set network.LTE.disabled='0'
        uci commit network
    fi
}

# Get LTE info and cache it
update_lte_info_cache() {
    local lte_device rssi lte_status is_connected

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

    # Get IMEI via uqmi if not cached
    if [ -z "$CACHED_IMEI" ]; then
        CACHED_IMEI=$(uqmi -s -t 500 -d "$lte_device" --get-imei 2>/dev/null | tr -d '"\n')
        [ -n "$CACHED_IMEI" ] && log "Got IMEI: $CACHED_IMEI"
    fi
    if [ -n "$CACHED_IMEI" ]; then
        json_add_string imei "$CACHED_IMEI"
    fi

    # Get ICCID via uqmi if not cached (may not be supported)
    if [ -z "$CACHED_ICCID" ]; then
        CACHED_ICCID=$(uqmi -s -t 500 -d "$lte_device" --get-iccid 2>/dev/null | tr -d '"\n')
        # Check if valid (not "Not supported")
        if [ -z "$CACHED_ICCID" ] || [ "$CACHED_ICCID" = "Not supported" ] || [ "$CACHED_ICCID" = "not supported" ]; then
            CACHED_ICCID=""
        else
            log "Got ICCID: $CACHED_ICCID"
        fi
    fi
    if [ -n "$CACHED_ICCID" ]; then
        json_add_string iccid "$CACHED_ICCID"
    fi

    # Get RSSI via uqmi
    rssi=$(uqmi -s -t 500 -d "$lte_device" --get-signal-info 2>/dev/null | jsonfilter -e '@.rssi' 2>/dev/null)
    if [ -n "$rssi" ]; then
        json_add_string rssi "$rssi"
    fi

    # Add timestamp
    json_add_int timestamp $(date +%s)

    # Write to cache file
    json_dump > "$LTE_INFO_CACHE"
}

# Initialize LTE module
init_lte_module() {
    log "Starting LTE module initialization..."

    # Load GPIO configuration
    LTE_RST_CHIP=$(uci get hardware.hardware.lte_rst_chip)
    LTE_RST_LINE=$(uci get hardware.hardware.lte_rst_line)
    LTE_USB_PORT=$(uci get hardware.hardware.lte_usb_port)

    # Step 1: GPIO reset LTE module
    reset_lte_module

    # Step 2: Wait for USB port and get IMEI/ICCID
    if wait_for_usb_port; then
        sleep 3  # Wait for module to stabilize
        get_lte_info_at
    fi

    # Step 3: Check and configure wwan0 (waits for interface to appear)
    configure_wwan0

    # Step 4: Enable LTE dialing (set lte_disable=0)
    enable_lte_dialing

    # Step 5: Trigger interface up to start dialing
    log "Triggering LTE interface up..."
    ifup LTE 2>/dev/null &

    log "LTE module initialization completed"
}

# Main loop
main() {
    log "Starting LTE Module Service"

    # Initialize module first
    init_lte_module

    local last_lte_info_update=$(date +%s)

    # Main loop - just update info cache
    while true; do
        current_time=$(date +%s)

        # Update LTE info cache at intervals
        if [ $((current_time - last_lte_info_update)) -ge $LTE_INFO_UPDATE_INTERVAL ]; then
            update_lte_info_cache
            last_lte_info_update=$current_time
        fi

        sleep 10
    done
}

# Run main loop
main
