// L1 structure recovery (16 blocks)
void nested_loops_l1(void) {
    __asm("push rbx");
    __asm("push rsi");
    esi = (esi ^ esi);
    r11d = (r11d ^ r11d);
    ebx = ecx;
    __asm("if (jle 0xADDR /* unmodelled flags */) goto 0xADDR");
    __asm("movdqa xmm3, [0xADDR]");
    __asm("xorps xmm4, xmm4");
    var_28 = rbp;
    __asm("xorps xmm2, xmm2");
    ebp = *(u32*)(0xADDR);
    var_30 = rdi;
    edi = (edi ^ edi);
    edx = (edx ^ edx);
    __asm("movd xmm1, r11d");
    __asm("pshufd xmm1, xmm1, 0xADDR");
    if ((ebx < 8)) goto loc_ADDR;
    loc_ADDR:
    if ((ebp < 2 /* signed */)) goto loc_ADDR;
    loc_ADDR:
    ecx = ebx;
    ecx = (ecx & 0xADDR);
    loc_ADDR:
    __asm("movd xmm0, edx");
    eax = (rdx + 4);
    __asm("pshufd xmm0, xmm0, 0xADDR");
    edx = (edx + 8);
    __asm("paddd xmm0, xmm3");
    __asm("pmulld xmm0, xmm1");
    __asm("paddd xmm4, xmm0");
    __asm("movd xmm0, eax");
    __asm("pshufd xmm0, xmm0, 0xADDR");
    __asm("paddd xmm0, xmm3");
    __asm("pmulld xmm0, xmm1");
    __asm("paddd xmm2, xmm0");
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
    __asm("imul eax, r11d");
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
    __asm("imul edx, r11d");
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
    __asm("paddd xmm2, xmm4");
    rbp = var_28;
    __asm("movdqa xmm0, xmm2");
    __asm("psrldq xmm0, 0xADDR");
    __asm("paddd xmm2, xmm0");
    __asm("movdqa xmm0, xmm2");
    __asm("psrldq xmm0, 0xADDR");
    __asm("paddd xmm2, xmm0");
    __asm("movd eax, xmm2");
    eax = (eax + esi);
    __asm("pop rsi");
    __asm("pop rbx");
    return;
    loc_ADDR:
    eax = esi;
    __asm("pop rsi");
    __asm("pop rbx");
    return;
}
