// L1 structure recovery (11 blocks)
void internal/abi.Name.Name_l1(void) {
    while (1) {
        if (rsp <= *(u64*)((r14 + 0xADDR))) {
            runtime.morestack_noctxt();
            continue;
        }
        if (rax == 0) break;
        ebx = 1;
        internal/abi.Name.ReadVarint();
        rbx = (rax + 1);
        internal/abi.Name.DataChecked(off_1400dd27c);
        if (rbx < 0 /* signed */) {
            runtime.panicunsafestringlen();
            eax = 0;
            ebx = 0;
            return;
        }
        rcx = rax;
        rcx = -(rcx);
        if (rbx > rcx) {
            if (rax != 0) {
                runtime.panicunsafestringlen();
            }
            runtime.panicunsafestringnilptr();
            runtime.panicunsafestringlen();
        }
    }
    eax = 0;
    ebx = 0;
    return;
}
