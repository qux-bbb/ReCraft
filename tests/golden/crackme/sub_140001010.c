// L1 structure recovery (12 blocks)
void sub_140001010_l1(void) {
// some loops keep labels (an inner loop exits its parent)
    sub_140004cc4("==========================================");
    sub_140004cc4("  Simple Crackme v2.0  (C edition)");
    sub_140004cc4("  flag format: flag{...}");
    sub_140004cc4("==========================================");
    ebx = 0;
    rdi = 0xADDR;
    loc_ADDR:
    sub_140001170("Enter flag: ", rdx, r8, r9);
    sub_140004580(1);
    sub_1400049f8(rax);
    ecx = 0;
    sub_140004580(0);
    sub_140004bd4(&var_20, 0xADDR, rax);
    if (rax == 0) {
        sub_140004cc4("[?] Hint: it is shuffled, xored, and shifted.");
        eax = 1;
        return;
    }
    sub_140006fd0(&var_20, "\r\n");
    if (rax == 0) {
        sub_140016b60(&var_20, "flag{strings_are_not_enough_you_know}");
        if (eax == 0) goto loc_ADDR;
        sub_140016be0(&var_20);
        if (rax != 0xADDR) goto loc_ADDR;
        eax = 0;
        r8d = (uint32)(*(u8*)(((rax + rdi) + 0xADDR)));
        rcx = rax;
        ecx &= 0xADDR;
        edx = (uint32)(*(u8*)(((rsp + r8) + 0xADDR)));
        dl ^= *(u8*)(((rcx + rdi) + 0xADDR));
        dl += r8b;
        if (dl != *(u8*)(((rax + rdi) + 0xADDR))) goto loc_ADDR;
        rax += 1;
        if (rax < 0xADDR) goto loc_ADDR;
        sub_140004cc4("[-] Wrong flag. Try again.");
        ebx += 1;
        if (ebx < 3 /* signed */) goto loc_ADDR;
        sub_140004cc4("[+] Correct! Nice reversing.");
        eax = 0;
        return;
    }
    *(u8*)(rax) = 0;
    loc_ADDR:
    sub_140016b60(&var_20, "flag{strings_are_not_enough_you_know}");
    if (eax == 0) {
        sub_140004cc4("[-] Wrong flag. Try again.");
        ebx += 1;
        if (ebx < 3 /* signed */) goto loc_ADDR;
        sub_140004cc4("[+] Correct! Nice reversing.");
        eax = 0;
        return;
    }
    sub_140016be0(&var_20);
    if (rax != 0xADDR) {
        sub_140004cc4("[-] Wrong flag. Try again.");
        ebx += 1;
        if (ebx < 3 /* signed */) goto loc_ADDR;
        sub_140004cc4("[+] Correct! Nice reversing.");
        eax = 0;
        return;
    }
    eax = 0;
    loc_ADDR:
    r8d = (uint32)(*(u8*)(((rax + rdi) + 0xADDR)));
    rcx = rax;
    ecx &= 0xADDR;
    edx = (uint32)(*(u8*)(((rsp + r8) + 0xADDR)));
    dl ^= *(u8*)(((rcx + rdi) + 0xADDR));
    dl += r8b;
    if (dl != *(u8*)(((rax + rdi) + 0xADDR))) {
        sub_140004cc4("[-] Wrong flag. Try again.");
        ebx += 1;
        if (ebx < 3 /* signed */) goto loc_ADDR;
        sub_140004cc4("[+] Correct! Nice reversing.");
        eax = 0;
        return;
    }
    rax += 1;
    if (rax < 0xADDR) {
        r8d = (uint32)(*(u8*)(((rax + rdi) + 0xADDR)));
        rcx = rax;
        ecx &= 0xADDR;
        edx = (uint32)(*(u8*)(((rsp + r8) + 0xADDR)));
        dl ^= *(u8*)(((rcx + rdi) + 0xADDR));
        dl += r8b;
        if (dl != *(u8*)(((rax + rdi) + 0xADDR))) goto loc_ADDR;
        rax += 1;
        if (rax < 0xADDR) goto loc_ADDR;
        sub_140004cc4("[-] Wrong flag. Try again.");
        ebx += 1;
        if (ebx < 3 /* signed */) goto loc_ADDR;
        sub_140004cc4("[+] Correct! Nice reversing.");
        eax = 0;
        return;
    }
    loc_ADDR:
    sub_140004cc4("[-] Wrong flag. Try again.");
    ebx += 1;
    if (ebx < 3 /* signed */) {
        sub_140001170("Enter flag: ", rdx, r8, r9);
        sub_140004580(1);
        sub_1400049f8(rax);
        ecx = 0;
        sub_140004580(0);
        sub_140004bd4(&var_20, 0xADDR, rax);
        if (rax == 0) {
            sub_140004cc4("[?] Hint: it is shuffled, xored, and shifted.");
            eax = 1;
            return;
        }
        sub_140006fd0(&var_20, "\r\n");
        if (rax == 0) goto loc_ADDR;
        *(u8*)(rax) = 0;
        sub_140016b60(&var_20, "flag{strings_are_not_enough_you_know}");
        if (eax == 0) goto loc_ADDR;
        sub_140016be0(&var_20);
        if (rax != 0xADDR) goto loc_ADDR;
        eax = 0;
        r8d = (uint32)(*(u8*)(((rax + rdi) + 0xADDR)));
        rcx = rax;
        ecx &= 0xADDR;
        edx = (uint32)(*(u8*)(((rsp + r8) + 0xADDR)));
        dl ^= *(u8*)(((rcx + rdi) + 0xADDR));
        dl += r8b;
        if (dl != *(u8*)(((rax + rdi) + 0xADDR))) goto loc_ADDR;
        rax += 1;
        if (rax < 0xADDR) goto loc_ADDR;
        sub_140004cc4("[-] Wrong flag. Try again.");
        ebx += 1;
        if (ebx < 3 /* signed */) goto loc_ADDR;
        sub_140004cc4("[+] Correct! Nice reversing.");
        eax = 0;
        return;
    }
    sub_140004cc4("[+] Correct! Nice reversing.");
    eax = 0;
    return;
}
