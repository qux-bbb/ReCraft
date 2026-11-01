// L1 structure recovery (3 blocks)
void sub_140001de4_l1(void) {
    var_30 = ecx;
    IsProcessorFeaturePresent(0xADDR);
    if (eax == 0) {
        sub_140001e88(off_140034dc0);
        qword_ADDR = var_28;
        rax = (&var_28 + 8);
        qword_ADDR = rax;
        qword_ADDR = qword_ADDR;
        dword_ADDR = 0xADDR;
        dword_ADDR = 1;
        dword_ADDR = 1;
        eax = 8;
        rax = 0;
        rcx = off_140034d40;
        edx = var_30;
        *(u64*)((rcx + rax)) = rdx;
        sub_140001cb4(off_140029000);
        return;
    }
    eax = var_30;
    ecx = eax;
    __asm("int 0xADDR");
}
