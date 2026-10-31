// L1 structure recovery (10 blocks)
void sub_140002cb4_l1(void) {
    rbx = r8;
    if (rcx == 0) {
        sub_140008860();
    }
    r11 = (int64)(*(u32*)((rcx + 0xADDR)));
    r10 = *(u64*)((rdx + 8));
    rax = (r10 + r11);
    if (rax == 0) {
        sub_140008860();
        return;
    }
    r8d = *(u32*)((rcx + 0xADDR));
    r9d = 0;
    if (r8d == 0) {
        eax = -1;
    } else {
        do {
            rcx = (r11 + (r9 * 8));
            rdx = (int64)(*(u32*)((rcx + r10)));
            rdx += r10;
            if (rbx < rdx) break;
            r9d += 1;
        } while (r9d < r8d);
        if (r9d == 0) {
            eax = -1;
            return;
        }
        ecx = (r9 - 1);
        rax = (r10 + (rcx * 8));
        eax = *(u32*)(((rax + r11) + 4));
    }
    return;
}
