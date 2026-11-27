This service is designed to read information from the EEPROM (/dev/eeprom) and write it into system configuration files, including:
- Gateway SN  ->  /etc/deviceinfo/sn
- Gateway EUI  ->  /etc/deviceinfo/eui
- Gateway Frequency Plan  ->  /etc/deviceinfo/freq_plan
- Gateway Hardware Name  ->  /etc/deviceinfo/hw_name


EEPROM binary format:
Offset  Length  Description
0x00    4       Magic Number: 0xDEADBEEF
0X04    2       DATA length (0x04 to 0X46)
0x06    18      Gateway SN (big-endian) E.g., "RCPGW0001XXXXXXXX"
0x18    8       Gateway EUI (big-endian) E.g., 0X0011223344556677
0X20    2       Freq Plan (big-endian) E.g., 0X0001 -> "US915"
        0X0001: US915
        0X0002: EU868
        0X0003: CN470
0X22    2      Hardware Name Length (big-endian) E.g., 0X0008
0X24    32     Hardware Name E.g., "R1225-EU868-4G" (Null-terminated if length <32)
0X44    4      CRC32 checksum of bytes 0x06 to 0X44 (magic and DATA length excluded)
