#include "bindings/bindings.h"

extern "C" bool ufc_vpx_present(void);
extern "C" void ufc_vpx_video_sample(const void *, const void *);
extern "C" void ufc_vpx_register(bool (*present)(void), void (*sample)(const void *, const void *));

int main(int argc, char * argv[]) {
	ufc_vpx_register(ufc_vpx_present, ufc_vpx_video_sample);
	ffi::start_app();
	return 0;
}
