#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <sys/stat.h>
#include <errno.h>

#define EEPROM_FILE "/dev/eeprom"
#define OUT_DIR "/etc/deviceinfo"

struct __attribute__((packed)) eeprom_layout {
    uint32_t magic;         // 0x00
    uint16_t data_len;      // 0x04
    uint8_t sn[18];         // 0x06
    uint8_t eui[8];         // 0x18
    uint16_t freq_plan;     // 0x20
    uint16_t hw_name_len;   // 0x22
    char hw_name[32];       // 0x24
    uint32_t crc;           // 0x44
};

// Freq Plan
static const char *freq_plans[] = {
    "US915", "EU868", "CN470"
};
uint32_t calc_crc32(const uint8_t *data, size_t len) {
    uint32_t crc = 0xFFFFFFFF;
    for (size_t i = 0; i < len; i++) {
        crc ^= data[i];
        for (int j = 0; j < 8; j++) {
            if (crc & 1)
                crc = (crc >> 1) ^ 0xEDB88320;
            else
                crc >>= 1;
        }
    }
    return ~crc;
}

void write_file(const char *filename, const char *content) {
    char path[256];
    snprintf(path, sizeof(path), "%s/%s", OUT_DIR, filename);
    FILE *f = fopen(path, "w");
    if (f) {
        fprintf(f, "%s", content);
        fclose(f);
    } else {
        fprintf(stderr, "Failed to write %s: %s\n", path, strerror(errno));
    }
}

int main(int argc, char *argv[]) {
    const char *dev_path = EEPROM_FILE;
    if (argc > 1) dev_path = argv[1];

    int fd = open(dev_path, O_RDONLY);
    if (fd < 0) {
        fprintf(stderr, "Cannot open %s: %s\n", dev_path, strerror(errno));
        return 1;
    }

    struct eeprom_layout data;
    if (read(fd, &data, sizeof(data)) != sizeof(data)) {
        fprintf(stderr, "Failed to read EEPROM data\n");
        close(fd);
        return 1;
    }
    close(fd);

    // Verify Magic
    uint32_t magic_val = ntohl(data.magic);
    if (magic_val != 0xDEADBEEF) {
        fprintf(stderr, "Warning: Invalid Magic: 0x%08X\n", magic_val);
    }

    // Verify CRC
    // CRC of bytes 0x06 to 0x44 (length 62), data_len excluded, as the production tool writes it
    uint32_t stored_crc = ntohl(data.crc);
    uint32_t calced_crc = calc_crc32((uint8_t*)&data + 6, 62);
    
    if (stored_crc != calced_crc) {
         fprintf(stderr, "Warning: CRC Mismatch. Stored: 0x%08X, Calc: 0x%08X\n", stored_crc, calced_crc);
    }

    // Create output directory
    mkdir(OUT_DIR, 0755);

    // SN
    char sn_str[19];
    memcpy(sn_str, data.sn, 18);
    sn_str[18] = '\0';
    write_file("sn", sn_str);

    // EUI
    char eui_str[17];
    snprintf(eui_str, sizeof(eui_str), "%02X%02X%02X%02X%02X%02X%02X%02X",
        data.eui[0], data.eui[1], data.eui[2], data.eui[3],
        data.eui[4], data.eui[5], data.eui[6], data.eui[7]);
    write_file("eui", eui_str);

    // Freq Plan
    uint16_t freq = ntohs(data.freq_plan);
    const char *freq_str = "UNKNOWN";
    if (freq > 0 && freq <= sizeof(freq_plans)/sizeof(freq_plans[0])) {
        freq_str = freq_plans[freq - 1];
    }
    write_file("freq_plan", freq_str);

    // HW Name
    uint16_t hw_len = ntohs(data.hw_name_len);
    if (hw_len > 32) hw_len = 32;
    char hw_name_str[33];
    memcpy(hw_name_str, data.hw_name, hw_len);
    hw_name_str[hw_len] = '\0';
    write_file("hw_name", hw_name_str);

    return 0;
}
