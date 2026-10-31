// L1 structure recovery (1 blocks)
void sub_140004bdc_l1(void) {
    rbx = r9;
    rdi = r8;
    sub_1400045f0(*(u64*)(rdx));
    sub_140004c1c(rdi);
    edi = eax;
    sub_1400045fc(*(u64*)(rbx));
    eax = edi;
    return;
}
