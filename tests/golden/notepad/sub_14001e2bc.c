// L1 structure recovery (13 blocks)
void sub_14001e2bc_l1(void) {
    rbx = rcx;
    rdx = rcx;
    sub_14001f57c("RestartByRestartManager:", rcx);
    if (eax == 0) {
        sub_1400222dc(&var_38, rbx);
        rbx = var_38;
        if (rbx != 0) {
            rdx = off_14002a790;
            sub_140001c90(0xADDR);
            if (rax == 0) {
                if (rbx != 0) {
                    rcx = rbx;
                    CoTaskMemFree(rcx);
                }
            } else {
                sub_140021fc4(rax, &var_38);
                rbx = rax;
                if (rax == 0) {
                    rbx = var_38;
                } else {
                    sub_1400229e8(rax);
                    rcx = *(u64*)(rbx);
                    rax = *(u64*)(rcx);
                    sub_140028010(rbx, 1);
                    rcx = var_38;
                    if (rcx != 0) {
                        CoTaskMemFree(rcx);
                        al = 1;
                        return;
                    }
                }
            }
        }
        al = 1;
    } else {
        al = 0;
    }
    return;
}
