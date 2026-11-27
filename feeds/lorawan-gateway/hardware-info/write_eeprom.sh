#!/bin/sh

# Defaults
DEFAULT_SN="1000791542610000"
DEFAULT_EUI="2CF7F1C0751000A0"
DEFAULT_FREQ_PLAN="US915"
DEFAULT_HW_NAME="reComputer-R1125"
DEFAULT_OUTPUT_FILE="/dev/eeprom"

# Frequency Plans Mapping
# US915: 0x0001
# EU868: 0x0002
# CN470: 0x0003

usage() {
    echo "Usage: $0 [options]"
    echo "Options:"
    echo "  --sn <sn>              Gateway SN (default: $DEFAULT_SN)"
    echo "  --eui <eui>            Gateway EUI hex (default: $DEFAULT_EUI)"
    echo "  --freq-plan <plan>     Frequency Plan (US915, EU868, CN470) (default: $DEFAULT_FREQ_PLAN)"
    echo "  --hw-name <name>       Hardware Name (default: $DEFAULT_HW_NAME)"
    echo "  --output <file>        Output file (default: $DEFAULT_OUTPUT_FILE)"
    echo "  --help                 Show this help message"
    exit 1
}

# Parse arguments
SN="$DEFAULT_SN"
EUI="$DEFAULT_EUI"
FREQ_PLAN="$DEFAULT_FREQ_PLAN"
HW_NAME="$DEFAULT_HW_NAME"
OUTPUT_FILE="$DEFAULT_OUTPUT_FILE"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --sn) SN="$2"; shift 2 ;;
        --eui) EUI="$2"; shift 2 ;;
        --freq-plan) FREQ_PLAN="$2"; shift 2 ;;
        --hw-name) HW_NAME="$2"; shift 2 ;;
        --output) OUTPUT_FILE="$2"; shift 2 ;;
        --help) usage ;;
        *) echo "Unknown option: $1"; usage ;;
    esac
done

# Validate Freq Plan
case "$FREQ_PLAN" in
    US915) FREQ_VAL="0001" ;;
    EU868) FREQ_VAL="0002" ;;
    CN470) FREQ_VAL="0003" ;;
    *) echo "Error: Invalid Frequency Plan. Must be US915, EU868, or CN470"; exit 1 ;;
esac


# Helper: 将16进制字符串转为二进制写入文件（仅支持偶数字符且无空格）
hexstr_to_bin() {
    hexstr="$1"
    while [ -n "$hexstr" ]; do
        byte=${hexstr:0:2}
        printf "\\x$byte"
        hexstr=${hexstr#??}
    done
}

# Helper to pad string with null bytes
pad_string() {
    str="$1"
    len="$2"
    printf "%-${len}s" "$str" | tr ' ' '\0'
}

# Create a temporary file for the data content (without CRC)
DATA_TMP=$(mktemp)

# 1. Magic (0xDEADBEEF) - 4 bytes + data length - 2 bytes
printf "\xDE\xAD\xBE\xEF" > "$DATA_TMP"
printf "\x00\x48" >> "$DATA_TMP"

# 2. SN - 18 bytes
# Truncate if too long, pad if too short
printf "%.18s" "$SN" >> "$DATA_TMP"
# Pad with nulls if length < 18
current_len=$(printf "%.18s" "$SN" | wc -c)
if [ "$current_len" -lt 18 ]; then
    pad_len=$((18 - current_len))
    dd if=/dev/zero bs=1 count="$pad_len" 2>/dev/null >> "$DATA_TMP"
fi


# 3. EUI - 8 bytes
# Ensure it is 16 chars long (8 bytes)
EUI_CLEAN=$(echo "$EUI" | tr -d '[:space:]')
if [ ${#EUI_CLEAN} -ne 16 ]; then
    echo "Error: EUI must be 16 hex characters (8 bytes)"
    rm "$DATA_TMP"
    exit 1
fi
hexstr_to_bin "$EUI_CLEAN" >> "$DATA_TMP"

# 4. Freq Plan - 2 bytes
hexstr_to_bin "$FREQ_VAL" >> "$DATA_TMP"

# 5. HW Name Length - 2 bytes
HW_NAME_LEN=${#HW_NAME}
if [ "$HW_NAME_LEN" -gt 32 ]; then
    HW_NAME_LEN=32
fi
# Convert length to 2-byte hex (Big Endian)
printf "\\$(printf '%03o' $((HW_NAME_LEN / 256)))\\$(printf '%03o' $((HW_NAME_LEN % 256)))" >> "$DATA_TMP"

# 6. HW Name - 32 bytes
printf "%.32s" "$HW_NAME" >> "$DATA_TMP"
current_len=$(printf "%.32s" "$HW_NAME" | wc -c)
if [ "$current_len" -lt 32 ]; then
    pad_len=$((32 - current_len))
    dd if=/dev/zero bs=1 count="$pad_len" 2>/dev/null >> "$DATA_TMP"
fi

# Calculate CRC32
calc_crc32() {
    if command -v python3 >/dev/null 2>&1; then
        python3 -c "import zlib, sys; print('%08X' % (zlib.crc32(sys.stdin.buffer.read()) & 0xffffffff))" < "$1"
    else
        echo "Error: python3 is required for CRC32 calculation." >&2
        exit 1
    fi
}

CRC_DATA_TMP=$(mktemp)
# same range as the production writer: bytes 6..67 (magic and data_len excluded)
tail -c +7 "$DATA_TMP" > "$CRC_DATA_TMP"
CRC_HEX=$(calc_crc32 "$CRC_DATA_TMP")
rm "$CRC_DATA_TMP"
# Convert CRC hex to binary (4 bytes)
CRC_HEX_PADDED=$(printf "%08s" "$CRC_HEX" | tr ' ' '0')
hexstr_to_bin "$CRC_HEX_PADDED" >> "$DATA_TMP"

# Write to output file
cat "$DATA_TMP" > "$OUTPUT_FILE"
if [ $? -eq 0 ]; then
    echo "Successfully wrote to $OUTPUT_FILE"
else
    echo "Error writing to $OUTPUT_FILE"
    rm "$DATA_TMP"
    exit 1
fi

rm "$DATA_TMP"
