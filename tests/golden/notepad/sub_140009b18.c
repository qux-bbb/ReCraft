// L1 structure recovery (9 blocks)
void sub_140009b18_l1(void) {
    r9 = (rcx + 0xADDR);
    edi = 0;
    *(u64*)(r9) = rdi;
    rbx = rcx;
    rcx = *(u64*)(rdx);
    rdx = -1;
    do {
        rdx += 1;
    } while (*(u16*)((rcx + (rdx * 2))) != di);
    if (rdx > 0xADDR) {
        sub_14000b320(0xADDR);
    }
    eax = (rdx + 1);
    if (eax < edx) {
        sub_14000b320(0xADDR);
    }
    __asm("cmova edx, edx");
    WindowsCreateStringReference(rcx, rdx, rbx, r9);
    if (eax < 0 /* signed */) {
        ecx = eax;
        sub_14000b320(eax);
    }
    rax = rbx;
    return;
}
