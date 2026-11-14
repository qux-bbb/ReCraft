// L1 structure recovery (3 blocks)
void __report_gsfailure_l1(void) {
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
        qword_ADDR = 2;
        *(u64*)(((rsp + 0) + 0xADDR)) = qword_ADDR;
        eax = 8;
        rcx = qword_ADDR;
        *(u64*)(((rsp + rax) + 0xADDR)) = rcx;
        sub_140001cb4(off_140029000);
        return;
    }
    __fastfail(2);
}
