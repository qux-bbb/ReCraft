// L1 structure recovery (28 blocks)
void sub_14000a630_l1(void) {
    rsi = r9;
    rbx = rcx;
    if (edx == 5) {
        sub_14000accc();
        eax = 0;
    } else if (edx == 0xADDR) {
        edx = 0;
        sub_1400029dc(&var_f0, 0, (rdx + 0xADDR));
        edx = 0;
        sub_1400029dc(&var_40, 0, (rdx + 0xADDR));
        BeginPaint(rbx, &var_f0);
        xmm1 = var_50;
        var_48 = 0;
        esi = 0xADDR;
        xmm0 = var_40;
        rdi = rax;
        var_a0 = xmm1;
        xmm1 = var_70;
        var_90 = xmm0;
        xmm0 = var_60;
        var_c0 = xmm1;
        var_b0 = xmm0;
        xmm0 = var_80;
        var_d0 = xmm0;
        CreateSolidBrush(esi);
        rbx = rax;
        FillRect(rdi, &var_fc, rax);
        SetBkColor(rdi, esi);
        SelectObject(rdi, qword_ADDR);
        r8d = -1;
        var_20 = 0xADDR;
        DrawTextW(rdi, qword_ADDR, (-1 | -1), off_140035508, 0xADDR);
        if (rbx != 0) {
            DeleteObject(rbx);
        }
        sub_14000a048(&var_90);
        eax = 0;
        return;
    } else if (edx == 0xADDR) {
        rax = qword_ADDR;
        if (*(u64*)((r9 + 0xADDR)) == rax) {
            ebx = *(u32*)((r9 + 0xADDR));
            edi = 0;
            edx = 0;
            r8 = *(u64*)((rsi + 0xADDR));
            rcx = qword_ADDR;
            dl = (uint8)((byte_ADDR != dil));
            eax = (rdi + 2);
            var_28 = 0;
            var_20 = 0;
            __asm("cmova edx, eax");
            ImageList_Draw(rcx, rdx, r8, 0);
            if (((ebx >> 4) & 1) != 0) {
                var_30 = edi;
                var_38 = edi;
                ImageList_GetIconSize(qword_ADDR, &var_30, &var_38);
                var_e8 = var_30;
                ecx = var_38;
                var_ec = ecx;
                var_e0 = rdi;
                DrawFocusRect(*(u64*)((rsi + 0xADDR)), &var_e0);
            }
        }
        eax = 1;
    } else if (edx != 0xADDR) {
        DefWindowProcW(rbx, rdx, r8, r9);
    } else if (r9 == 0) {
        if (r8 != 2) {
            DefWindowProcW(rbx, rdx, r8, r9);
            return;
        }
        sub_14000aac4();
        eax = 0;
        return;
    }
    rax = (r8 >> 0xADDR);
    if (ax == 0) {
        ecx = (uint32)(r8w);
        switch (ecx) {
            case 0xADDR: {
                sub_14001fd94("ms-windows-store://pdp/?PFN=Microsoft.WindowsNotepad_8wekyb3d8bbwe");
                eax = 0;
                return;
            }
            case 0xADDR: {
                rax = qword_ADDR;
                *(u8*)((rax + 0xADDR)) = 1;
                sub_1400225a4(qword_ADDR);
                rdx = qword_ADDR;
                rdx = *(u64*)((rdx + 0xADDR));
                sub_1400223a0(&var_38, rdx);
                rbx = var_38;
                if (rbx == 0) {
                    eax = 0;
                    return;
                }
                sub_140023c00(rbx);
                sil = al;
                if (rbx != 0) {
                    CoTaskMemFree(rbx);
                }
                if (sil == 0) {
                    eax = 0;
                    return;
                }
                PostQuitMessage(0);
                eax = 0;
                return;
            }
            case 0xADDR: {
                sub_14000aac4();
                eax = 0;
                return;
            }
        }
        DefWindowProcW(rbx, rdx, r8, r9);
        return;
    }
    return;
}
