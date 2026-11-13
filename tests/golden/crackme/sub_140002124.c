// L1 structure recovery (5 blocks)
void sub_140002124_l1(void) {
    rax = qword_ADDR;
    if (rax == sub_140001750) {
        return;
    }
    __asm("mov rax, gs:[0xADDR]");
    rcx = *(u64*)((rcx + 0xADDR));
    if (rcx < *(u64*)((rax + 0xADDR))) {
        __fastfail(0xADDR);
    }
    if (rcx <= *(u64*)((rax + 8))) {
        return;
    }
}
