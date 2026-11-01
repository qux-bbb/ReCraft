// L1 structure recovery (3 blocks)
void sub_14000a048_l1(void) {
    xmm0 = *(__m128*)((rcx + 0xADDR));
    xmm1 = *(__m128*)((rcx + 0xADDR));
    var_30 = xmm0;
    xmm0 = *(__m128*)((rcx + 0xADDR));
    var_40 = xmm1;
    xmm1 = *(__m128*)((rcx + 0xADDR));
    var_50 = xmm0;
    var_60 = xmm1;
    if (*(u64*)((rcx + 8)) != 0) {
        xmm0 = *(__m128*)((rcx + 0xADDR));
        xmm1 = *(__m128*)((rcx + 0xADDR));
        xmm2 = *(__m128*)(rcx);
        *(__m128*)((rax - 0xADDR)) = xmm0;
        xmm0 = *(__m128*)((rcx + 0xADDR));
        *(__m128*)((rax - 0xADDR)) = xmm1;
        xmm1 = *(__m128*)((rcx + 0xADDR));
        *(__m128*)((rax - 0xADDR)) = xmm0;
        *(__m128*)((rax - 0xADDR)) = xmm1;
        *(__m128*)((rax - 0xADDR)) = xmm2;
        EndPaint(xmm2, (rax - 0xADDR));
    }
    return;
}
