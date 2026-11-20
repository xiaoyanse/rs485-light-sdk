#ifndef RS485_LIGHT_SDK_H
#define RS485_LIGHT_SDK_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#define RS485_LIGHT_API __declspec(dllimport)
#define RS485_LIGHT_CALL __cdecl
#else
#define RS485_LIGHT_API
#define RS485_LIGHT_CALL
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define RS485_LIGHT_ABI_VERSION 1

typedef struct rs485_light_handle rs485_light_handle;
typedef int32_t rs485_light_status;

enum {
    RS485_LIGHT_OK = 0,
    RS485_LIGHT_INVALID_ARGUMENT = 1,
    RS485_LIGHT_OPEN_FAILED = 2,
    RS485_LIGHT_IO_FAILED = 3,
    RS485_LIGHT_TIMEOUT = 4,
    RS485_LIGHT_BAD_RESPONSE = 5,
    RS485_LIGHT_DEVICE_EXCEPTION = 6,
    RS485_LIGHT_RESET_UNCONFIRMED = 7,
    RS485_LIGHT_BUFFER_TOO_SMALL = 8,
    RS485_LIGHT_INTERNAL = 9
};

RS485_LIGHT_API uint32_t RS485_LIGHT_CALL rs485_light_abi_version(void);
/* Static UTF-8 string owned by the DLL. Do not free it. */
RS485_LIGHT_API const char *RS485_LIGHT_CALL rs485_light_status_message(rs485_light_status status);

/* UTF-8 port names separated by LF. required includes the final NUL. Use (NULL, 0, &required)
 * to query capacity. Retry after BUFFER_TOO_SMALL if the set of ports changed. */
RS485_LIGHT_API rs485_light_status RS485_LIGHT_CALL rs485_light_list_ports(
    char *buffer, size_t capacity, size_t *required);
RS485_LIGHT_API rs485_light_status RS485_LIGHT_CALL rs485_light_parse_station(
    const char *hex, uint8_t *value);

/* All input strings are NUL-terminated UTF-8. The station is a numeric byte
 * (e.g. 0xff), baud is positive, and timeout_ms is 1–30000. open clears *out on failure. */
RS485_LIGHT_API rs485_light_status RS485_LIGHT_CALL rs485_light_open(
    const char *port, uint8_t station, uint32_t baud, uint32_t timeout_ms, rs485_light_handle **out);
RS485_LIGHT_API rs485_light_status RS485_LIGHT_CALL rs485_light_check(
    rs485_light_handle *handle, uint16_t *word);
RS485_LIGHT_API rs485_light_status RS485_LIGHT_CALL rs485_light_apply(
    rs485_light_handle *handle, const char *scene);
RS485_LIGHT_API rs485_light_status RS485_LIGHT_CALL rs485_light_preview(
    rs485_light_handle *handle, const char *scene, uint32_t hold_ms);

/* Serialize all requests on each handle, including close. After close the handle is invalid.
 * Close releases the port. Use apply(handle, "off") when all-off is required. */
RS485_LIGHT_API rs485_light_status RS485_LIGHT_CALL rs485_light_close(rs485_light_handle *handle);

#ifdef __cplusplus
}
#endif

#endif
