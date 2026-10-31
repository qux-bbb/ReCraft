// L1 structure recovery (7 blocks)
void sub_140002adc_l1(void) {
    rdi = rcx;
    sub_14000217c(rcx);
    if (rdi != *(u64*)((rax + 0xADDR))) {
        sub_140008860();
    }
    sub_14000217c(rcx);
    rdx = *(u64*)((rax + 0xADDR));
    if (rdx == 0) {
        sub_140008860();
        return;
    }
    while (1) {
        rbx = *(u64*)((rdx + 8));
        if (rdi == rdx) break;
        rdx = rbx;
        if (rbx == 0) {
            sub_140008860();
            return;
        }
        continue;
    }
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = rbx;
    return;
}
