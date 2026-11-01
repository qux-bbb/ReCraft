// L1 structure recovery (5 blocks)
void sub_140022100_l1(void) {
    rbx = *(u64*)(r8);
    rdx = *(u64*)(rdx);
    rdi = *(u64*)(r9);
    GetCurrentDirectoryW(ebx, *(u64*)(rdx));
    ecx = 0;
    *(u64*)(rdi) = rax;
    if (rax != 0) {
        if (rax < rbx) {
            rax += 1;
            *(u64*)(rdi) = rax;
        }
    } else {
        sub_140007cd0(var_28, 0xADDR, "onecore\\internal\\sdk\\inc\\wil\\opensource/wil/filesystem.h");
        ecx = eax;
    }
    eax = ecx;
    return;
}
