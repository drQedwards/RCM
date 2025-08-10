#ifndef RCM_CLI_H
#define RCM_CLI_H

#ifdef __cplusplus
extern "C" {
#endif

// Run the RCM CLI with argc/argv semantics.
// Returns 0 on success, non-zero on error.
int rcm_run(int argc, const char* argv[]);

// Return a static, null-terminated version string.
// Lifetime: static; do NOT free.
const char* rcm_version(void);

#ifdef __cplusplus
}
#endif

#endif // RCM_CLI_H
