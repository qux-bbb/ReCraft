// L1 structure recovery (23 blocks)
void switcher_big_l1(void) {
    if ((ecx > 0xADDR)) {
        eax = 0xADDR;
        return;
    }
    rax = (int64*)(ecx);
    rdx = 0xADDR;
    ecx = *(u32*)(((rdx + (rax * 4)) + 0xADDR));
    rcx = (rcx + rdx);
    __asm("jmp rcx");
    /* jump table: 20 targets, selector=ecx */
    switch (ecx) {
        case 0: goto loc_ADDR;
        case 1: goto loc_ADDR;
        case 2: goto loc_ADDR;
        case 3: goto loc_ADDR;
        case 4: goto loc_ADDR;
        case 5: goto loc_ADDR;
        case 6: goto loc_ADDR;
        case 7: goto loc_ADDR;
        case 8: goto loc_ADDR;
        case 9: goto loc_ADDR;
        case 10: goto loc_ADDR;
        case 11: goto loc_ADDR;
        case 12: goto loc_ADDR;
        case 13: goto loc_ADDR;
        case 14: goto loc_ADDR;
        case 15: goto loc_ADDR;
        case 16: goto loc_ADDR;
        case 17: goto loc_ADDR;
        case 18: goto loc_ADDR;
        case 19: goto loc_ADDR;
    }
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
    loc_ADDR:
    eax = 0xADDR;
    return;
}
