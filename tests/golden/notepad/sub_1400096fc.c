// L1 structure recovery (13 blocks)
void sub_1400096fc_l1(void) {
    rbx = r9;
    rsi = r8;
    rdi = rdx;
    if (r9 != 0) {
        if (rcx != 0) {
            if (rsi == 0) {
                r8 = rdi;
                edx = 0;
                sub_1400029dc();
                if (rsi == 0) {
                    _o__errno();
                    ebx = 0xADDR;
                    *(u32*)(rax) = ebx;
                    sub_140002934();
                    eax = ebx;
                } else if (rdi >= rbx) {
                    eax = 0xADDR;
                } else {
                    _o__errno();
                    ebx = 0xADDR;
                    *(u32*)(rax) = ebx;
                    sub_140002934();
                    eax = ebx;
                    return;
                }
            } else if (rdi >= rbx) {
                r8 = rbx;
                rdx = rsi;
                sub_1400274ac();
                eax = 0;
            }
        } else {
            _o__errno();
            ebx = 0xADDR;
            *(u32*)(rax) = ebx;
            sub_140002934();
            eax = ebx;
            return;
        }
    } else {
        eax = 0;
        return;
    }
    return;
}
