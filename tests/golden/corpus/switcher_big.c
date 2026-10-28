// L1 structure recovery (23 blocks)
void switcher_big_l1(void) {
    if ((ecx > 0xADDR)) {
        eax = 0xADDR;
        return;
    }
    rax = (int64)(ecx);
    rdx = 0xADDR;
    ecx = *(u32*)(((rdx + (rax * 4)) + 0xADDR));
    rcx += rdx;
    __asm("jmp rcx");
    /* jump table: 20 targets, selector=ecx */
    switch (ecx) {
        case 0: {
            eax = 0xADDR;
            return;
        }
        case 1: {
            eax = 0xADDR;
            return;
        }
        case 2: {
            eax = 0xADDR;
            return;
        }
        case 3: {
            eax = 0xADDR;
            return;
        }
        case 4: {
            eax = 0xADDR;
            return;
        }
        case 5: {
            eax = 0xADDR;
            return;
        }
        case 6: {
            eax = 0xADDR;
            return;
        }
        case 7: {
            eax = 0xADDR;
            return;
        }
        case 8: {
            eax = 0xADDR;
            return;
        }
        case 9: {
            eax = 0xADDR;
            return;
        }
        case 10: {
            eax = 0xADDR;
            return;
        }
        case 11: {
            eax = 0xADDR;
            return;
        }
        case 12: {
            eax = 0xADDR;
            return;
        }
        case 13: {
            eax = 0xADDR;
            return;
        }
        case 14: {
            eax = 0xADDR;
            return;
        }
        case 15: {
            eax = 0xADDR;
            return;
        }
        case 16: {
            eax = 0xADDR;
            return;
        }
        case 17: {
            eax = 0xADDR;
            return;
        }
        case 18: {
            eax = 0xADDR;
            return;
        }
        case 19: {
            eax = 0xADDR;
            return;
        }
    }
}
