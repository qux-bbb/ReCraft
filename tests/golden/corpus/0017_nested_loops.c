// L1 structure recovery (16 blocks)
void nested_loops_l1(void) {
    *(u64*)((rsp - 8)) = rbx;
    rsp = (rsp - 8);
    *(u64*)((rsp - 8)) = rsi;
    rsp = (rsp - 8);
    esi = (esi ^ esi);
    r11d = (r11d ^ r11d);
    ebx = ecx;
    __asm("if (jle 0xADDR /* unmodelled flags */) goto 0xADDR");
    __asm("movdqa xmm3, [0xADDR]");
    xmm4 = (xmm4 ^ xmm4);
    var_28 = rbp;
    xmm2 = (xmm2 ^ xmm2);
    ebp = *(u32*)(0xADDR);
    var_30 = rdi;
    edi = (edi ^ edi);
    edx = (edx ^ edx);
    xmm1 = r11d;
    __asm("pshufd xmm1, xmm1, 0xADDR");
    if ((ebx < 8)) goto loc_ADDR;
    loc_ADDR:
    if ((ebp < 2 /* signed */)) goto loc_ADDR;
    loc_ADDR:
    ecx = ebx;
    ecx = (ecx & 0xADDR);
    loc_ADDR:
    xmm0 = edx;
    eax = (rdx + 4);
    __asm("pshufd xmm0, xmm0, 0xADDR");
    edx = (edx + 8);
    xmm0 = (xmm0 + xmm3);
    xmm0 = (xmm0 * xmm1);
    xmm4 = (xmm4 + xmm0);
    xmm0 = eax;
    __asm("pshufd xmm0, xmm0, 0xADDR");
    xmm0 = (xmm0 + xmm3);
    xmm0 = (xmm0 * xmm1);
    xmm2 = (xmm2 + xmm0);
    if ((edx < ecx /* signed */)) goto loc_ADDR;
    loc_ADDR:
    r8d = (r8d ^ r8d);
    r9d = (r9d ^ r9d);
    if ((edx >= ebx /* signed */)) goto loc_ADDR;
    loc_ADDR:
    eax = ebx;
    eax = (eax - edx);
    if ((eax < 2 /* signed */)) goto loc_ADDR;
    loc_ADDR:
    eax = edx;
    ecx = ebx;
    ecx = (ecx - edx);
    eax = (eax * r11d);
    ecx = (ecx - 2);
    ecx = (ecx >> 1);
    ecx = (ecx + 1);
    r10d = ecx;
    edx = (rdx + (rcx * 2));
    loc_ADDR:
    ecx = (rax + (r11 * 1));
    r8d = (r8d + eax);
    r9d = (r9d + ecx);
    eax = (eax + edi);
    r10 = (r10 - 1);
    if ((r10 != 1)) goto loc_ADDR;
    loc_ADDR:
    if ((edx >= ebx /* signed */)) goto loc_ADDR;
    loc_ADDR:
    edx = (edx * r11d);
    esi = (esi + edx);
    loc_ADDR:
    eax = (r9 + (r8 * 1));
    esi = (esi + eax);
    loc_ADDR:
    r11d = (r11d + 1);
    edi = (edi + 2);
    if ((r11d < ebx /* signed */)) goto loc_ADDR;
    loc_ADDR:
    rdi = var_30;
    xmm2 = (xmm2 + xmm4);
    rbp = var_28;
    xmm0 = xmm2;
    xmm0 = (xmm0 >> 8);
    xmm2 = (xmm2 + xmm0);
    xmm0 = xmm2;
    xmm0 = (xmm0 >> 4);
    xmm2 = (xmm2 + xmm0);
    eax = xmm2;
    eax = (eax + esi);
    rsi = *(u64*)(rsp);
    rsp = (rsp + 8);
    rbx = *(u64*)(rsp);
    rsp = (rsp + 8);
    return;
    loc_ADDR:
    eax = esi;
    rsi = *(u64*)(rsp);
    rsp = (rsp + 8);
    rbx = *(u64*)(rsp);
    rsp = (rsp + 8);
    return;
}
