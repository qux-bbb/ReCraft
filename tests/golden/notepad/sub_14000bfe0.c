// L1 structure recovery (4 blocks)
void sub_14000bfe0_l1(void) {
    rcx = *(u64*)((rcx + 8));
    rbx = *(u64*)(r8);
    rdx = *(u64*)(rdx);
    rdi = *(u64*)(r9);
    rcx = *(u64*)(rcx);
    sub_140024420(rcx, rdx, (uint32)(bx));
    rdx = (int64)(ax);
    ecx = 0;
    if (dx >= 0 /* signed */) {
        __asm("cmovne rbx, rdx");
        *(u64*)(rdi) = rbx;
    } else {
        sub_140007cd0(var_28, 0xADDR, "shell\\osshell\\accesory\\notepad\\filesystemhelpers.h");
        ecx = eax;
    }
    eax = ecx;
    return;
}
