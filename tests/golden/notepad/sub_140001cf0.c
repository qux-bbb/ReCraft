// L1 structure recovery (3 blocks)
void sub_140001cf0_l1(void) {
    var_40 = rcx;
    IsProcessorFeaturePresent(0xADDR);
    if (eax == 0) {
        sub_140001f0c(off_140034dc0);
        qword_ADDR = var_38;
        rax = (&var_38 + 8);
        qword_ADDR = rax;
        qword_ADDR = qword_ADDR;
        qword_ADDR = var_40;
        dword_ADDR = 0xADDR;
        dword_ADDR = 1;
        dword_ADDR = 1;
        rax = 0;
        rcx = off_140034d40;
        *(u64*)((rcx + rax)) = 2;
        rax = 0;
        *(u64*)(((rsp + rax) + 0xADDR)) = qword_ADDR;
        eax = 8;
        rcx = qword_ADDR;
        *(u64*)(((rsp + rax) + 0xADDR)) = rcx;
        sub_140001cb4(off_140029000);
        return;
    }
    ecx = 2;
    __asm("int 0xADDR");
}
