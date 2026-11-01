// L1 structure recovery (6 blocks)
void sub_140025650_l1(void) {
    r9d = 0xADDR;
    while (1) {
        r8d = *(u32*)((rcx + 0xADDR));
        if (r8d == r9d) break;
        edx = (r8 + 1);
        eax = r8d;
        __asm("lock cmpxchg [rcx+0xADDR], edx");
        if (*(u32*)((rcx + 0xADDR)) != eax) continue;
    }
    eax = r9d;
    return;
}
