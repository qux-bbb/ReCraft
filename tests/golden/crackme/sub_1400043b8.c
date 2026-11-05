// L1 structure recovery (8 blocks)
void sub_1400043b8_l1(void) {
    rbx = rdx;
    rsi = rcx;
    edi = 0xADDR;
    if (rdx != 0 && (*(u8*)(rdx) & 0xADDR) != 0) {
        rcx = (*(u64*)(rcx) - 0xADDR);
        rax = *(u64*)(rcx);
        rbx = *(u64*)((rax + 0xADDR));
        rax = *(u64*)((rax + 0xADDR));
        (*qword_ADDR)();
    }
    eax = 0;
    var_20 = 0;
    if (rbx != 0) {
        RtlPcToFileHeader(rbx, &var_20);
        var_20 = 0;
        if ((*(u8*)(rbx) & 8) != 0) {
            edi = 0xADDR;
        } else if (rax == 0) {
            edx = 1;
            var_28 = 0xADDR;
            var_30 = rsi;
            var_38 = rbx;
            var_40 = 0;
            RaiseException(0xADDR, 1, (rdx + 3), &var_28);
            return;
        }
    }
    edx = 1;
    var_28 = 0xADDR;
    var_30 = rsi;
    var_38 = rbx;
    var_40 = rax;
    RaiseException(0xADDR, 1, (rdx + 3), &var_28);
    return;
}
