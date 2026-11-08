// L1 structure recovery (5 blocks)
void bytes_op_l1(void) {
    if (esi <= 0 /* signed */) {
        eax = 0;
        return;
    }
    rsi = (int64)(esi);
    eax = 0;
    rcx = (rdi + rsi);
    do {
        edx = (uint32)(*(u8*)(rdi));
        rdi += 1;
        eax += edx;
    } while (rdi != rcx);
    return;
}
