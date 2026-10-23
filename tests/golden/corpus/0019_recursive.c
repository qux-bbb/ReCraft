// L1 structure recovery (3 blocks)
void recursive_l1(void) {
    __asm("push rbx");
    rsp = (rsp - 0xADDR);
    ebx = ecx;
    if ((ecx > 1 /* signed */)) {
        ecx = (ecx - 1);
        recursive();
        __asm("imul eax, ebx");
        rsp = (rsp + 0xADDR);
        __asm("pop rbx");
        return;
    }
    eax = 1;
    rsp = (rsp + 0xADDR);
    __asm("pop rbx");
    return;
}
