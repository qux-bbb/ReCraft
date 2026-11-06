// L1 structure recovery (12 blocks)
void sub_140001010_l1(void) {
// some loops keep labels (an inner loop exits its parent)
    puts("==========================================");
    puts("  Simple Crackme v2.0  (C edition)");
    puts("  flag format: flag{...}");
    puts("==========================================");
    ebx = 0;
    do {
        printf("Enter flag: ");
        __acrt_iob_func(1);
        fflush(rax);
        ecx = 0;
        __acrt_iob_func(0);
        fgets(&var_20, 0xADDR, rax);
        if (rax == 0) {
            puts("[?] Hint: it is shuffled, xored, and shifted.");
            eax = 1;
            return;
        }
        strpbrk(&var_20, "\r\n");
        if (rax != 0) {
            *(u8*)(rax) = 0;
        }
        strcmp(&var_20, "flag{strings_are_not_enough_you_know}");
        if (eax != 0) {
            strlen(&var_20);
            if (rax == 0xADDR) {
                eax = 0;
                do {
                    r8d = (uint32)(byte_ADDR[rax]);
                    rcx = rax;
                    ecx &= 0xADDR;
                    edx = (uint32)(*(u8*)(&var_20 + r8));
                    dl ^= byte_ADDR[rcx];
                    dl += r8b;
                    if (dl != byte_ADDR[rax]) goto loc_ADDR;
                    rax += 1;
                } while (rax < 0xADDR);
                puts("[+] Correct! Nice reversing.");
                eax = 0;
                return;
            }
        }
        loc_ADDR:
        puts("[-] Wrong flag. Try again.");
        ebx += 1;
    } while (ebx < 3 /* signed */);
    puts("[?] Hint: it is shuffled, xored, and shifted.");
    eax = 1;
    return;
}
