// L1 structure recovery (8 blocks)
void switcher_l1(void) {
    eax = var_4;
    if (eax > 4) {
        eax = -1;
        return;
    }
    /* jump table: 5 targets, selector=index */
    switch (index) {
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
    }
}
