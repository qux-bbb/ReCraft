// L1 structure recovery (5 blocks)
void sub_140025680_l1(void) {
    var_38 = 0;
    RoGetMatchingRestrictedErrorInfo();
    if (eax >= 0 /* signed */) {
        SetRestrictedErrorInfo(var_38);
    }
    rcx = var_38;
    if (rcx != 0) {
        var_38 = 0;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010();
    }
    return;
}
