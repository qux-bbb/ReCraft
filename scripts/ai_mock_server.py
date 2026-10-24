# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Minimal OpenAI-compatible chat-completions stub for `recraft ai polish` smoke tests.
# It returns a canned source-like suggestion; it is NOT a model.

import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

POLISHED = r"""#include <stdint.h>
#include <stdio.h>
#include <string.h>

static const uint8_t k_perm[32] = {
    0x02, 0x19, 0x11, 0x12, 0x1a, 0x0f, 0x09, 0x0c, 0x17, 0x01, 0x0e, 0x0a,
    0x13, 0x16, 0x06, 0x00, 0x05, 0x1f, 0x0d, 0x1e, 0x14, 0x1c, 0x03, 0x15,
    0x18, 0x07, 0x04, 0x10, 0x1d, 0x1b, 0x0b, 0x08,
};
static const uint8_t k_target[32] = {
    0x24, 0x50, 0x81, 0x48, 0x1f, 0x82, 0x36, 0x47, 0x29, 0x2a, 0x35, 0x68,
    0x82, 0x6c, 0x62, 0x47, 0x35, 0x41, 0x29, 0x54, 0x14, 0x53, 0x2f, 0x46,
    0x56, 0x37, 0x28, 0x50, 0x20, 0x6f, 0x12, 0x4f,
};
static const char k_key[16] = "C_CR4CK_ME_2024!";

static int validate(const char *input)
{
    if (strcmp(input, "flag{strings_are_not_enough_you_know}") == 0)
        return 0; /* obvious decoy */
    if (strlen(input) != 32)
        return 0;
    for (int i = 0; i < 32; i++) {
        uint8_t index = k_perm[i];
        uint8_t value = (uint8_t)input[index] ^ (uint8_t)k_key[i & 15];
        value += index;
        if (value != k_target[i])
            return 0;
    }
    return 1;
}

int main(void)
{
    char input[0x100];
    printf("==========================================\n");
    printf("  Simple Crackme v2.0  (C edition)\n");
    printf("  flag format: flag{...}\n");
    printf("==========================================\n");
    for (int attempt = 0; attempt < 3; attempt++) {
        printf("Enter flag: ");
        fflush(stdout);
        if (!fgets(input, sizeof input, stdin))
            break;
        char *newline = strchr(input, '\n');
        if (newline)
            *newline = '\0';
        if (!validate(input)) {
            printf("[-] Wrong flag. Try again.\n");
            continue;
        }
        printf("[+] Correct! Nice reversing.\n");
        return 0;
    }
    printf("[?] Hint: it is shuffled, xored, and shifted.\n");
    return 1;
}"""


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):  # noqa: N802 (http.server API)
        length = int(self.headers.get("Content-Length", "0"))
        self.rfile.read(length)
        body = json.dumps(
            {"choices": [{"message": {"role": "assistant", "content": POLISHED}}]}
        ).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        pass


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 18765
    HTTPServer(("127.0.0.1", port), Handler).serve_forever()
