// L1 structure recovery (5 blocks)
void sub_1400087e0_l1(void) {
    xmm0 = *(__m128*)((rcx + 0xADDR));
    rbx = rdx;
    rdi = rcx;
    var_20 = xmm0;
    var_40 = *(f64*)((rcx + 0xADDR));
    rbp = *(u64*)((rcx + 0xADDR));
    *(u64*)((rcx + 0xADDR)) = 0;
    xmm0 = *(__m128*)((rdx + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = xmm0;
    *(f64*)((rcx + 0xADDR)) = *(f64*)((rdx + 0xADDR));
    rax = *(u64*)((rdx + 0xADDR));
    *(u64*)((rdx + 0xADDR)) = 0;
    rsi = *(u64*)((rcx + 0xADDR));
    *(u64*)((rcx + 0xADDR)) = rax;
    if (rsi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rsi);
    }
    *(__m128*)((rbx + 0xADDR)) = var_20;
    xmm0 = var_40;
    *(f64*)((rbx + 0xADDR)) = xmm0;
    rsi = *(u64*)((rbx + 0xADDR));
    *(u64*)((rbx + 0xADDR)) = rbp;
    if (rsi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rsi);
    }
    al = *(u8*)((rbx + 0xADDR));
    cl = *(u8*)((rdi + 0xADDR));
    *(u8*)((rdi + 0xADDR)) = al;
    al = *(u8*)((rbx + 0xADDR));
    *(u8*)((rbx + 0xADDR)) = cl;
    cl = *(u8*)((rdi + 0xADDR));
    *(u8*)((rdi + 0xADDR)) = al;
    *(u8*)((rbx + 0xADDR)) = cl;
    return;
}
