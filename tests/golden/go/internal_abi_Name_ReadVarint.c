// L1 structure recovery (8 blocks)
void internal/abi.Name.ReadVarint_l1(void) {
// some loops keep labels (an inner loop exits its parent)
    while (1) {
        if (rsp > *(u64*)((r14 + 0xADDR))) {
            ecx = 0;
            edx = 0;
            goto loc_ADDR;
            do {
                rcx = (rdx + 1);
                rdx = rbx;
                loc_ADDR:
                var_20 = rdx;
                var_28 = rcx;
                rbx += rcx;
                internal/abi.Name.DataChecked(off_1400dbe94);
                rdx = var_28;
                rsi = (rdx + (rdx * 2));
                rcx = (rdx + (rsi * 2));
                if (rcx < 0 /* signed */) goto loc_ADDR;
                esi = (uint32)(*(u8*)(rax));
                r8d = esi;
                esi &= 0xADDR;
                rsi <<= cl;
                r9 = ((rcx < 0xADDR) ? -1 : 0);
                rsi &= r9;
                r9 = var_20;
                rbx = (rsi + r9);
            } while ((r8b & 0xADDR) != 0);
            rax = (rdx + 1);
            return;
            loc_ADDR:
        }
        runtime.morestack_noctxt();
    }
    rax = (rdx + 1);
    return;
}
