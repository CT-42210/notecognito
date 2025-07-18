#ifndef NOTECOGNITO_H
#define NOTECOGNITO_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque type for the configuration manager */
typedef struct ConfigManager ConfigManager;

/* Result type for FFI functions */
typedef struct {
    bool success;
    char* error_message;
} FfiResult;

/* Frees a string allocated by Rust */
void notecognito_free_string(char* s);

/* Creates a new configuration manager */
ConfigManager* notecognito_config_manager_new(void);

/* Frees a configuration manager */
void notecognito_config_manager_free(ConfigManager* manager);

/* Updates a notecard (id must be 1-9) */
FfiResult notecognito_update_notecard(ConfigManager* manager, int id, const char* content);

/* Gets notecard content (caller must free the returned string) */
char* notecognito_get_notecard_content(ConfigManager* manager, int id);

/* Gets the configuration as JSON (caller must free the returned string) */
char* notecognito_get_config_json(ConfigManager* manager);

/* Sets the launch on startup flag */
FfiResult notecognito_set_launch_on_startup(ConfigManager* manager, bool enabled);

#ifdef __cplusplus
}
#endif

#endif /* NOTECOGNITO_H */