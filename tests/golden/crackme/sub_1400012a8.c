// L1 structure recovery (22 blocks)
void sub_1400012a8_l1(void) {
    sub_140001474(1);
    if (al == 0) {
        sub_1400017a0(7);
        sub_1400017a0(7);
        ecx = ebx;
        sub_140008044(ebx);
        ecx = ebx;
        sub_140007ffc(ebx);
    }
    sil = 0;
    var_20 = sil;
    sub_140001438();
    bl = al;
    ecx = dword_ADDR;
    if (ecx == 1) {
        sub_1400017a0(7);
        ecx = ebx;
        sub_140008044(ebx);
        ecx = ebx;
        sub_140007ffc(ebx);
        return;
    }
    if (ecx != 0) {
        sil = 1;
        var_20 = sil;
        sub_1400015d4(bl);
        sub_140001784();
        rbx = rax;
        if (*(u64*)(rax) != 0) {
            sub_14000153c(rax);
            if (al != 0) {
                r8d = 0;
                edx = (r8 + 2);
                ecx = 0;
                rax = *(u64*)(rbx);
                (*qword_ADDR)();
            }
        }
        sub_14000178c();
        rbx = rax;
        if (*(u64*)(rax) != 0) {
            sub_14000153c(rax);
            if (al != 0) {
                sub_140008008(*(u64*)(rbx));
            }
        }
        sub_140007c98();
        rdi = rax;
        sub_1400080c0();
        rbx = *(u64*)(rax);
        sub_1400080b8();
        sub_140001010(*(u32*)(rax), rbx, rdi);
        ebx = eax;
        sub_1400018f4();
        if (al == 0) {
            ecx = ebx;
            sub_140008044(ebx);
            ecx = ebx;
            sub_140007ffc(ebx);
            return;
        }
        if (sil == 0) {
            sub_140007fec();
        }
        edx = 0;
        sub_1400015f8(1);
        eax = ebx;
        return;
    }
    dword_ADDR = 1;
    sub_140007d28(off_1400182c8, off_140018300);
    if (eax == 0) {
        sub_140007cf0(off_1400182b0, off_1400182c0);
        dword_ADDR = 2;
        sub_1400015d4(bl);
        sub_140001784();
        rbx = rax;
        if (*(u64*)(rax) != 0) {
            sub_14000153c(rax);
            if (al != 0) {
                r8d = 0;
                edx = (r8 + 2);
                ecx = 0;
                rax = *(u64*)(rbx);
                (*qword_ADDR)();
            }
        }
        sub_14000178c();
        rbx = rax;
        if (*(u64*)(rax) != 0) {
            sub_14000153c(rax);
            if (al != 0) {
                sub_140008008(*(u64*)(rbx));
            }
        }
        sub_140007c98();
        rdi = rax;
        sub_1400080c0();
        rbx = *(u64*)(rax);
        sub_1400080b8();
        sub_140001010(*(u32*)(rax), rbx, rdi);
        ebx = eax;
        sub_1400018f4();
        if (al == 0) {
            ecx = ebx;
            sub_140008044(ebx);
            ecx = ebx;
            sub_140007ffc(ebx);
            return;
        }
        if (sil == 0) {
            sub_140007fec();
        }
        edx = 0;
        sub_1400015f8(1);
        eax = ebx;
        return;
    }
    eax = 0xADDR;
    return;
}
