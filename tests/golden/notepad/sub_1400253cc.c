// L1 structure recovery (5 blocks)
void sub_1400253cc_l1(void) {
    if (*(u8*)((rcx + 0xADDR)) != 0xADDR) {
        *(u8*)((rcx + 0xADDR)) = bl;
        rax = *(u64*)((rcx + 8));
        esi = *(u32*)(rax);
        rax = *(u64*)(rcx);
        rdi = *(u64*)(rax);
        if (rsi != 0) {
            do {
                CoTaskMemFree(*(u64*)((rdi + (rbx * 8))));
                rbx += 1;
            } while (rbx < rsi);
        }
        CoTaskMemFree(rdi);
    }
    return;
}
