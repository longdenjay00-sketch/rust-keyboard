/* librust_keyboard.h — C-ABI Header for rust-keyboard (aarch64) */
#ifndef LIBRUST_KEYBOARD_H
#define LIBRUST_KEYBOARD_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct RkbHandle RkbHandle;

RkbHandle* rkb_engine_new(void);
const char* rkb_engine_press(RkbHandle* handle, const char* token_cstr);
void rkb_engine_free(RkbHandle* handle);

#ifdef __cplusplus
}
#endif

#endif /* LIBRUST_KEYBOARD_H */
