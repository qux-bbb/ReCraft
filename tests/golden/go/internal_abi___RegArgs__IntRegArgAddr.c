// L1 structure recovery (9 blocks)
void internal/abi.(*RegArgs).IntRegArgAddr_l1(void) {
    while (1) {
        if (rsp <= *(u64*)((r14 + 0xADDR))) {
            runtime.morestack_noctxt();
            continue;
        }
        if (rcx > 8) {
            rax = off_1400bd6e0;
            rbx = off_1400e73b0;
            runtime.gopanic();
        }
        rdx = (rcx - 1);
        if (rbx < 9) break;
        runtime.panicBounds(rcx, rdx, r8, r9);
    }
    rax += (rbx * 8);
    return;
}
