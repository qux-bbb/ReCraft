// L1 structure recovery (5 blocks)
void sub_140010ac0_l1(void) {
    rbx = *(u64*)(r8);
    rcx = *(u64*)((rcx + 8));
    rdi = *(u64*)(r9);
    r8 = *(u64*)(rdx);
    rcx = *(u64*)(rcx);
    GetFullPathNameW(*(u64*)(rcx), ebx, *(u64*)(rdx), 0);
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
