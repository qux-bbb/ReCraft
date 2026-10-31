// L1 structure recovery (3 blocks)
void sub_140001e88_l1(void) {
    var_58 = 0;
    rbx = rcx;
    var_50 = 0;
    var_60 = 0;
    RtlCaptureContext(rcx);
    rdi = *(u64*)((rbx + 0xADDR));
    RtlLookupFunctionEntry(rdi, &var_50, 0);
    if (rax != 0) {
        var_38 = 0;
        var_30 = &var_58;
        rcx = &var_60;
        var_28 = rcx;
        var_20 = rbx;
        RtlVirtualUnwind(0, var_50, rdi, rax, rbx, &var_60, &var_58, 0);
    }
    return;
}
