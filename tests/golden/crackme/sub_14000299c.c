// L1 structure recovery (1 blocks)
void sub_14000299c_l1(void) {
    r10 = var_138;
    rax = ")";
    xmm0 = *(__m128*)(rax);
    r11 = rcx;
    rcx = &var_30;
    xmm1 = *(__m128*)((rax + 0xADDR));
    *(__m128*)(rcx) = xmm0;
    xmm0 = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = xmm1;
    xmm1 = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = xmm0;
    xmm0 = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = xmm1;
    xmm1 = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = xmm0;
    xmm0 = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = xmm1;
    xmm1 = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = xmm0;
    xmm0 = *(__m128*)((rax + 0xADDR));
    rax = *(u64*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = xmm0;
    *(__m128*)((rcx + 0xADDR)) = xmm1;
    *(u64*)((rcx + 0xADDR)) = rax;
    eax = (uint32)(var_140);
    rax = *(u64*)((r10 + 0xADDR));
    var_90 = 0xADDR;
    RtlUnwindEx(*(u64*)(r11), *(u64*)(r10), &var_30, 0, *(u64*)((r10 + 0xADDR)), *(u64*)((r10 + 0xADDR)));
    return;
}
