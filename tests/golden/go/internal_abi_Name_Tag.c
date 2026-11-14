// L1 structure recovery (11 blocks)
void internal/abi.Name.Tag_l1(void) {
    while (1) {
        if (rsp <= *(u64*)((r14 + 0xADDR))) {
            runtime.morestack_noctxt();
            continue;
        }
        internal/abi.Name.HasTag();
        if (al == 0) break;
        internal/abi.Name.ReadVarint();
        var_10 = rax;
        var_18 = 1;
        rbx += rax;
        rbx += 1;
        internal/abi.Name.ReadVarint();
        rcx = var_10;
        rcx += var_18;
        rbx = ((rcx + rax) + 1);
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
        } else {
            return;
        }
    }
    eax = 0;
    ebx = 0;
    return;
}
