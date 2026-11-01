// L1 structure recovery (5 blocks)
void sub_140022fe8_l1(void) {
    rbx = rcx;
    if (dword_ADDR > 4) {
        r10 = 0xADDR;
        if ((qword_ADDR & 0xADDR) != 0) {
            rax = qword_ADDR;
            rax &= r10;
            if (rax == qword_ADDR) {
                var_30 = ax;
                var_32 = ax;
                var_34 = ax;
                var_36 = ax;
                var_38 = ax;
                var_3a = ax;
                var_3c = ax;
                var_3e = ax;
                var_40 = ax;
                var_42 = ax;
                var_44 = ax;
                var_46 = ax;
                var_48 = ax;
                var_4a = ax;
                var_4c = ax;
                var_4e = ax;
                eax = (uint32)(*(u16*)(rcx));
                var_50 = ax;
                rax = (qword_ADDR + 1);
                var_54 = r9d;
                qword_ADDR = rax;
                var_60 = rax;
                var_1e0 = &var_54;
                var_1d0 = &var_58;
                var_1c0 = &var_5c;
                var_1b0 = &var_30;
                var_1a0 = &var_32;
                var_190 = &var_34;
                var_180 = &var_36;
                var_170 = &var_38;
                var_160 = &var_3a;
                var_150 = &var_3c;
                var_140 = &var_3e;
                var_130 = &var_40;
                var_120 = &var_42;
                rax = &var_44;
                var_58 = r8d;
                var_5c = edx;
                var_1e8 = r11;
                var_1d8 = r11;
                var_1c8 = r11;
                var_1b8 = 2;
                var_1a8 = 2;
                var_198 = 2;
                var_188 = 2;
                var_178 = 2;
                var_168 = 2;
                var_158 = 2;
                var_148 = 2;
                var_138 = 2;
                var_128 = 2;
                var_110 = rax;
                rax = &var_46;
                var_118 = 2;
                var_100 = rax;
                rax = &var_48;
                var_108 = 2;
                var_f0 = rax;
                r9d = 0;
                rax = &var_4a;
                var_f8 = 2;
                var_e0 = rax;
                r8d = 0;
                rax = &var_4c;
                var_e8 = 2;
                var_d0 = rax;
                var_c0 = &var_4e;
                var_b0 = &var_50;
                var_a0 = &var_60;
                var_90 = off_140036178;
                rax = &var_70;
                var_28 = rax;
                var_20 = 0xADDR;
                var_d8 = 2;
                var_c8 = 2;
                var_b8 = 2;
                var_a8 = 8;
                var_98 = 0xADDR;
                sub_1400012dc(off_1400341f8, off_14002e9a7, 0);
            }
        }
    }
    xmm0 = 0;
    eax = 0;
    *(__m128*)(rbx) = xmm0;
    *(__m128*)((rbx + 0xADDR)) = xmm0;
    *(u16*)((rbx + 0xADDR)) = ax;
    return;
}
