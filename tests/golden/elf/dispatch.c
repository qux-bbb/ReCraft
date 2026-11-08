// L1 structure recovery (3 blocks)
void dispatch_l1(void) {
    if (edi > 3) {
        eax = 0xADDR;
        return;
    }
    rdx = (int64)(edi);
    rax = 0xADDR;
    __asm("jmp qword ptr [rax+rdx*8]");
}
