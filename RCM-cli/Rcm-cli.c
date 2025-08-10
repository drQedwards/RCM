// Minimal C front-end that forwards to the Rust library.
// Build example (Linux/macOS):
//   cc -O2 -o rcm-cli RCM-cli.c -L. -lrcm_cli
// …assuming you've built the Rust cdylib named `librcm_cli.(so|dylib|dll)`.

#include <stdio.h>
#include <stdlib.h>
#include "RCM-cli.h"

int main(int argc, const char* argv[]) {
    // If invoked with no args, show a tiny help hint via Rust path.
    if (argc <= 1) {
        const char* ver = rcm_version();
        fprintf(stdout, "RCM CLI (C front-end) - %s\n", ver ? ver : "unknown");
        const char* helpv[] = {"rcm", "--help"};
        return rcm_run(2, helpv);
    }
    return rcm_run(argc, argv);
}
