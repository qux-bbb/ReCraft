// L1 structure recovery (11 blocks)
void sub_140006420_l1(void) {
    ebx = ecx;
    if (ecx < 0 /* signed */) {
        if (ebx != 0xADDR) {
            if (qword_ADDR == 0) {
                ebx |= 0xADDR;
                eax = ebx;
            } else {
                sub_140028010();
                if (eax == 0) {
                    ebx |= 0xADDR;
                    eax = ebx;
                    return;
                } else if (eax == 0xADDR) {
                    ebx |= 0xADDR;
                    eax = ebx;
                    return;
                } else if (eax > 0 /* signed */) {
                    eax = (uint32)(ax);
                    eax |= 0xADDR;
                }
            }
        } else {
            eax = 0xADDR;
        }
    } else {
        eax = 0;
    }
    return;
}
