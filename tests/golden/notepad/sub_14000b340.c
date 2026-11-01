// L1 structure recovery (9 blocks)
void sub_14000b340_l1(void) {
    r9d = 0xADDR;
    do {
        r8d = *(u32*)((rcx + 0xADDR));
        if (r8d == r9d) break;
        edx = (r8 - 1);
        eax = r8d;
        __asm("lock cmpxchg [rcx+0xADDR], edx");
    } while (*(u32*)((rcx + 0xADDR)) != eax);
    ebx = (r8 - 1);
    if (ebx == 0) {
        if (rcx != 0) {
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, (rbx + 1));
        }
        rcx = qword_ADDR;
        if (rcx != 0) {
            rax = *(u64*)((*(u64*)(rcx) + 0xADDR));
            sub_140028010(rcx, *(u64*)(rcx));
        }
    }
    eax = ebx;
    return;
}
