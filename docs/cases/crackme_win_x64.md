# Case: crackme_win_x64.exe — strings → xref → disassembly → decompile → AI polish

End-to-end walkthrough of the feature requested for the CTF sample. The binary is not
vendored in this repository; the script takes its path.

| | |
|---|---|
| Binary | `D:\files\projects\ctf-simple-crackme\challenge\crackme_win_x64.exe` |
| Size | 146,432 bytes |
| SHA-256 | `6C84C77BFFFF5AD56123E3C2DCED03CF07E7DEAAFE9A3527E21E30FA8A38C5BC` |
| Project | new `.rcraft` project created by `recraft import` |
| Recovered flag | **`flag{shuffl3d_x0r3d_4nd_sh1ft3d}`** |

Reproduce the ReCraft part with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/case-crackme.ps1
```

## 1. Locate the flag feature string

After `import` + `analyze` (89 ms, 2,085 strings), the flag-like rows are:

| Address | Len | Text |
|---|---|---|
| `0x140018340` | 37 | `flag{strings_are_not_enough_you_know}` |
| `0x140018418` | 24 | `flag format: flag{...}` |
| `0x140018438` | 12 | `Enter flag:` |
| `0x140018450` | 26 | `[-] Wrong flag. Try again.` |
| `0x140018470` | 28 | `[+] Correct! Nice reversing.` |
| `0x140018490` | 45 | `[?] Hint: it is shuffled, xored, and shifted.` |

In the desktop app, double-clicking a row in **Strings** jumps to the address in
**Disassembly**; when the address names a stored string, it is rendered as a data row
(`db '…',0`, `; DATA XREF: …`) instead of being decoded as instructions.

The obvious `flag{strings_are_not_enough_you_know}` is a **trap**, not the answer (see
the `strcmp` branch below). The hint says the real check is shuffled/xored/shifted.

## 2. Xref and disassembly

`recraft xrefs --addr 0x140018340` → one code data reference:

```
<- 0x01400010b9  data
```

That lies in `sub_140001010` (`0x140001010..0x140001169`, 345 bytes). The relevant tail:

```asm
0x1400010a0  lea rdx, [0x140018448]       ; "\r\n"
0x1400010a7  lea rcx, [rsp+0x20]          ; char input[0x100]
0x1400010ac  call strchr
0x1400010b1  test rax, rax
0x1400010b4  je   0x1400010b9
0x1400010b6  mov  byte ptr [rax], 0       ; strip newline
0x1400010b9  lea rdx, [0x140018340]       ; decoy flag
0x1400010c0  lea rcx, [rsp+0x20]
0x1400010c5  call strcmp
0x1400010ca  test eax, eax
0x1400010cc  je   0x140001130            ; decoy -> "Wrong flag"
0x1400010ce  lea rcx, [rsp+0x20]
0x1400010d3  call strlen
0x1400010d8  cmp  rax, 0x20               ; real flag is 32 bytes
0x1400010dc  jne  0x140001130
...
0x1400010e0  movzx r8d, byte ptr [rax+rdi+0x18380]   ; perm[i]
0x1400010ef  movzx edx, byte ptr [rsp+r8+0x20]       ; input[perm[i]]
0x1400010f5  xor   dl, [rcx+rdi+0x18368]             ; ^ key[i & 15]
0x1400010fc  add   dl, r8b                            ; + perm[i]
0x1400010ff  cmp   dl, [rax+rdi+0x183a0]             ; target[i]
0x140001106  jne   0x140001130
```

Data tables in `.rdata`:

```
0x140018368  "C_CR4CK_ME_2024!"                         (16-byte XOR key)
0x140018380  02 19 11 12 1a 0f 09 0c 17 01 0e 0a ...     (permutation, 32 bytes)
0x1400183a0  24 50 81 48 1f 82 36 47 29 2a 35 68 ...     (target bytes, 32)
```

## 3. ReCraft decompiler output

The L2 output used to show raw registers only; after the string-literal and
prototype-name improvements it reads like this (abridged):

```c
// prototype: int main(void)
// L1 structure recovery (12 blocks)
int32 main(void) {
    printf((char*)"==========================================");
    printf((char*)"  Simple Crackme v2.0  (C edition)");
    ...
    while (1) {
        puts((char*)"Enter flag: ");
        __acrt_iob_func(1);
        fflush((void*)rax);
        __acrt_iob_func(rcx);
        fgets((char*)(rsp + 0x20), 0x100, (void*)rax);
        if (!((rax == 0))) break;
        strchr((char*)(rsp + 0x20), "\r\n");
        if (!(rax == 0)) {
            *(u8*)(rax) = 0;
        }
        strcmp((char*)(rsp + 0x20), (char*)"flag{strings_are_not_enough_you_know}");
        if ((eax == 0)) {
            printf((char*)"[-] Wrong flag. Try again.");
            ...
        }
        strlen((char*)(rsp + 0x20));
        if ((rax != 0x20)) goto loc_140001130;
    }
    loc_1400010e0:
    r8d = (uint32)(*(u8*)(((rax + (rdi * 1)) + 0x18380)));
    edx = (uint32)(*(u8*)(((rsp + (r8 * 1)) + 0x20)));
    dl = (dl ^ *(u8*)(((rcx + (rdi * 1)) + 0x18368)));
    dl = (dl + r8b);
    if ((dl != *(u8*)(((rax + (rdi * 1)) + 0x183a0)))) goto loc_140001130;
    ...
}
```

This keeps source addresses for UI cross-highlighting and is the evidence fed to the AI
polish prompt.

## 4. AI-polished, source-like reconstruction

> The AI polish step in this walkthrough was performed by the assistant in the session
> (no LLM provider/API key is configured in this checkout). ReCraft's
> `recraft ai polish` uses the same L1 evidence with a configured provider and returns a
> suggestion; it never stores the result automatically. The polished source below is the
> expected output for this case.
>
> The provider path itself is smoke-tested by `scripts/ai-polish-mock.ps1`, which starts a
> local OpenAI-compatible stub and runs `recraft ai polish` end-to-end (prompt → HTTP →
> cache → output). It returns canned text, so it validates plumbing, not model quality.
> With a real endpoint, set `RECRAFT_AI_PROVIDER`/`RECRAFT_AI_ENDPOINT`/`RECRAFT_AI_MODEL`
> and the key env var, then run:
>
> ```powershell
> recraft ai polish <project> --addr 0x140001010
> ```

```c
#include <stdint.h>
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
static const char k_decoy[] = "flag{strings_are_not_enough_you_know}";

static int validate(const char *input)
{
    if (strcmp(input, k_decoy) == 0) {
        return 0;                       /* obvious string is a trap */
    }
    if (strlen(input) != 32) {
        return 0;
    }
    for (int i = 0; i < 32; i++) {
        uint8_t index = k_perm[i];
        uint8_t value = (uint8_t)input[index];
        value ^= (uint8_t)k_key[i & 15];
        value += index;
        if (value != k_target[i]) {
            return 0;
        }
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
        if (!fgets(input, sizeof input, stdin)) {
            break;
        }
        char *newline = strchr(input, '\n');
        if (newline) {
            *newline = '\0';
        }
        if (!validate(input)) {
            printf("[-] Wrong flag. Try again.\n");
            continue;
        }
        printf("[+] Correct! Nice reversing.\n");
        return 0;
    }
    printf("[?] Hint: it is shuffled, xored, and shifted.\n");
    return 1;
}
```

## 5. Recovering and verifying the flag

Inverting the check (`target[i] == ((input[perm[i]] ^ key[i&15]) + perm[i]) & 0xff`):

```
input[perm[i]] = ((target[i] - perm[i]) & 0xff) ^ key[i & 15]
               = flag{shuffl3d_x0r3d_4nd_sh1ft3d}
```

Verification against the real binary:

```text
$ "flag{shuffl3d_x0r3d_4nd_sh1ft3d}" | crackme_win_x64.exe
...
Enter flag: [+] Correct! Nice reversing.
exit code: 0

$ "flag{strings_are_not_enough_you_know}" | crackme_win_x64.exe
Enter flag: [-] Wrong flag. Try again.
Enter flag: [?] Hint: it is shuffled, xored, and shifted.
exit code: 1
```

## Notes / remaining work

- ReCraft's decompiler improvements used here: address constants that name a stored
  string render as C literals (with a `.rdata` fallback for short literals such as
  `"\r\n"`), and user prototypes supply readable call names (`printf`, `fgets`,
  `strcmp`, …) and argument lists.
- The L2 output is still register-oriented: duplicated argument-register assignments,
  `var_8`/`var_n8` prologue noise and `goto`-based labels remain. The AI polish step
  removes them; a future round can fold call arguments and hide the prologue directly in
  the decompiler.
- Automated AI polish needs a configured provider (`RECRAFT_AI_PROVIDER`,
  `RECRAFT_AI_ENDPOINT`, API key env var); this checkout has none, so the AI pass was
  done interactively and recorded here.
