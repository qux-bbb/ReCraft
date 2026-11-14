// L1 structure recovery (7 blocks)
void sub_140006a1c_l1(void) {
    rbx = rcx;
    if (*(u8*)((rcx + 0xADDR)) != 0) {
        rax = qword_ADDR;
        r8 = rcx;
        var_40 = rax;
        var_38 = qword_ADDR;
        rax = qword_ADDR;
        var_30 = rax;
        sub_140007570(&var_40, 3, rcx);
    }
    if (*(u8*)((rbx + 0xADDR)) != 0) {
        var_40 = qword_ADDR;
        var_38 = qword_ADDR;
        rax = qword_ADDR;
        var_30 = rax;
        sub_140007570(&var_40, 3, (rbx + 0xADDR));
    }
    if (*(u8*)((rbx + 0xADDR)) != 0) {
        var_40 = qword_ADDR;
        var_38 = qword_ADDR;
        var_30 = qword_ADDR;
        var_28 = qword_ADDR;
        var_20 = qword_ADDR;
        rax = qword_ADDR;
        var_18 = rax;
        sub_140007570(&var_40, 6, (rbx + 0xADDR));
    }
    return;
}
