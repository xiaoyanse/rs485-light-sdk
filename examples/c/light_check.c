#include "rs485_light_sdk.h"
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc < 2 || argc > 4) {
        fprintf(stderr, "usage: light-check <port> [station-hex] [preview-scene]\n");
        return 2;
    }
    if (rs485_light_abi_version() != RS485_LIGHT_ABI_VERSION) {
        fprintf(stderr, "incompatible SDK ABI\n");
        return 2;
    }
    uint8_t station = 0;
    uint16_t word = 0;
    rs485_light_handle *light = NULL;
    rs485_light_status status = rs485_light_parse_station(argc >= 3 ? argv[2] : "FF", &station);
    if (status == RS485_LIGHT_OK) {
        status = rs485_light_open(argv[1], station, 9600, 750, &light);
    }
    if (status == RS485_LIGHT_OK) {
        status = rs485_light_check(light, &word);
    }
    if (status == RS485_LIGHT_OK) {
        printf("Station %02X responded, register 00FF = %04X\n", (unsigned)station, (unsigned)word);
        if (argc == 4) {
            status = rs485_light_preview(light, argv[3], 500);
            if (status == RS485_LIGHT_OK) {
                puts("Scene and all-off acknowledged");
            }
        }
    }
    rs485_light_status close_status = rs485_light_close(light);
    if (status == RS485_LIGHT_OK) {
        status = close_status;
    }
    if (status != RS485_LIGHT_OK) {
        fprintf(stderr, "%s\n", rs485_light_status_message(status));
        return 1;
    }
    return 0;
}
