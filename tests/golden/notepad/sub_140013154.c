// L1 structure recovery (1620 blocks)
void sub_140013154_l1(void) {
// some loops keep labels (an inner loop exits its parent)
    rbp = &var_100;
    xmm1 = 0;
    eax = 0;
    rdi = rcx;
    var_990 = rax;
    xmm0 = 0;
    var_998 = eax;
    r12d = r9d;
    rect = xmm0;
    rsi = r8;
    r14 = rdx;
    var_970 = xmm1;
    var_980 = xmm1;
    RegisterWindowMessageW("commdlg_FindReplace");
    ebx = 0;
    dword_ADDR = eax;
    if (eax == 0) {
        eax = 0;
    } else {
        RegisterWindowMessageW("commdlg_help");
        dword_ADDR = eax;
        if (eax == 0) {
            eax = 0;
            return;
        }
        GetDC(0);
        r15 = rax;
        if (rax == 0) {
            eax = 0;
            return;
        }
        sub_140012f3c(r14);
        if (eax == 0) {
            eax = 0;
            return;
        }
        LoadCursorW(0, 0xADDR);
        qword_ADDR = rax;
        LoadCursorW(0, 0xADDR);
        qword_ADDR = rax;
        LoadAcceleratorsW(r14, "GlobalAcc");
        qword_ADDR = rax;
        LoadAcceleratorsW(r14, "MainAcc");
        qword_ADDR = rax;
        if (qword_ADDR == rbx) {
            eax = 0;
            return;
        } else if (rax == 0) {
            eax = 0;
            return;
        }
        if (qword_ADDR == 0) {
            eax = 0;
            return;
        }
        sub_14001e1dc(rdi, r14);
        ebx = 2;
        qword_ADDR = rdi;
        xmm0 = 0;
        qword_ADDR = r14;
        rax = sub_140021020;
        dword_ADDR = 0xADDR;
        xmmword_ADDR = xmm0;
        qword_ADDR = r14;
        qword_ADDR = rax;
        qword_ADDR = 0xADDR;
        GetLocaleInfoW(0xADDR, (rbx + 0xADDR), &var_198, ebx);
        if (var_198 != 0xADDR) {
            eax = 0xADDR;
            dword_ADDR = 0xADDR;
            dword_ADDR = eax;
            dword_ADDR = eax;
            eax = 0xADDR;
        } else {
            eax = 0xADDR;
            dword_ADDR = 0xADDR;
            dword_ADDR = eax;
            dword_ADDR = eax;
            eax = 0xADDR;
        }
        dword_ADDR = eax;
        dword_ADDR = eax;
        eax = 0;
        word_ADDR = ax;
        word_ADDR = ax;
        byte_ADDR = al;
        byte_ADDR = al;
        sub_140012a2c();
        sub_1400221ac(&var_2b8);
        rdx = off_14002a790;
        rdi = rax;
        sub_140001c90(0xADDR);
        if (rax != 0) {
            sub_140021fc4(rax, rdi);
        }
        rcx = qword_ADDR;
        edx = 1;
        qword_ADDR = rax;
        if (rcx != 0) {
            rax = *(u64*)(rcx);
            rax = *(u64*)(rax);
            sub_140028010();
            rax = qword_ADDR;
        }
        rcx = var_2b8;
        if (rcx != 0) {
            CoTaskMemFree(*(u64*)((rbp + 0xADDR)));
            rax = qword_ADDR;
        }
        if (rax == 0) {
            loc_ADDR:
            eax = 0;
        } else {
            sub_1400223a0(&var_1a8, *(u64*)((rax + 0xADDR)));
            rdi = var_1a8;
            eax = 0;
            if (rdi != 0) {
                RegisterApplicationRestart(rdi, 0);
                if (eax < 0 /* signed */) {
                    r9d = eax;
                    sub_140007cf0(var_1108, 0xADDR, "shell\\osshell\\accesory\\notepad\\nprestart.cpp");
                }
                if (rdi == 0) goto loc_ADDR;
            }
        }
        var_58 = rax;
        var_50 = r14;
        var_48 = rax;
        var_40 = rax;
        var_38 = dword_ADDR;
        var_30 = dword_ADDR;
        var_28 = dword_ADDR;
        eax = dword_ADDR;
        var_20 = eax;
        CreateWindowExW(0, "Notepad", off_14002b210, 0xADDR, dword_ADDR, dword_ADDR, dword_ADDR, dword_ADDR, rax, rax, r14, rax);
        r13d = 0;
        qword_ADDR = rax;
        qword_ADDR = rax;
        if (rax == 0) {
            eax = 0;
            return;
        }
        ecx = 0;
        sub_14000c980(0);
        sub_140026ee8();
        if (eax < 0 /* signed */) {
            eax = 0;
            qword_ADDR = rax;
        }
        if (dword_ADDR != 0xADDR && dword_ADDR != 0xADDR) {
            eax = 0;
            xmm0 = 0;
            var_990 = rax;
            var_980 = xmm0;
            edi = (rax + 0xADDR);
            var_998 = eax;
            var_970 = edi;
            GetWindowPlacement(qword_ADDR, &var_970);
            if (eax != 0) {
                eax = dword_ADDR;
                xmm0 = 0;
                dword_ADDR = eax;
                eax -= var_98c;
                eax += var_994;
                dword_ADDR = eax;
                eax = dword_ADDR;
                dword_ADDR = eax;
                eax -= var_990;
                eax += var_998;
                dword_ADDR = eax;
                xmmword_ADDR = xmm0;
                qword_ADDR = r13;
                dword_ADDR = edi;
            }
            rbx = qword_ADDR;
            rcx = -1;
            SetThreadDpiAwarenessContext((rcx | -1));
            rdi = rax;
            SetWindowPlacement(rbx, off_140035820);
            SetThreadDpiAwarenessContext(rdi);
            ebx = 2;
        }
        DragAcceptFiles(qword_ADDR, 1);
        GetClientRect(qword_ADDR, &rect);
        eax = dword_ADDR;
        eax = -(eax);
        r8d = (((eax != 0) ? 0xADDR : 0) + 0xADDR);
        sub_1400125a0(rect.right, rect.bottom, r8, r14);
        if (rax == 0) {
            eax = 0;
            return;
        }
        sub_14001f318(rax);
        r13d = 1;
        SendMessageW(qword_ADDR, 0xADDR, r13d, r13d);
        SendMessageW(qword_ADDR, 0xADDR, r13d, 0);
        SendMessageW(qword_ADDR, 0xADDR, 0, "NPCTXT");
        eax = dword_ADDR;
        eax = -(eax);
        ecx = (((eax != 0) ? 0xADDR : 0) + 0xADDR);
        CreateStatusWindowW((ecx + 0xADDR), off_14002b210, qword_ADDR, 0xADDR);
        rdi = rax;
        sub_14001ec2c(off_140036698, r13b, r8, r9);
        sub_14000b3e4(rcx);
        if (rdi == 0) {
            eax = 0;
            return;
        }
        rcx = qword_ADDR;
        if (rcx != 0) {
            sub_140023d18(rcx);
            DestroyWindow(qword_ADDR);
        }
        rdx = qword_ADDR;
        qword_ADDR = rdi;
        sub_140023d8c(rdi, qword_ADDR);
        sub_14000f9f0();
        xmm0 = 0;
        rect2 = xmm0;
        GetClientRect(qword_ADDR, &rect2);
        ecx = rect2.right;
        ecx -= rect2.left;
        sub_14001f420(rcx);
        SendMessageW(qword_ADDR, 0xADDR, (int64)(dword_ADDR), 0);
        GetDpiForWindow(qword_ADDR);
        MulDiv(dword_ADDR, eax, 0xADDR);
        eax = -(eax);
        dword_ADDR = eax;
        CreateFontIndirectW(off_140035650);
        qword_ADDR = rax;
        SelectObject(r15, rax);
        rdi = rax;
        GetTextFaceW(r15, 0xADDR, &var_db0);
        SelectObject(r15, rdi);
        lstrcmpiW(&var_db0, off_14003566c);
        r14d = 0;
        if (eax == 0) {
            rax = qword_ADDR;
        } else {
            EnumFontsW(r15, off_14003566c, 0xADDR, off_140035650);
            DeleteObject(qword_ADDR);
            CreateFontIndirectW(off_140035650);
            qword_ADDR = rax;
        }
        r9d = 0;
        SendMessageW(qword_ADDR, (r9 + 0xADDR), rax, 0);
        ReleaseDC(0, r15);
        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
        qword_ADDR = rax;
        eax = dword_ADDR;
        dword_ADDR = eax;
        sub_14000fe3c(rcx);
        sub_14000f9f0();
        ShowWindow(qword_ADDR, r12d);
        SetCursor(qword_ADDR);
        edi = 0xADDR;
        while (1) {
            if (*(u16*)(rsi) == di) {
                rsi += rbx;
                continue;
            }
            if (*(u16*)(rsi) != 9) break;
        }
        dword_ADDR = r14d;
        sub_14001f57c("/", rsi);
        if (eax != 0) {
            sub_14001f57c("/", rsi);
            if (eax != 0) {
                eax = dword_ADDR;
            } else {
                eax = ebx;
                dword_ADDR = ebx;
            }
            if (eax != 0) {
                loc_ADDR:
                rsi += 4;
                while (1) {
                    if (*(u16*)(rsi) == di) {
                        rsi += rbx;
                        continue;
                    }
                    if (*(u16*)(rsi) != 9) break;
                }
            }
        } else {
            dword_ADDR = r13d;
            goto loc_ADDR;
        }
        edx = 0;
        sub_1400029dc(&var_3a0, 0, (rdx + 0xADDR));
        GetStartupInfoW(&var_3a0);
        if ((var_3dc & 0xADDR) != 0) {
            if (*(u16*)(rsi) == 0xADDR) {
                loc_ADDR:
                edi = 0xADDR;
                r8d = edi;
                edx = 0;
                sub_1400029dc(off_1400355d0);
                rax = qword_ADDR;
                rcx = qword_ADDR;
                qword_ADDR = rax;
                dword_ADDR = edi;
                var_194 = r14d;
                var_18c = r14d;
                SendMessageW(qword_ADDR, (rdi + 0xADDR), &var_194, &var_18c);
                SendMessageW(qword_ADDR, (rdi + 0xADDR), var_194, var_18c);
                SendMessageW(qword_ADDR, (rdi + 0xADDR), 0, 0);
                GetKeyboardLayout(0);
                ax &= 0xADDR;
                if (ax == 0xADDR) {
                    SendMessageW(qword_ADDR, 0xADDR, r13, r13);
                }
                var_130 = r14d;
                r15d = r14d;
                var_1b8 = r14;
                var_188 = r14d;
                var_138 = r14;
                var_118 = r14d;
                do {
                    eax = 0;
                    __asm("lock cmpxchg [0xADDR], r13d");
                } while (dword_ADDR != eax);
                eax = dword_ADDR;
                r12 = off_140029260;
                edi = -1;
                rsi = off_140034000;
                var_154 = edi;
                ebx = 0xADDR;
                if (eax != 0) {
                    eax += r13d;
                    r12d = 2;
                    dword_ADDR = eax;
                } else {
                    var_a8 = r14d;
                    var_150 = r14d;
                    GetProcessHeap();
                    HeapAlloc(rax, 0, ebx);
                    rbx = rax;
                    eax = 0;
                    if (rbx == 0) {
                        ebx = 4;
                        r13 = off_1400360c0;
                        rdi = r13;
                        r14d = 0;
                        eax = (rbx - 3);
                        do {
                            rcx = *(u64*)(rdi);
                            if (rcx != 0) {
                                FreeLibrary(*(u64*)(rdi));
                                eax = 1;
                            }
                            rdi += 0xADDR;
                            rbx -= rax;
                        } while (rbx != 0);
                        edx = 0;
                        sub_1400029dc(r13, 0, (rbx + 0xADDR));
                        rcx = r12;
                        rax = rsi;
                        r12d = (rbx + 2);
                        edx = r12d;
                        edi = (r12 + 0xADDR);
                        do {
                            xmm0 = *(__m128*)(rcx);
                            xmm1 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)(rax) = xmm0;
                            xmm0 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm1;
                            xmm1 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm0;
                            xmm0 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm1;
                            xmm1 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm0;
                            xmm0 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm1;
                            xmm1 = *(__m128*)((rcx + 0xADDR));
                            rcx += rdi;
                            *(__m128*)((rax + 0xADDR)) = xmm0;
                            rax += rdi;
                            *(__m128*)((rax - 0xADDR)) = xmm1;
                            rdx -= 1;
                        } while (rdx != 0);
                        xmm0 = *(__m128*)(rcx);
                        xmm1 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)(rax) = xmm0;
                        xmm0 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm1;
                        xmm1 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm0;
                        xmm0 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm1;
                        xmm1 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm0;
                        xmm0 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm1;
                        *(__m128*)((rax + 0xADDR)) = xmm0;
                    } else {
                        r8d = eax;
                        r15 = (rbx + 7);
                        r13d = edi;
                        r12 = off_14002da82;
                        r11d = eax;
                        ecx = (rax + 1);
                        esi = eax;
                        r14d = (rax + 0xADDR);
                        do {
                            eax = (uint32)(*(u8*)((r12 - 1)));
                            r10d = (uint32)(*(u8*)((r12 - 2)));
                            r9d = (uint32)(*(u8*)((r12 + 2)));
                            r10d <<= 8;
                            r10d |= eax;
                            r9d <<= 8;
                            eax = (uint32)(*(u8*)(r12));
                            r12 += 8;
                            r10d <<= 8;
                            r10d |= eax;
                            eax = (uint32)(*(u8*)((r12 - 7)));
                            r10d <<= 8;
                            r10d |= eax;
                            eax = (uint32)(*(u8*)((r12 - 5)));
                            r9d |= eax;
                            esi ^= r10d;
                            eax = (uint32)(*(u8*)((r12 - 4)));
                            r9d <<= 8;
                            r9d |= eax;
                            eax = (uint32)(*(u8*)((r12 - 3)));
                            r9d <<= 8;
                            r9d |= eax;
                            r11d ^= r9d;
                            r11d ^= esi;
                            r11d ^= 0xADDR;
                            eax = (r11 + 0xADDR);
                            eax = ror(eax, 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = r11d;
                            eax = ror(eax, 0xADDR);
                            edx += eax;
                            esi ^= edx;
                            eax = (rsi + 0xADDR);
                            eax = ror(eax, 9);
                            edx = (eax * 0xADDR);
                            eax = esi;
                            eax = ror(eax, 0xADDR);
                            edx -= eax;
                            r11d ^= edx;
                            edx = (r11d * 0xADDR);
                            eax = (r11d >> 0xADDR);
                            edx -= eax;
                            edx -= 0xADDR;
                            esi ^= edx;
                            eax = (esi ^ 0xADDR);
                            eax = ror(eax, 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = esi;
                            eax = ror(eax, 0xADDR);
                            edx -= eax;
                            eax = 0xADDR;
                            r11d ^= edx;
                            edx = (r11d ^ 0xADDR);
                            eax -= edx;
                            esi ^= eax;
                            eax = esi;
                            edi = esi;
                            eax ^= 0xADDR;
                            edi = ror(edi, 6);
                            edx = (eax * 0xADDR);
                            edi ^= edx;
                            r11d ^= edi;
                            edi = 0xADDR;
                            eax = (r11 + 0xADDR);
                            eax = ror(eax, 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = r11d;
                            eax = ror(eax, 0xADDR);
                            edx += eax;
                            esi ^= edx;
                            eax = (rsi + 0xADDR);
                            eax = ror(eax, 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = esi;
                            eax = ror(eax, 0xADDR);
                            edx -= eax;
                            r11d ^= edx;
                            eax = r11d;
                            eax ^= edi;
                            eax = ror(eax, 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = r11d;
                            eax = ror(eax, 0xADDR);
                            edx ^= eax;
                            esi ^= edx;
                            eax = (esi ^ 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = (esi >> 0xADDR);
                            edx ^= eax;
                            edx ^= r11d;
                            eax = edx;
                            eax = ~(eax);
                            eax = ror(eax, 5);
                            eax += 0xADDR;
                            eax *= 0xADDR;
                            esi ^= eax;
                            r11d = (rsi - 0xADDR);
                            r11d ^= edx;
                            r11d ^= edi;
                            eax = (r11d ^ 0xADDR);
                            eax = ror(eax, 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = (r11d >> 2);
                            edx += eax;
                            esi ^= edx;
                            eax = (rsi - 0xADDR);
                            eax = ror(eax, 6);
                            edx = (eax * 0xADDR);
                            eax = esi;
                            eax = ror(eax, 0xADDR);
                            edx += eax;
                            r11d ^= edx;
                            eax = r11d;
                            edi = r11d;
                            eax ^= 0xADDR;
                            edi = ror(edi, 9);
                            edx = (eax * 0xADDR);
                            edi += edx;
                            esi ^= edi;
                            eax = (esi ^ 0xADDR);
                            eax = ror(eax, 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = esi;
                            eax = ror(eax, 0xADDR);
                            edx += eax;
                            r11d ^= edx;
                            edi = r11d;
                            edi ^= esi;
                            edi ^= 0xADDR;
                            eax = edi;
                            eax = ror(eax, 3);
                            edx = ((eax * 0xADDR) - 0xADDR);
                            r11d ^= edx;
                            eax = (r11 - 0xADDR);
                            eax = ror(eax, 1);
                            edx = (eax * 0xADDR);
                            eax = r11d;
                            eax = ror(eax, 6);
                            edx -= eax;
                            edi ^= edx;
                            eax = (rdi - 0xADDR);
                            eax = ror(eax, 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = edi;
                            eax = ror(eax, 0xADDR);
                            edx += eax;
                            r11d ^= edx;
                            eax = (r11 - 0xADDR);
                            eax = ror(eax, 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = r11d;
                            eax = ror(eax, 0xADDR);
                            edx -= eax;
                            edi ^= edx;
                            eax = (edi ^ 0xADDR);
                            edx = (eax * 0xADDR);
                            eax = (edi >> 3);
                            edx ^= eax;
                            r11d ^= edx;
                            eax = (r11d ^ 0xADDR);
                            eax = ror(eax, 0xADDR);
                            esi = (eax * 0xADDR);
                            eax = r11d;
                            eax = ror(eax, 0xADDR);
                            r11d ^= r13d;
                            r13d = r9d;
                            esi ^= eax;
                            esi ^= edi;
                            edi = r11d;
                            esi ^= r8d;
                            edi <<= 0xADDR;
                            *(u8*)((r15 - 4)) = sil;
                            edx = esi;
                            *(u8*)(r15) = r11b;
                            eax = (esi >> 8);
                            r15 += 8;
                            edx <<= 0xADDR;
                            edx |= eax;
                            eax = r11d;
                            *(u8*)((r15 - 0xADDR)) = dl;
                            r8d = edx;
                            eax >>= 8;
                            edi |= eax;
                            r8d <<= 0xADDR;
                            *(u8*)((r15 - 9)) = dil;
                            eax = (edi << 0xADDR);
                            edx >>= 8;
                            r8d |= edx;
                            edi >>= 8;
                            *(u8*)((r15 - 0xADDR)) = r8b;
                            eax |= edi;
                            *(u8*)((r15 - 0xADDR)) = al;
                            r8d = ror(r8d, 8);
                            *(u8*)((r15 - 0xADDR)) = r8b;
                            r8d = r10d;
                            eax = ror(eax, 8);
                            *(u8*)((r15 - 0xADDR)) = al;
                            r14 -= rcx;
                        } while (r14 != 0);
                        r14d = var_150;
                        edx = 0;
                        eax = edx;
                        cl = dl;
                        r13d = edx;
                        r8d = 0xADDR;
                        edi = (rdx + 1);
                        do {
                            cl ^= *(u8*)((rax + rbx));
                            rax += rdi;
                        } while (rax < r8);
                        eax = (uint32)(cl);
                        if (rax == *(u64*)("@")) {
                            *(u8*)(((rbx + r8) - 1)) = dl;
                            rdi = off_1400360c0;
                            sub_1400029dc(rdi, rdx, 0xADDR);
                            rsi = rbx;
                            if (*(u8*)(rbx) == 0xADDR) {
                                GetProcessHeap();
                                HeapFree(rax, 0, rsi);
                            } else {
                                rdx = rbx;
                                rax = -1;
                            }
                            loc_ADDR:
                            r15d = var_118;
                            if (r14d < 0 /* signed */) {
                                r12 = off_140029260;
                                rsi = off_140034000;
                            } else {
                                eax = dword_ADDR;
                                r14d = 0;
                                r13d = (r14 + 1);
                            }
                        } else {
                            GetProcessHeap();
                            HeapFree(rax, 0, rbx);
                            r15d = r14d;
                        }
                    }
                }
                eax = r14d;
                esi = r14d;
                __asm("xchg [0xADDR], eax");
                __asm("mov rax, gs:[0xADDR]");
                ebx = r14d;
                var_1d0 = r14;
                edi = 0xADDR;
                var_1d8 = r14;
                r13d = 0xADDR;
                if (*(u32*)((rax + 0xADDR)) == r14d) {
                    r15d = esi;
                } else {
                    rax = qword_ADDR;
                    sub_140028010();
                    ecx = 0;
                    r14 = rax;
                    if (rax != 0) {
                        var_178 = ecx;
                        rdi = rcx;
                        var_1e0 = rcx;
                        ebx = ecx;
                        r9d = ecx;
                        rax = rcx;
                        while (1) {
                            r8 = rdi;
                            rax = &var_178;
                            edx = r12d;
                            r8 = ((rax == 0) ? rcx : r8);
                            var_20 = rax;
                            rax = qword_ADDR;
                            rcx = r14;
                            sub_140028010(r14, r12d, r8);
                            if (eax != 0) break;
                            GetLastError();
                            if (eax == 0xADDR && rdi == 0) {
                                edi = var_178;
                                GetProcessHeap();
                                HeapAlloc(rax, 0, edi);
                                rdi = rax;
                                sub_140024c8c(&var_1e0);
                                ecx = 0;
                                var_1e0 = rdi;
                                rax = rdi;
                                if (rdi != 0) {
                                    r9d = var_178;
                                    continue;
                                }
                            }
                        }
                        r14d = 0;
                        var_1d0 = rdi;
                        var_1e0 = r14;
                        loc_ADDR:
                        sub_140024c8c(&var_1e0);
                        if (ebx >= 0 /* signed */) {
                            rdi = qword_ADDR;
                            GetCurrentThreadId();
                            ecx = eax;
                            rax = rdi;
                            sub_140028010();
                            ecx = 0;
                            r14 = rax;
                            if (rax != 0) {
                                var_16c = ecx;
                                rdi = rcx;
                                var_1e8 = rcx;
                                ebx = ecx;
                                r9d = ecx;
                                rax = rcx;
                                while (1) {
                                    r8 = rdi;
                                    rax = &var_16c;
                                    edx = r12d;
                                    r8 = ((rax == 0) ? rcx : r8);
                                    var_20 = rax;
                                    rax = qword_ADDR;
                                    rcx = r14;
                                    sub_140028010(r14, r12d, r8);
                                    if (eax != 0) break;
                                    GetLastError();
                                    if (eax == 0xADDR && rdi == 0) {
                                        edi = var_16c;
                                        GetProcessHeap();
                                        HeapAlloc(rax, 0, edi);
                                        rdi = rax;
                                        sub_140024c8c(&var_1e8);
                                        ecx = 0;
                                        var_1e8 = rdi;
                                        rax = rdi;
                                        if (rdi != 0) {
                                            r9d = var_16c;
                                            continue;
                                        }
                                    }
                                }
                                r14d = 0;
                                var_1d8 = rdi;
                                var_1e8 = r14;
                                loc_ADDR:
                                r12d = 0xADDR;
                                loc_ADDR:
                                sub_140024c8c(&var_1e8);
                                if (ebx >= 0 /* signed */) {
                                    rcx = var_1d0;
                                    edi = 1;
                                    if (rcx >= 1) {
                                        sub_140027494(rcx, "WinSta0");
                                        if (eax == 0) {
                                            rcx = var_1d8;
                                            if (rcx >= rdi) {
                                                sub_140027494(rcx, "Default");
                                                if (eax == 0) {
                                                    eax = 0;
                                                    var_168 = eax;
                                                    GetCurrentProcess();
                                                    GetProcessMitigationPolicy(rax, (rdi + 0xADDR), &var_168, (rdi + 3));
                                                    if (eax != 0) {
                                                        eax = 0xADDR;
                                                        esi = (((var_168 & al) == 0) ? edi : esi);
                                                    } else {
                                                        GetLastError();
                                                        ebx = eax;
                                                        if (eax > 0 /* signed */) {
                                                            ebx = (uint32)(ax);
                                                            ebx |= r13d;
                                                        }
                                                        if (ebx >= 0 /* signed */) {
                                                            ebx = r12d;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                GetLastError();
                                ebx = eax;
                                if (eax > 0 /* signed */) {
                                    ebx = (uint32)(ax);
                                    ebx |= r13d;
                                }
                                if (ebx >= 0 /* signed */) {
                                    ebx = 0xADDR;
                                }
                            }
                        }
                    } else {
                        GetLastError();
                        ebx = eax;
                        if (eax > 0 /* signed */) {
                            ebx = (uint32)(ax);
                            ebx |= r13d;
                        }
                        if (ebx >= 0 /* signed */) {
                            ebx = edi;
                        }
                    }
                }
                sub_140024c8c(&var_1d8);
                sub_140024c8c(&var_1d0);
                edx = 4;
                r12d = r14d;
                r12d = ((ebx >= 0 /* signed */) ? r15d : r12d);
                var_108 = r12d;
                LocalAlloc((rdx + 0xADDR), 4);
                sub_140024d08(&var_138, rax);
                rax = var_138;
                var_148 = rax;
                if (rax != 0) {
                    xmm0 = 0;
                    var_164 = r14d;
                    xmm1 = 0;
                    rsi = r14;
                    var_2f0 = xmm0;
                    var_300 = xmm1;
                    GetProcessHeap();
                    HeapAlloc(rax, 8, 0xADDR);
                    edi = 0;
                    var_150 = 0xADDR;
                    rbx = rax;
                    r14 = 0xADDR;
                    rax = 0xADDR;
                    r13d = 0xADDR;
                    if (rbx == 0) {
                        var_d8 = r13d;
                        rbx = rdi;
                        loc_ADDR:
                        edi = r12d;
                    } else {
                        rax += 0xADDR;
                        edx = 0xADDR;
                        *(__m128*)(rbx) = *(__m128*)(rax);
                        *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                        *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                        *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                        *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                        *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                        *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                        *(__m128*)(((rbx + rdx) - 0xADDR)) = *(__m128*)((rax + 0xADDR));
                        xmm0 = *(__m128*)((rax + rdx));
                        *(__m128*)((rbx + rdx)) = xmm0;
                        xmm1 = *(__m128*)(((rax + rdx) + 0xADDR));
                        eax = (rdx + 0xADDR);
                        var_a8 = eax;
                        *(__m128*)(((rbx + rdx) + 0xADDR)) = xmm1;
                        var_118 = eax;
                        GetProcessHeap();
                        edx = (rdi + 8);
                        HeapAlloc(rax, (rdi + 8), edx);
                        r8 = rax;
                        if (rax == 0) {
                            var_d8 = r13d;
                            rsi = rdi;
                            var_78 = edi;
                            r10d = edi;
                            r8 = rdi;
                            edi = r12d;
                            if (r13d >= 0 /* signed */) {
                                loc_ADDR:
                                __asm("rdtsc");
                                rdx <<= 0xADDR;
                                rax |= rdx;
                                var_b8 = rax;
                                ecx = var_a8;
                                eax = (r10 + 4);
                                ecx += 0xADDR;
                                r11d = -1;
                                rsi = r8;
                                r8d = 0xADDR;
                                r9d = 0xADDR;
                                r13d = ((ecx < 8) ? -1 : 0);
                                edx = ecx;
                                r13d &= 0xADDR;
                                edi = r12d;
                                r13d += 0xADDR;
                                var_d8 = r13d;
                                if (eax < 4) {
                                    r13d = r8d;
                                    var_d8 = r8d;
                                } else {
                                    eax += ecx;
                                    ecx = r11d;
                                    ecx = ((eax >= edx) ? eax : ecx);
                                    r13d = ((eax < edx) ? r8d : 0);
                                    r13d += r9d;
                                    var_d8 = r13d;
                                    if (eax < edx) {
                                        if (r13d >= 0 /* signed */) {
                                            r12d = var_164;
                                            loc_ADDR:
                                            r13d = ((r12d != 0) ? r12d : r13d);
                                            loc_ADDR:
                                            var_d8 = r13d;
                                        }
                                    } else {
                                        var_a8 = 0xADDR;
                                        eax = (rcx + 0xADDR);
                                        r11d = ((eax >= ecx) ? eax : r11d);
                                        r13d = ((eax < ecx) ? r8d : 0);
                                        r13d += r9d;
                                        var_d8 = r13d;
                                        if (eax >= ecx) {
                                            edi = 0;
                                            var_1f8 = rdi;
                                            sub_140024e6c(rcx, rdx, &var_1f8);
                                            if (eax >= 0 /* signed */) {
                                                rax = var_1f8;
                                                r8d = 0xADDR;
                                                rax += 1;
                                                var_1f8 = rax;
                                                eax = ((rax * 2) + 4);
                                                if (eax < 4) {
                                                    eax = var_118;
                                                    r13d = r8d;
                                                    var_118 = eax;
                                                    eax = var_a8;
                                                    var_d8 = r8d;
                                                    var_78 = r10d;
                                                } else {
                                                    r13d = edi;
                                                    var_d8 = edi;
                                                }
                                                edi = r12d;
                                                if (r13d >= 0 /* signed */) {
                                                    eax += r11d;
                                                    edx = -1;
                                                    r9d = 0xADDR;
                                                    edx = ((eax >= r11d) ? eax : edx);
                                                    r13d = (((eax < r11d) ? r8d : 0) + 0xADDR);
                                                    var_d8 = r13d;
                                                    if (eax >= r11d) {
                                                        eax = (rdx + 8);
                                                        r11d = -1;
                                                        ecx = r11d;
                                                        ecx = ((eax >= edx) ? eax : ecx);
                                                        r13d = ((eax < edx) ? r8d : 0);
                                                        r13d += r9d;
                                                        var_d8 = r13d;
                                                        if (eax >= edx) {
                                                            eax = (rcx + 8);
                                                            edx = r11d;
                                                            edx = ((eax >= ecx) ? eax : edx);
                                                            r13d = ((eax < ecx) ? r8d : 0);
                                                            r13d += r9d;
                                                            var_d8 = r13d;
                                                            if (eax >= ecx) {
                                                                var_2f4 = edx;
                                                                edi = edx;
                                                                GetProcessHeap();
                                                                HeapAlloc(rax, 8, edi);
                                                                r13d = 0;
                                                                if (rax == 0) {
                                                                    eax = 0xADDR;
                                                                    r13d = eax;
                                                                    var_d8 = eax;
                                                                } else {
                                                                    var_2f8 = rax;
                                                                    r10d = r13d;
                                                                    var_2f0 = r13d;
                                                                    edx = var_78;
                                                                    r9 = (rax + 4);
                                                                    edi = var_118;
                                                                    r8 = rax;
                                                                    r11d = (r13 + 1);
                                                                    if (r9 < rax) {
                                                                        r13d = 0xADDR;
                                                                        loc_ADDR:
                                                                        var_d8 = r13d;
                                                                        goto loc_ADDR;
                                                                    }
                                                                    ecx = var_2f4;
                                                                    rcx += rax;
                                                                    rax += 8;
                                                                    if (rax <= rcx) {
                                                                        *(u32*)(r8) = 4;
                                                                        *(u32*)(r9) = r13d;
                                                                        r10d = var_2f0;
                                                                        ecx = var_2f4;
                                                                        r10d += r11d;
                                                                        var_2f0 = r10d;
                                                                        if (rbx != 0) {
                                                                            if (edi == 0) {
                                                                                loc_ADDR:
                                                                                r13d = 0xADDR;
                                                                                goto loc_ADDR;
                                                                            } else if (var_2f8 != r13) {
                                                                                r8 = var_2f8;
                                                                                r9d = r13d;
                                                                                if (r10d == 0) {
                                                                                    r10 = (r8 + 4);
                                                                                    var_78 = edx;
                                                                                    if (r10 >= r8) {
                                                                                        rcx += var_2f8;
                                                                                        r9d = edi;
                                                                                        rax = (r9 + 4);
                                                                                        rax += r8;
                                                                                        if (rax > rcx) {
                                                                                            r13d = 0xADDR;
                                                                                            goto loc_ADDR;
                                                                                        }
                                                                                        *(u32*)(r8) = edi;
                                                                                        if (rbx != 0) {
                                                                                            r8d = r9d;
                                                                                            rdx = rbx;
                                                                                            rcx = r10;
                                                                                            sub_1400274ac(r10, rbx, r9d);
                                                                                            edx = var_78;
                                                                                            r11d = 1;
                                                                                        }
                                                                                        r10d = var_2f0;
                                                                                        r8d = var_2f4;
                                                                                        loc_ADDR:
                                                                                        r10d += r11d;
                                                                                        var_2f0 = r10d;
                                                                                        if (rsi != 0) {
                                                                                            if (edx != 0) {
                                                                                                if (var_2f8 != r13) {
                                                                                                    rdi = var_2f8;
                                                                                                    ecx = r13d;
                                                                                                    if (r10d == 0) {
                                                                                                        r10 = (rdi + 4);
                                                                                                        if (r10 >= rdi) {
                                                                                                            r9d = edx;
                                                                                                            ecx = r8d;
                                                                                                            rcx += var_2f8;
                                                                                                            rax = (r9 + 4);
                                                                                                            rax += rdi;
                                                                                                            if (rax <= rcx) {
                                                                                                                *(u32*)(rdi) = edx;
                                                                                                                if (rsi != 0) {
                                                                                                                    r8d = r9d;
                                                                                                                    rdx = rsi;
                                                                                                                    rcx = r10;
                                                                                                                    sub_1400274ac(r10, rsi, r9d);
                                                                                                                    r11d = 1;
                                                                                                                }
                                                                                                                r10d = var_2f0;
                                                                                                                ecx = var_2f4;
                                                                                                                loc_ADDR:
                                                                                                                r10d += r11d;
                                                                                                                var_2f0 = r10d;
                                                                                                                if (var_2f8 != r13) {
                                                                                                                    rdx = var_2f8;
                                                                                                                    edi = r13d;
                                                                                                                    if (r10d == 0) {
                                                                                                                        rdi = (rdx + 4);
                                                                                                                        if (rdi >= rdx) {
                                                                                                                            rcx += var_2f8;
                                                                                                                            rax = (rdx + 0xADDR);
                                                                                                                            if (rax <= rcx) {
                                                                                                                                rax = var_b8;
                                                                                                                                *(u32*)(rdx) = 8;
                                                                                                                                *(u64*)(rdi) = rax;
                                                                                                                                r10d = var_2f0;
                                                                                                                                r10d += r11d;
                                                                                                                                r11d = var_2f4;
                                                                                                                                loc_ADDR:
                                                                                                                                var_2f0 = r10d;
                                                                                                                                var_200 = r13;
                                                                                                                                sub_140024e6c(rcx, rdx, &var_200);
                                                                                                                                if (eax < 0 /* signed */) {
                                                                                                                                    r13d = 0xADDR;
                                                                                                                                    goto loc_ADDR;
                                                                                                                                }
                                                                                                                                rax = var_200;
                                                                                                                                rdi = (rax + 1);
                                                                                                                                if (rdi >= rax) {
                                                                                                                                    var_200 = rdi;
                                                                                                                                    edi += edi;
                                                                                                                                    if (edi == 0) {
                                                                                                                                        r13d = 0xADDR;
                                                                                                                                        loc_ADDR:
                                                                                                                                        r9d = 1;
                                                                                                                                        loc_ADDR:
                                                                                                                                        var_d8 = r13d;
                                                                                                                                    } else if (var_2f8 != r13) {
                                                                                                                                        rdx = var_2f8;
                                                                                                                                        r9d = r13d;
                                                                                                                                        if (r10d == 0) {
                                                                                                                                            r9 = (rdx + 4);
                                                                                                                                            if (r9 < rdx) {
                                                                                                                                                loc_ADDR:
                                                                                                                                                r13d = 0xADDR;
                                                                                                                                                goto loc_ADDR;
                                                                                                                                            }
                                                                                                                                            ecx = r11d;
                                                                                                                                            rax = (rdi + 4);
                                                                                                                                            rcx += var_2f8;
                                                                                                                                            rax += rdx;
                                                                                                                                            var_d8 = r13d;
                                                                                                                                            r8d = edi;
                                                                                                                                            if (rax <= rcx) {
                                                                                                                                                *(u32*)(rdx) = edi;
                                                                                                                                                sub_1400274ac(r9, "Security-SPP-GenuineLocalStatus");
                                                                                                                                                r10d = var_2f0;
                                                                                                                                                r11d = var_2f4;
                                                                                                                                                r10d += 1;
                                                                                                                                                var_2f0 = r10d;
                                                                                                                                            } else {
                                                                                                                                                r13d = 0xADDR;
                                                                                                                                                goto loc_ADDR;
                                                                                                                                            }
                                                                                                                                        } else {
                                                                                                                                            rcx = rsi;
                                                                                                                                            r8 = rbx;
                                                                                                                                            while (1) {
                                                                                                                                                eax = *(u32*)(rdx);
                                                                                                                                                rbx = r8;
                                                                                                                                                eax += 4;
                                                                                                                                                rsi = rcx;
                                                                                                                                                if (eax < 4) break;
                                                                                                                                                ecx = eax;
                                                                                                                                                rcx += rdx;
                                                                                                                                                if (rcx < rdx) break;
                                                                                                                                                r9d += 1;
                                                                                                                                                rdx = rcx;
                                                                                                                                                rcx = rsi;
                                                                                                                                                if (r9d < r10d) continue;
                                                                                                                                            }
                                                                                                                                        }
                                                                                                                                    } else {
                                                                                                                                        edi += 4;
                                                                                                                                        if (edi < 4) goto loc_ADDR;
                                                                                                                                    }
                                                                                                                                    r8 = rbx;
                                                                                                                                    rcx = rsi;
                                                                                                                                    edi = r12d;
                                                                                                                                    if (r13d >= 0 /* signed */) {
                                                                                                                                        r13d = 0;
                                                                                                                                        if (var_2f8 != 0) {
                                                                                                                                            rdx = var_2f8;
                                                                                                                                            edi = r13d;
                                                                                                                                            if (r10d == 0) {
                                                                                                                                                rdi = (rdx + 4);
                                                                                                                                                if (rdi >= rdx) {
                                                                                                                                                    ecx = r11d;
                                                                                                                                                    rax = (rdx + 8);
                                                                                                                                                    rcx += var_2f8;
                                                                                                                                                    if (rax <= rcx) {
                                                                                                                                                        *(u32*)(rdx) = 4;
                                                                                                                                                        *(u32*)(rdi) = r12d;
                                                                                                                                                        r10d = var_2f0;
                                                                                                                                                        edi = var_2f4;
                                                                                                                                                        loc_ADDR:
                                                                                                                                                        r10d += r9d;
                                                                                                                                                        var_2f0 = r10d;
                                                                                                                                                        if (var_2f8 != r13) {
                                                                                                                                                            rdx = var_2f8;
                                                                                                                                                            ecx = r13d;
                                                                                                                                                            if (r10d == 0) {
                                                                                                                                                                r8 = (rdx + 4);
                                                                                                                                                                if (r8 >= rdx) {
                                                                                                                                                                    ecx = edi;
                                                                                                                                                                    rax = (rdx + 8);
                                                                                                                                                                    rcx += var_2f8;
                                                                                                                                                                    if (rax <= rcx) {
                                                                                                                                                                        *(u32*)(rdx) = 4;
                                                                                                                                                                        *(u32*)(r8) = 4;
                                                                                                                                                                        var_2f0 += r9d;
                                                                                                                                                                        ecx = var_2f4;
                                                                                                                                                                        loc_ADDR:
                                                                                                                                                                        rdi = r13;
                                                                                                                                                                        var_80 = r13;
                                                                                                                                                                        var_a0 = r13;
                                                                                                                                                                        var_e0 = r13;
                                                                                                                                                                        var_e8 = r13;
                                                                                                                                                                        var_128 = r13;
                                                                                                                                                                        __asm("rdtsc");
                                                                                                                                                                        rdx <<= 0xADDR;
                                                                                                                                                                        rax |= rdx;
                                                                                                                                                                        var_c8 = rax;
                                                                                                                                                                        ecx += 8;
                                                                                                                                                                        if (ecx < 8) {
                                                                                                                                                                            r13d = 0xADDR;
                                                                                                                                                                        } else {
                                                                                                                                                                            rdx = (rcx + 7);
                                                                                                                                                                            var_c0 = rbx;
                                                                                                                                                                            edx &= -0xADDR;
                                                                                                                                                                            var_120 = rsi;
                                                                                                                                                                            if (edx >= ecx) {
                                                                                                                                                                                r13d = edx;
                                                                                                                                                                                GetProcessHeap();
                                                                                                                                                                                HeapAlloc(rax, 8, r13d);
                                                                                                                                                                                var_f8 = rax;
                                                                                                                                                                                r12 = rax;
                                                                                                                                                                                if (rax == 0) {
                                                                                                                                                                                    r13d = (0xADDR | 0xADDR);
                                                                                                                                                                                    var_d8 = r13d;
                                                                                                                                                                                    loc_ADDR:
                                                                                                                                                                                    ecx = var_108;
                                                                                                                                                                                    edi = ecx;
                                                                                                                                                                                    if (r13d >= 0 /* signed */) {
                                                                                                                                                                                        r11d = 1;
                                                                                                                                                                                        if (var_300 >= 1) {
                                                                                                                                                                                            edi = 0;
                                                                                                                                                                                            if (var_308 != 0) {
                                                                                                                                                                                                if (var_300 <= edi) {
                                                                                                                                                                                                    loc_ADDR:
                                                                                                                                                                                                    r13d = 0xADDR;
                                                                                                                                                                                                    loc_ADDR:
                                                                                                                                                                                                    edi = ecx;
                                                                                                                                                                                                    goto loc_ADDR;
                                                                                                                                                                                                }
                                                                                                                                                                                                r12 = var_308;
                                                                                                                                                                                                rax = (r12 + 4);
                                                                                                                                                                                                if (rax < r12) {
                                                                                                                                                                                                    loc_ADDR:
                                                                                                                                                                                                    r13d = 0xADDR;
                                                                                                                                                                                                } else {
                                                                                                                                                                                                    r13 = rdi;
                                                                                                                                                                                                    r8 = rbx;
                                                                                                                                                                                                    rdx = rsi;
                                                                                                                                                                                                    r13 = ((*(u32*)(r12) != edi) ? rax : r13);
                                                                                                                                                                                                    if (*(u32*)(r12) == 4) {
                                                                                                                                                                                                        r13d = *(u32*)(r13);
                                                                                                                                                                                                        var_d8 = r13d;
                                                                                                                                                                                                        if (r13d != 0xADDR) {
                                                                                                                                                                                                            r12d = r13d;
                                                                                                                                                                                                            if (r13d == 0xADDR) {
                                                                                                                                                                                                                loc_ADDR:
                                                                                                                                                                                                                if (var_300 != 6) goto loc_ADDR;
                                                                                                                                                                                                            } else {
                                                                                                                                                                                                                edi = ecx;
                                                                                                                                                                                                            }
                                                                                                                                                                                                        } else {
                                                                                                                                                                                                            r12d = 0xADDR;
                                                                                                                                                                                                            goto loc_ADDR;
                                                                                                                                                                                                        }
                                                                                                                                                                                                    } else {
                                                                                                                                                                                                        loc_ADDR:
                                                                                                                                                                                                        r13d = 0xADDR;
                                                                                                                                                                                                        goto loc_ADDR;
                                                                                                                                                                                                    }
                                                                                                                                                                                                }
                                                                                                                                                                                            }
                                                                                                                                                                                        } else {
                                                                                                                                                                                            r13d = 0xADDR;
                                                                                                                                                                                            goto loc_ADDR;
                                                                                                                                                                                        }
                                                                                                                                                                                    }
                                                                                                                                                                                } else {
                                                                                                                                                                                    eax = var_2f0;
                                                                                                                                                                                    rdx = (r12 + 4);
                                                                                                                                                                                    *(u32*)(r12) = eax;
                                                                                                                                                                                    if (rdx < r12) {
                                                                                                                                                                                        eax = 0;
                                                                                                                                                                                        var_c8 = r12;
                                                                                                                                                                                        r12d = eax;
                                                                                                                                                                                        var_f8 = rax;
                                                                                                                                                                                        var_c0 = rbx;
                                                                                                                                                                                        r13d = 0xADDR;
                                                                                                                                                                                        var_120 = rsi;
                                                                                                                                                                                        GetProcessHeap();
                                                                                                                                                                                        HeapFree(rax, 0, var_c8);
                                                                                                                                                                                        r13d |= 0xADDR;
                                                                                                                                                                                        var_d8 = r13d;
                                                                                                                                                                                        if (r13d < 0 /* signed */) {
                                                                                                                                                                                            loc_ADDR:
                                                                                                                                                                                            if (r12 != 0) {
                                                                                                                                                                                                loc_ADDR:
                                                                                                                                                                                                GetProcessHeap();
                                                                                                                                                                                                HeapFree(rax, 0, r12);
                                                                                                                                                                                            }
                                                                                                                                                                                            if (rdi != 0) {
                                                                                                                                                                                                r12 = *(u64*)((rdi + 8));
                                                                                                                                                                                                if (r12 != 0) {
                                                                                                                                                                                                    GetProcessHeap();
                                                                                                                                                                                                    HeapFree(rax, 0, r12);
                                                                                                                                                                                                    r12d = 0;
                                                                                                                                                                                                    *(u64*)((rdi + 8)) = r12;
                                                                                                                                                                                                }
                                                                                                                                                                                                r12 = *(u64*)((rdi + 0xADDR));
                                                                                                                                                                                                if (r12 != 0) {
                                                                                                                                                                                                    GetProcessHeap();
                                                                                                                                                                                                    HeapFree(rax, 0, r12);
                                                                                                                                                                                                    r12d = 0;
                                                                                                                                                                                                    *(u64*)((rdi + 0xADDR)) = r12;
                                                                                                                                                                                                }
                                                                                                                                                                                                r12 = *(u64*)((rdi + 0xADDR));
                                                                                                                                                                                                if (r12 != 0) {
                                                                                                                                                                                                    GetProcessHeap();
                                                                                                                                                                                                    HeapFree(rax, 0, r12);
                                                                                                                                                                                                    eax = 0;
                                                                                                                                                                                                    *(u64*)((rdi + 0xADDR)) = rax;
                                                                                                                                                                                                }
                                                                                                                                                                                                GetProcessHeap();
                                                                                                                                                                                                HeapFree(rax, 0, rdi);
                                                                                                                                                                                            }
                                                                                                                                                                                            rdi = var_a0;
                                                                                                                                                                                            if (rdi != 0) {
                                                                                                                                                                                                GetProcessHeap();
                                                                                                                                                                                                HeapFree(rax, 0, rdi);
                                                                                                                                                                                            }
                                                                                                                                                                                            rdi = var_e0;
                                                                                                                                                                                            if (rdi != 0) {
                                                                                                                                                                                                GetProcessHeap();
                                                                                                                                                                                                HeapFree(rax, 0, rdi);
                                                                                                                                                                                            }
                                                                                                                                                                                            rdi = var_e8;
                                                                                                                                                                                            if (rdi != 0) {
                                                                                                                                                                                                r12 = *(u64*)((rdi + 8));
                                                                                                                                                                                                if (r12 != 0) {
                                                                                                                                                                                                    GetProcessHeap();
                                                                                                                                                                                                    HeapFree(rax, 0, r12);
                                                                                                                                                                                                    r12d = 0;
                                                                                                                                                                                                    *(u64*)((rdi + 8)) = r12;
                                                                                                                                                                                                }
                                                                                                                                                                                                r12 = *(u64*)((rdi + 0xADDR));
                                                                                                                                                                                                if (r12 != 0) {
                                                                                                                                                                                                    GetProcessHeap();
                                                                                                                                                                                                    HeapFree(rax, 0, r12);
                                                                                                                                                                                                    r12d = 0;
                                                                                                                                                                                                    *(u64*)((rdi + 0xADDR)) = r12;
                                                                                                                                                                                                }
                                                                                                                                                                                                r12 = *(u64*)((rdi + 0xADDR));
                                                                                                                                                                                                if (r12 != 0) {
                                                                                                                                                                                                    GetProcessHeap();
                                                                                                                                                                                                    HeapFree(rax, 0, r12);
                                                                                                                                                                                                    eax = 0;
                                                                                                                                                                                                    *(u64*)((rdi + 0xADDR)) = rax;
                                                                                                                                                                                                }
                                                                                                                                                                                                GetProcessHeap();
                                                                                                                                                                                                HeapFree(rax, 0, rdi);
                                                                                                                                                                                            }
                                                                                                                                                                                            rdi = var_128;
                                                                                                                                                                                            if (rdi == 0) goto loc_ADDR;
                                                                                                                                                                                        } else {
                                                                                                                                                                                            eax = edi;
                                                                                                                                                                                            loc_ADDR:
                                                                                                                                                                                            ecx = 0;
                                                                                                                                                                                            var_c8 = rcx;
                                                                                                                                                                                            if (r12 != 0) {
                                                                                                                                                                                                r13d = eax;
                                                                                                                                                                                                var_f0 = r13;
                                                                                                                                                                                                if (eax == 0) goto loc_ADDR;
                                                                                                                                                                                            } else {
                                                                                                                                                                                                edi = var_108;
                                                                                                                                                                                                r13d = 0xADDR;
                                                                                                                                                                                                goto loc_ADDR;
                                                                                                                                                                                            }
                                                                                                                                                                                        }
                                                                                                                                                                                    } else {
                                                                                                                                                                                        eax = var_2f4;
                                                                                                                                                                                        rcx = (rdx + 4);
                                                                                                                                                                                        *(u32*)(rdx) = eax;
                                                                                                                                                                                        if (rcx >= rdx) {
                                                                                                                                                                                            rdx = var_c8;
                                                                                                                                                                                            *(u64*)(((r13 + r12) - 8)) = rdx;
                                                                                                                                                                                            sub_1400274ac(rcx, var_2f8, var_2f4);
                                                                                                                                                                                            eax = r13d;
                                                                                                                                                                                            goto loc_ADDR;
                                                                                                                                                                                        }
                                                                                                                                                                                    }
                                                                                                                                                                                }
                                                                                                                                                                            }
                                                                                                                                                                        }
                                                                                                                                                                    }
                                                                                                                                                                }
                                                                                                                                                            } else {
                                                                                                                                                                while (1) {
                                                                                                                                                                    eax = (*(u32*)(rdx) + 4);
                                                                                                                                                                    if (eax < 4) break;
                                                                                                                                                                    r8d = eax;
                                                                                                                                                                    r8 += rdx;
                                                                                                                                                                    if (r8 < rdx) break;
                                                                                                                                                                    r9d = 1;
                                                                                                                                                                    rdx = r8;
                                                                                                                                                                    ecx += 1;
                                                                                                                                                                    if (ecx < r10d) continue;
                                                                                                                                                                }
                                                                                                                                                            }
                                                                                                                                                        } else {
                                                                                                                                                            ecx = (rdi + 8);
                                                                                                                                                            if (ecx >= edi) {
                                                                                                                                                                r10d += r9d;
                                                                                                                                                                var_2f4 = ecx;
                                                                                                                                                                var_2f0 = r10d;
                                                                                                                                                                goto loc_ADDR;
                                                                                                                                                            }
                                                                                                                                                        }
                                                                                                                                                    }
                                                                                                                                                }
                                                                                                                                            } else {
                                                                                                                                                while (1) {
                                                                                                                                                    eax = *(u32*)(rdx);
                                                                                                                                                    rbx = r8;
                                                                                                                                                    eax += 4;
                                                                                                                                                    rsi = rcx;
                                                                                                                                                    if (eax < 4) break;
                                                                                                                                                    ecx = eax;
                                                                                                                                                    rcx += rdx;
                                                                                                                                                    if (rcx < rdx) break;
                                                                                                                                                    edi += r9d;
                                                                                                                                                    rdx = rcx;
                                                                                                                                                    rcx = rsi;
                                                                                                                                                    if (edi < r10d) continue;
                                                                                                                                                }
                                                                                                                                            }
                                                                                                                                        } else {
                                                                                                                                            edi = (r11 + 8);
                                                                                                                                            if (edi >= r11d) {
                                                                                                                                                var_2f4 = edi;
                                                                                                                                                goto loc_ADDR;
                                                                                                                                            }
                                                                                                                                        }
                                                                                                                                    }
                                                                                                                                }
                                                                                                                            }
                                                                                                                        }
                                                                                                                    } else {
                                                                                                                        while (1) {
                                                                                                                            eax = (*(u32*)(rdx) + 4);
                                                                                                                            if (eax < 4) break;
                                                                                                                            r8d = eax;
                                                                                                                            r8 += rdx;
                                                                                                                            if (r8 < rdx) break;
                                                                                                                            edi += r11d;
                                                                                                                            rdx = r8;
                                                                                                                            if (edi < r10d) continue;
                                                                                                                        }
                                                                                                                    }
                                                                                                                } else {
                                                                                                                    r11d = (rcx + 0xADDR);
                                                                                                                    if (r11d >= ecx) {
                                                                                                                        var_2f4 = r11d;
                                                                                                                        r10d += 1;
                                                                                                                        goto loc_ADDR;
                                                                                                                    }
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                    } else {
                                                                                                        while (1) {
                                                                                                            eax = (*(u32*)(rdi) + 4);
                                                                                                            if (eax < 4) break;
                                                                                                            r9d = eax;
                                                                                                            r9 += rdi;
                                                                                                            if (r9 < rdi) break;
                                                                                                            rdi = r9;
                                                                                                            ecx += 1;
                                                                                                            if (ecx < r10d) continue;
                                                                                                        }
                                                                                                    }
                                                                                                } else {
                                                                                                    edx += 4;
                                                                                                    if (edx >= 4) {
                                                                                                        ecx = (r8 + rdx);
                                                                                                        if (ecx >= r8d) {
                                                                                                            var_2f4 = ecx;
                                                                                                            goto loc_ADDR;
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        } else if (edx != 0) {
                                                                                            goto loc_ADDR;
                                                                                        }
                                                                                    }
                                                                                } else {
                                                                                    while (1) {
                                                                                        eax = (*(u32*)(r8) + 4);
                                                                                        if (eax < 4) break;
                                                                                        r11d = eax;
                                                                                        r11 += r8;
                                                                                        if (r11 < r8) break;
                                                                                        r8 = r11;
                                                                                        r9d += 1;
                                                                                        if (r9d < r10d) continue;
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                edi += 4;
                                                                                if (edi >= 4) {
                                                                                    r8d = (rcx + rdi);
                                                                                    if (r8d >= ecx) {
                                                                                        var_2f4 = r8d;
                                                                                        goto loc_ADDR;
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            rcx = qword_ADDR;
                            r10d = (rdi + 8);
                            *(u64*)(rax) = rcx;
                            var_78 = r10d;
                            goto loc_ADDR;
                        }
                    }
                    var_2f0 = 0;
                    r12 = var_2f8;
                    if (r12 != 0) {
                        GetProcessHeap();
                        HeapFree(rax, 0, r12);
                        r12d = 0;
                        var_2f8 = r12;
                    }
                    var_300 = 0;
                    r12 = var_308;
                    if (r12 != 0) {
                        GetProcessHeap();
                        HeapFree(rax, 0, r12);
                        r12d = 0;
                        var_308 = r12;
                    }
                    if (rbx != 0) {
                        GetProcessHeap();
                        HeapFree(rax, 0, rbx);
                    }
                    if (rsi != 0) {
                        GetProcessHeap();
                        HeapFree(rax, 0, rsi);
                    }
                    if (r13d < 0 /* signed */) {
                        esi = var_130;
                    } else {
                        if (edi != 0) {
                            dword_ADDR = edi;
                            edi = 0xADDR;
                            var_220 = r12;
                            GetModuleFileNameW(0xADDR, &var_ea0, edi);
                            if (eax != 0) {
                                if (eax != edi) {
                                    rax = qword_ADDR;
                                    edx = 0;
                                    ecx = 0;
                                    sub_140028010(0, 0, 0xADDR);
                                    var_d0 = rax;
                                    if (rax != 0) {
                                        rax = qword_ADDR;
                                        sub_140028010();
                                        esi = dword_ADDR;
                                        edx = 0;
                                        qword_ADDR = rax;
                                        var_f0 = esi;
                                        sub_1400029dc(&var_810, 0, (rdx + 0xADDR));
                                        xmm0 = 0;
                                        var_110 = r12;
                                        eax = 0;
                                        var_100 = r12;
                                        var_310 = xmm0;
                                        var_a30 = r12d;
                                        var_a34 = xmm0;
                                        edx = (rax + 1);
                                        var_a54 = rax;
                                        var_a44 = xmm0;
                                        var_98 = r12;
                                        do {
                                            eax = 0;
                                            __asm("lock cmpxchg [0xADDR], edx");
                                        } while (dword_ADDR != eax);
                                        eax = dword_ADDR;
                                        if (eax != 0) {
                                            eax += edx;
                                            dword_ADDR = eax;
                                        } else {
                                            edi = 0;
                                            var_78 = r12d;
                                            var_a8 = edi;
                                            var_108 = r12d;
                                            sub_140024bf8(0xADDR);
                                            var_68 = rax;
                                            if (rax == 0) {
                                                ebx = 4;
                                                rdi = off_1400360c0;
                                                r12d = 0;
                                                eax = (rbx - 3);
                                                do {
                                                    rcx = *(u64*)(rdi);
                                                    if (rcx != 0) {
                                                        FreeLibrary(*(u64*)(rdi));
                                                        eax = 1;
                                                    }
                                                    rdi += 0xADDR;
                                                    rbx -= rax;
                                                } while (rbx != 0);
                                                edx = 0;
                                                sub_1400029dc(off_1400360c0, 0, (rbx + 0xADDR));
                                                edx = (rbx + 2);
                                                rax = off_140034000;
                                                rcx = off_140029260;
                                                edi = (rdx + 0xADDR);
                                                do {
                                                    xmm0 = *(__m128*)(rcx);
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)(rax) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    rcx += rdi;
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    rax += rdi;
                                                    *(__m128*)((rax - 0xADDR)) = xmm1;
                                                    rdx -= 1;
                                                } while (rdx != 0);
                                                xmm0 = *(__m128*)(rcx);
                                                edx = 1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)(rax) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                            } else {
                                                r12d = edi;
                                                r14 = (rax + 7);
                                                ecx = -1;
                                                r13 = off_14002da82;
                                                r15d = edi;
                                                esi = (rdi + 0xADDR);
                                                r11d = edi;
                                                ebx = (rdi + 1);
                                                do {
                                                    eax = (uint32)(*(u8*)((r13 - 1)));
                                                    r10d = (uint32)(*(u8*)((r13 - 2)));
                                                    r9d = (uint32)(*(u8*)((r13 + 2)));
                                                    r10d <<= 8;
                                                    r10d |= eax;
                                                    r9d <<= 8;
                                                    eax = (uint32)(*(u8*)(r13));
                                                    r13 += 8;
                                                    r10d <<= 8;
                                                    r10d |= eax;
                                                    eax = (uint32)(*(u8*)((r13 - 7)));
                                                    r10d <<= 8;
                                                    r10d |= eax;
                                                    eax = (uint32)(*(u8*)((r13 - 5)));
                                                    r9d |= eax;
                                                    r11d ^= r10d;
                                                    eax = (uint32)(*(u8*)((r13 - 4)));
                                                    r9d <<= 8;
                                                    r9d |= eax;
                                                    eax = (uint32)(*(u8*)((r13 - 3)));
                                                    r9d <<= 8;
                                                    r9d |= eax;
                                                    r8d = r9d;
                                                    r8d ^= r15d;
                                                    r8d ^= r11d;
                                                    r8d ^= 0xADDR;
                                                    eax = (r8 + 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r8d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r11d ^= edx;
                                                    eax = (r11 + 0xADDR);
                                                    eax = ror(eax, 9);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    r8d ^= edx;
                                                    edx = (r8d * 0xADDR);
                                                    eax = (r8d >> 0xADDR);
                                                    edx -= eax;
                                                    edx -= 0xADDR;
                                                    r11d ^= edx;
                                                    eax = (r11d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    eax = 0xADDR;
                                                    r8d ^= edx;
                                                    edx = (r8d ^ 0xADDR);
                                                    eax -= edx;
                                                    r11d ^= eax;
                                                    eax = r11d;
                                                    edi = r11d;
                                                    eax ^= 0xADDR;
                                                    edi = ror(edi, 6);
                                                    edx = (eax * 0xADDR);
                                                    edi ^= edx;
                                                    r8d ^= edi;
                                                    eax = (r8 + 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r8d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r11d ^= edx;
                                                    eax = (r11 + 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    r8d ^= edx;
                                                    eax = r8d;
                                                    edi = 0xADDR;
                                                    eax ^= 0xADDR;
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r8d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx ^= eax;
                                                    r11d ^= edx;
                                                    eax = (r11d ^ 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = (r11d >> 0xADDR);
                                                    edx ^= eax;
                                                    edx ^= r8d;
                                                    r8d = 0xADDR;
                                                    eax = edx;
                                                    eax = ~(eax);
                                                    eax = ror(eax, 5);
                                                    eax += 0xADDR;
                                                    eax *= 0xADDR;
                                                    r11d ^= eax;
                                                    r15d = (r11 - 0xADDR);
                                                    r15d ^= edx;
                                                    r15d ^= edi;
                                                    eax = (r15d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = (r15d >> 2);
                                                    edx += eax;
                                                    r11d ^= edx;
                                                    eax = (r11 - 0xADDR);
                                                    eax = ror(eax, 6);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r15d ^= edx;
                                                    eax = r15d;
                                                    edi = r15d;
                                                    eax ^= 0xADDR;
                                                    edi = ror(edi, 9);
                                                    edx = (eax * 0xADDR);
                                                    edi += edx;
                                                    r11d ^= edi;
                                                    eax = (r11d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r15d ^= edx;
                                                    r11d ^= r15d;
                                                    r11d ^= 0xADDR;
                                                    eax = r11d;
                                                    edi = r11d;
                                                    eax = ror(eax, 3);
                                                    edx = ((eax * 0xADDR) - 0xADDR);
                                                    r15d ^= edx;
                                                    eax = (r15 - 0xADDR);
                                                    eax = ror(eax, 1);
                                                    edx = (eax * 0xADDR);
                                                    eax = r15d;
                                                    eax = ror(eax, 6);
                                                    edx -= eax;
                                                    edi ^= edx;
                                                    eax = (rdi - 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = edi;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r15d ^= edx;
                                                    eax = (r15 - 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r15d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    edi ^= edx;
                                                    eax = edi;
                                                    eax ^= r8d;
                                                    edx = (eax * 0xADDR);
                                                    eax = (edi >> 3);
                                                    edx ^= eax;
                                                    r15d ^= edx;
                                                    eax = (r15d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    r11d = (eax * 0xADDR);
                                                    eax = r15d;
                                                    eax = ror(eax, 0xADDR);
                                                    r15d ^= ecx;
                                                    ecx = r9d;
                                                    r11d ^= eax;
                                                    r11d ^= edi;
                                                    edi = r15d;
                                                    r11d ^= r12d;
                                                    edi <<= 0xADDR;
                                                    *(u8*)((r14 - 4)) = r11b;
                                                    edx = r11d;
                                                    *(u8*)(r14) = r15b;
                                                    eax = (r11d >> 8);
                                                    r14 += 8;
                                                    edx <<= 0xADDR;
                                                    r12d = r10d;
                                                    edx |= eax;
                                                    eax = r15d;
                                                    *(u8*)((r14 - 0xADDR)) = dl;
                                                    r8d = edx;
                                                    eax >>= 8;
                                                    edi |= eax;
                                                    r8d <<= 0xADDR;
                                                    *(u8*)((r14 - 9)) = dil;
                                                    eax = (edi << 0xADDR);
                                                    edx >>= 8;
                                                    r8d |= edx;
                                                    edi >>= 8;
                                                    *(u8*)((r14 - 0xADDR)) = r8b;
                                                    eax |= edi;
                                                    *(u8*)((r14 - 0xADDR)) = al;
                                                    r8d = ror(r8d, 8);
                                                    eax = ror(eax, 8);
                                                    *(u8*)((r14 - 0xADDR)) = r8b;
                                                    *(u8*)((r14 - 0xADDR)) = al;
                                                    rsi -= rbx;
                                                } while (rsi != 0);
                                                esi = var_f0;
                                                edx = 0;
                                                r13d = var_d8;
                                                eax = edx;
                                                rbx = var_68;
                                                cl = dl;
                                                r12d = var_108;
                                                r14 = 0xADDR;
                                                edi = (rdx + 1);
                                                r8d = 0xADDR;
                                                do {
                                                    cl ^= *(u8*)((rax + rbx));
                                                    rax += rdi;
                                                } while (rax < r8);
                                                eax = (uint32)(cl);
                                                if (rax == *(u64*)("@")) {
                                                    *(u8*)(((r8 + rbx) - 1)) = dl;
                                                    rdi = off_1400360c0;
                                                    sub_1400029dc(rdi, rdx, 0xADDR);
                                                    if (*(u8*)(rbx) == 0xADDR) {
                                                        rdi = rbx;
                                                        loc_ADDR:
                                                        GetProcessHeap();
                                                        HeapFree(rax, 0, rdi);
                                                    } else {
                                                        var_230 = rbx;
                                                        rdx = rbx;
                                                        rax = -1;
                                                    }
                                                    loc_ADDR:
                                                    if (r12d >= 0 /* signed */) {
                                                        eax = dword_ADDR;
                                                        r12d = 0;
                                                        edx = (r12 + 1);
                                                    }
                                                } else {
                                                    sub_140024e2c(rbx);
                                                }
                                            }
                                        }
                                        ecx = 0;
                                        var_78 = r12d;
                                        eax = r12d;
                                        var_108 = r12d;
                                        __asm("xchg [0xADDR], eax");
                                        var_a8 = r12d;
                                        xmm0 = 0;
                                        r12d = 0;
                                        var_da0 = ecx;
                                        var_118 = r12d;
                                        var_130 = r12d;
                                        var_238 = r12;
                                        var_68 = r12;
                                        var_90 = r12;
                                        var_88 = r12;
                                        var_da4 = cx;
                                        var_320 = xmm0;
                                        do {
                                            eax = 0;
                                            __asm("lock cmpxchg [0xADDR], edx");
                                        } while (dword_ADDR != eax);
                                        eax = dword_ADDR;
                                        if (eax != 0) {
                                            eax += edx;
                                            dword_ADDR = eax;
                                        } else {
                                            var_f8 = r12d;
                                            var_148 = r12d;
                                            sub_140024bf8(0xADDR);
                                            var_80 = rax;
                                            rbx = rax;
                                            if (rax == 0) {
                                                ebx = 4;
                                                rdi = off_1400360c0;
                                                eax = (rbx - 3);
                                                do {
                                                    rcx = *(u64*)(rdi);
                                                    if (rcx != 0) {
                                                        FreeLibrary(*(u64*)(rdi));
                                                        eax = 1;
                                                    }
                                                    rdi += 0xADDR;
                                                    rbx -= rax;
                                                } while (rbx != 0);
                                                edx = 0;
                                                sub_1400029dc(off_1400360c0, 0, (rbx + 0xADDR));
                                                edx = (rbx + 2);
                                                rax = off_140034000;
                                                rcx = off_140029260;
                                                edi = (rdx + 0xADDR);
                                                do {
                                                    xmm0 = *(__m128*)(rcx);
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)(rax) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    rcx += rdi;
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    rax += rdi;
                                                    *(__m128*)((rax - 0xADDR)) = xmm1;
                                                    rdx -= 1;
                                                } while (rdx != 0);
                                                xmm0 = *(__m128*)(rcx);
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)(rax) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                            } else {
                                                ecx = -1;
                                                r14 = (rbx + 7);
                                                eax = 0;
                                                r15 = off_14002da82;
                                                r13d = r12d;
                                                r11d = eax;
                                                esi = (rax + 0xADDR);
                                                ebx = (rax + 1);
                                                do {
                                                    eax = (uint32)(*(u8*)((r15 - 1)));
                                                    r10d = (uint32)(*(u8*)((r15 - 2)));
                                                    r9d = (uint32)(*(u8*)((r15 + 2)));
                                                    r10d <<= 8;
                                                    r10d |= eax;
                                                    r9d <<= 8;
                                                    eax = (uint32)(*(u8*)(r15));
                                                    r15 += 8;
                                                    r10d <<= 8;
                                                    r10d |= eax;
                                                    eax = (uint32)(*(u8*)((r15 - 7)));
                                                    r10d <<= 8;
                                                    r10d |= eax;
                                                    eax = (uint32)(*(u8*)((r15 - 5)));
                                                    r9d |= eax;
                                                    r11d ^= r10d;
                                                    eax = (uint32)(*(u8*)((r15 - 4)));
                                                    r9d <<= 8;
                                                    r9d |= eax;
                                                    eax = (uint32)(*(u8*)((r15 - 3)));
                                                    r9d <<= 8;
                                                    r9d |= eax;
                                                    r12d ^= r9d;
                                                    r12d ^= r11d;
                                                    r12d ^= 0xADDR;
                                                    r8d = r12d;
                                                    eax = (r12 + 0xADDR);
                                                    r12d = ror(r12d, 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    edx += r12d;
                                                    r11d ^= edx;
                                                    eax = (r11 + 0xADDR);
                                                    eax = ror(eax, 9);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    r8d ^= edx;
                                                    edx = (r8d * 0xADDR);
                                                    eax = (r8d >> 0xADDR);
                                                    edx -= eax;
                                                    edx -= 0xADDR;
                                                    r11d ^= edx;
                                                    eax = (r11d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    eax = 0xADDR;
                                                    r8d ^= edx;
                                                    edx = (r8d ^ 0xADDR);
                                                    eax -= edx;
                                                    r11d ^= eax;
                                                    eax = r11d;
                                                    edi = r11d;
                                                    eax ^= 0xADDR;
                                                    edi = ror(edi, 6);
                                                    edx = (eax * 0xADDR);
                                                    edi ^= edx;
                                                    r8d ^= edi;
                                                    edi = 0xADDR;
                                                    eax = (r8 + 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r8d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r11d ^= edx;
                                                    eax = (r11 + 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    r8d ^= edx;
                                                    eax = r8d;
                                                    eax ^= edi;
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r8d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx ^= eax;
                                                    r11d ^= edx;
                                                    eax = (r11d ^ 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = (r11d >> 0xADDR);
                                                    edx ^= eax;
                                                    r8d ^= edx;
                                                    eax = r8d;
                                                    eax = ~(eax);
                                                    eax = ror(eax, 5);
                                                    eax += 0xADDR;
                                                    eax *= 0xADDR;
                                                    r11d ^= eax;
                                                    r12d = (r11 - 0xADDR);
                                                    r12d ^= r8d;
                                                    r12d ^= edi;
                                                    eax = (r12d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = (r12d >> 2);
                                                    edx += eax;
                                                    r11d ^= edx;
                                                    eax = (r11 - 0xADDR);
                                                    eax = ror(eax, 6);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r12d ^= edx;
                                                    eax = r12d;
                                                    edi = r12d;
                                                    eax ^= 0xADDR;
                                                    edi = ror(edi, 9);
                                                    edx = (eax * 0xADDR);
                                                    edi += edx;
                                                    r11d ^= edi;
                                                    eax = (r11d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r12d ^= edx;
                                                    edi = r12d;
                                                    edi ^= r11d;
                                                    edi ^= 0xADDR;
                                                    eax = edi;
                                                    eax = ror(eax, 3);
                                                    edx = ((eax * 0xADDR) - 0xADDR);
                                                    r12d ^= edx;
                                                    eax = (r12 - 0xADDR);
                                                    eax = ror(eax, 1);
                                                    edx = (eax * 0xADDR);
                                                    eax = r12d;
                                                    eax = ror(eax, 6);
                                                    edx -= eax;
                                                    edi ^= edx;
                                                    eax = (rdi - 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = edi;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r12d ^= edx;
                                                    eax = (r12 - 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r12d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    edi ^= edx;
                                                    eax = (edi ^ 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = (edi >> 3);
                                                    edx ^= eax;
                                                    r12d ^= edx;
                                                    eax = (r12d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    r11d = (eax * 0xADDR);
                                                    eax = r12d;
                                                    eax = ror(eax, 0xADDR);
                                                    r12d ^= ecx;
                                                    ecx = r9d;
                                                    r11d ^= eax;
                                                    r11d ^= edi;
                                                    edi = r12d;
                                                    r11d ^= r13d;
                                                    edi <<= 0xADDR;
                                                    *(u8*)((r14 - 4)) = r11b;
                                                    edx = r11d;
                                                    *(u8*)(r14) = r12b;
                                                    eax = (r11d >> 8);
                                                    r14 += 8;
                                                    edx <<= 0xADDR;
                                                    r13d = r10d;
                                                    edx |= eax;
                                                    eax = r12d;
                                                    *(u8*)((r14 - 0xADDR)) = dl;
                                                    r8d = edx;
                                                    eax >>= 8;
                                                    edi |= eax;
                                                    r8d <<= 0xADDR;
                                                    *(u8*)((r14 - 9)) = dil;
                                                    eax = (edi << 0xADDR);
                                                    edx >>= 8;
                                                    r8d |= edx;
                                                    edi >>= 8;
                                                    *(u8*)((r14 - 0xADDR)) = r8b;
                                                    eax |= edi;
                                                    *(u8*)((r14 - 0xADDR)) = al;
                                                    r8d = ror(r8d, 8);
                                                    eax = ror(eax, 8);
                                                    *(u8*)((r14 - 0xADDR)) = r8b;
                                                    *(u8*)((r14 - 0xADDR)) = al;
                                                    rsi -= rbx;
                                                } while (rsi != 0);
                                                esi = var_f0;
                                                r12d = 0;
                                                r13d = var_d8;
                                                eax = r12d;
                                                rbx = var_80;
                                                cl = r12b;
                                                r14 = 0xADDR;
                                                edi = 0xADDR;
                                                edx = (r12 + 1);
                                                do {
                                                    cl ^= *(u8*)((rax + rbx));
                                                    rax += rdx;
                                                } while (rax < rdi);
                                                eax = (uint32)(cl);
                                                if (rax == *(u64*)("@")) {
                                                    edx = 0;
                                                    *(u8*)(((rbx + rdi) - 1)) = r12b;
                                                    rdi = off_1400360c0;
                                                    sub_1400029dc(rdi, 0, (rdx + 0xADDR));
                                                    if (*(u8*)(rbx) == r12b) {
                                                        edi = r12d;
                                                        loc_ADDR:
                                                        GetProcessHeap();
                                                        HeapFree(rax, 0, rbx);
                                                    } else {
                                                        rdx = rbx;
                                                        rax = -1;
                                                    }
                                                    loc_ADDR:
                                                    if (edi >= 0 /* signed */) {
                                                        eax = dword_ADDR;
                                                        edx = 1;
                                                    }
                                                } else {
                                                    sub_140024e2c(rbx);
                                                }
                                            }
                                        }
                                        edx = 0;
                                        rcx = &var_810;
                                        eax = r12d;
                                        __asm("xchg [0xADDR], eax");
                                        sub_1400029dc(&var_810, rdx, (rdx + 0xADDR));
                                        ecx = 0xADDR;
                                        eax = (esi & 0xADDR);
                                        xmm0 = 0;
                                        var_148 = eax;
                                        edx = 0;
                                        eax = (esi >> 4);
                                        ax &= 0xADDR;
                                        var_da0 = ax;
                                        eax = (esi >> 8);
                                        ax &= 0xADDR;
                                        var_da2 = ax;
                                        eax = (esi >> 0xADDR);
                                        ax &= cx;
                                        ecx = 0;
                                        var_da4 = ax;
                                        rax = qword_ADDR;
                                        var_310 = xmm0;
                                        sub_140028010(0, 0, 0xADDR);
                                        var_80 = rax;
                                        var_120 = 0xADDR;
                                        if (rax != 0) {
                                            eax = 0;
                                            var_f8 = r12d;
                                            xmm0 = 0;
                                            var_158 = eax;
                                            rbx = r12;
                                            var_258 = r12;
                                            var_250 = rbx;
                                            ecx = (rax + 8);
                                            var_260 = r12;
                                            rax = qword_ADDR;
                                            r8d = 0;
                                            var_268 = r12;
                                            edi = r12d;
                                            var_270 = r12;
                                            var_c90 = xmm0;
                                            var_160 = r12d;
                                            var_ca0 = xmm0;
                                            var_180 = r12;
                                            var_cb0 = xmm0;
                                            sub_140028010((rax + 8), &var_160, 0, &var_158);
                                            if (eax != 0) {
                                                ecx = var_158;
                                                rcx += rcx;
                                                sub_140024bf8(rcx);
                                                sub_140024d58(&var_270, rax);
                                                eax = 0;
                                                if (var_270 != 0) {
                                                    ecx = (var_160 + 1);
                                                    rcx <<= 2;
                                                    sub_140024bf8(rcx);
                                                    sub_140024d58(&var_180, rax);
                                                    rdi = var_180;
                                                    if (rdi != 0) {
                                                        rax = qword_ADDR;
                                                        sub_140028010(8, &var_160, var_270, &var_158);
                                                        ecx = 0;
                                                        if (eax != 0) {
                                                            rax = rdi;
                                                            if (var_160 > ecx) {
                                                                rbx = var_270;
                                                                r14 = rdi;
                                                                r15d = ecx;
                                                                esi = 1;
                                                                do {
                                                                    rax = qword_ADDR;
                                                                    edx = 0;
                                                                    rcx = rbx;
                                                                    sub_140028010(rbx, 0);
                                                                    *(u32*)(r14) = eax;
                                                                    rax = -1;
                                                                    ecx = 0;
                                                                    do {
                                                                        rax += 1;
                                                                    } while (*(u16*)((rbx + (rax * 2))) != cx);
                                                                    rbx += (rax * 2);
                                                                    r14 += 4;
                                                                    rbx += 2;
                                                                    r15d += esi;
                                                                } while (r15d < var_160);
                                                                esi = var_f0;
                                                                var_180 = r14;
                                                                r14 = 0xADDR;
                                                                rax = var_180;
                                                            }
                                                            rbx = rdi;
                                                            *(u32*)(rax) = 0xADDR;
                                                            edi = (var_160 + 1);
                                                            var_180 = rcx;
                                                            var_f8 = edi;
                                                            var_250 = rbx;
                                                        } else {
                                                            GetLastError();
                                                            r12d = eax;
                                                            if (eax > 0 /* signed */) {
                                                                r12d = (uint32)(r12w);
                                                                r12d |= 0xADDR;
                                                            }
                                                            __asm("if (js 0xADDR /* unmodelled flags */) goto 0xADDR");
                                                            r12d = 0xADDR;
                                                            edi = ebx;
                                                        }
                                                    } else {
                                                        r12d = 0xADDR;
                                                    }
                                                } else {
                                                    r12d = 0xADDR;
                                                }
                                            } else {
                                                GetLastError();
                                                r12d = eax;
                                                if (eax > 0 /* signed */) {
                                                    r12d = (uint32)(r12w);
                                                    r12d |= 0xADDR;
                                                }
                                                __asm("if (js short 0xADDR /* unmodelled flags */) goto 0xADDR");
                                                r12d = 0xADDR;
                                            }
                                            sub_140024c8c(&var_180);
                                            sub_140024c8c(&var_270);
                                            r10d = 0;
                                            if (r12d < 0 /* signed */) {
                                                rbx = r10;
                                                r8 = r10;
                                            } else {
                                                var_c0 = r10d;
                                                r12d = r10d;
                                                r11 = off_14002d950;
                                                r8d = r10d;
                                                if (edi != 0) {
                                                    r9d = var_f8;
                                                    edi = r10d;
                                                    do {
                                                        ecx = r10d;
                                                        rdx = r11;
                                                        while (1) {
                                                            eax = *(u32*)(rdx);
                                                            if (*(u32*)((rbx + rdi)) == eax) break;
                                                            eax = 1;
                                                            rdx += 4;
                                                            ecx += 1;
                                                            if (ecx < 0xADDR) continue;
                                                        }
                                                        r8d += eax;
                                                        rdi += 4;
                                                    } while (r8d < r9d);
                                                }
                                                xmm0 = 0;
                                                eax = r12d;
                                                r9d = 0;
                                                var_f8 = r10d;
                                                var_900 = xmm0;
                                                ecx = *(u32*)((r11 + (rax * 4)));
                                                rax = qword_ADDR;
                                                var_910 = xmm0;
                                                sub_140028010(*(u32*)((r11 + (rax * 4))), &var_df0, (r9 + 0xADDR));
                                                ebx = 0;
                                                if (eax != 0) {
                                                    rax = qword_ADDR;
                                                    sub_140028010(&var_df0, (rbx + 0xADDR), &var_900, (rbx + 0xADDR));
                                                    if (eax > 0 /* signed */) {
                                                        ebx = ((var_90c >> 0xADDR) & 1);
                                                        var_f8 = ebx;
                                                        ebx = 0;
                                                    }
                                                }
                                                edx = 0;
                                                sub_1400029dc(&var_d08, 0, 0xADDR);
                                                edi = 0xADDR;
                                                rcx = "Segoe UI Light";
                                                rdx = &var_d00;
                                                r14d = (rdi - 0xADDR);
                                                do {
                                                    *(u64*)(rdx) = rcx;
                                                    rax = -1;
                                                    do {
                                                        rax += 1;
                                                    } while (*(u16*)((rcx + (rax * 2))) != bx);
                                                    rcx += (rax * 2);
                                                    rdx += 8;
                                                    rcx += 2;
                                                    rdi -= r14;
                                                } while (rdi != 0);
                                                rdx = 0xADDR;
                                                rcx = (rax + (rax * 2));
                                                rax = *(u64*)(((rbp + (rax * 8)) + 0xADDR));
                                                var_68 = rax;
                                                var_b8 = rax;
                                                r8 = *(u64*)(((rbp + (rax * 8)) + 0xADDR));
                                                eax = (uint32)(*(u8*)(((rcx + rdx) + 0xADDR)));
                                                var_90 = r8;
                                                var_a0 = r8;
                                                rbx = *(u64*)(((rbp + (rax * 8)) + 0xADDR));
                                                var_88 = rbx;
                                                var_e0 = rbx;
                                                sub_140024bf8(0xADDR);
                                                r10d = 0;
                                                var_e8 = rax;
                                                r11 = rax;
                                                r14 = 0xADDR;
                                                if (rax != 0) {
                                                    r14 = (r11 + 7);
                                                    r13d = r10d;
                                                    r11d = 1;
                                                    r15 = 0xADDR;
                                                    eax = -1;
                                                    r12d = r10d;
                                                    ebx = r10d;
                                                    esi = 0xADDR;
                                                    do {
                                                        ecx = (uint32)(*(u8*)((r15 - 1)));
                                                        r10d = (uint32)(*(u8*)((r15 - 2)));
                                                        r9d = (uint32)(*(u8*)((r15 + 2)));
                                                        r10d <<= 8;
                                                        r10d |= ecx;
                                                        r9d <<= 8;
                                                        ecx = (uint32)(*(u8*)(r15));
                                                        r15 += 8;
                                                        r10d <<= 8;
                                                        r10d |= ecx;
                                                        ecx = (uint32)(*(u8*)((r15 - 7)));
                                                        r10d <<= 8;
                                                        r10d |= ecx;
                                                        ecx = (uint32)(*(u8*)((r15 - 5)));
                                                        r9d |= ecx;
                                                        ebx ^= r10d;
                                                        ecx = (uint32)(*(u8*)((r15 - 4)));
                                                        r9d <<= 8;
                                                        r9d |= ecx;
                                                        ecx = (uint32)(*(u8*)((r15 - 3)));
                                                        r9d <<= 8;
                                                        r9d |= ecx;
                                                        r12d ^= r9d;
                                                        r12d ^= ebx;
                                                        r12d ^= 0xADDR;
                                                        r8d = r12d;
                                                        ecx = (r12 + 0xADDR);
                                                        r12d = ror(r12d, 0xADDR);
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        edx += r12d;
                                                        ebx ^= edx;
                                                        ecx = (rbx + 0xADDR);
                                                        ecx = ror(ecx, 9);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = ebx;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx -= ecx;
                                                        r8d ^= edx;
                                                        edx = (r8d * 0xADDR);
                                                        ecx = (r8d >> 0xADDR);
                                                        edx -= ecx;
                                                        edx -= 0xADDR;
                                                        ebx ^= edx;
                                                        ecx = (ebx ^ 0xADDR);
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = ebx;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx -= ecx;
                                                        ecx = 0xADDR;
                                                        r8d ^= edx;
                                                        edx = (r8d ^ 0xADDR);
                                                        ecx -= edx;
                                                        ebx ^= ecx;
                                                        ecx = ebx;
                                                        edi = ebx;
                                                        ecx ^= 0xADDR;
                                                        edi = ror(edi, 6);
                                                        edx = (ecx * 0xADDR);
                                                        edi ^= edx;
                                                        r8d ^= edi;
                                                        edi = 0xADDR;
                                                        ecx = (r8 + 0xADDR);
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = r8d;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx += ecx;
                                                        ebx ^= edx;
                                                        ecx = (rbx + 0xADDR);
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = ebx;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx -= ecx;
                                                        r8d ^= edx;
                                                        ecx = r8d;
                                                        ecx ^= edi;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = r8d;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx ^= ecx;
                                                        ebx ^= edx;
                                                        ecx = (ebx ^ 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = (ebx >> 0xADDR);
                                                        edx ^= ecx;
                                                        r8d ^= edx;
                                                        ecx = r8d;
                                                        ecx = ~(ecx);
                                                        ecx = ror(ecx, 5);
                                                        ecx += 0xADDR;
                                                        ecx *= 0xADDR;
                                                        ebx ^= ecx;
                                                        r12d = (rbx - 0xADDR);
                                                        r12d ^= r8d;
                                                        r12d ^= edi;
                                                        ecx = (r12d ^ 0xADDR);
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = (r12d >> 2);
                                                        edx += ecx;
                                                        ebx ^= edx;
                                                        ecx = (rbx - 0xADDR);
                                                        ecx = ror(ecx, 6);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = ebx;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx += ecx;
                                                        r12d ^= edx;
                                                        ecx = r12d;
                                                        edi = r12d;
                                                        ecx ^= 0xADDR;
                                                        edi = ror(edi, 9);
                                                        edx = (ecx * 0xADDR);
                                                        edi += edx;
                                                        ebx ^= edi;
                                                        ecx = (ebx ^ 0xADDR);
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = ebx;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx += ecx;
                                                        r12d ^= edx;
                                                        edi = r12d;
                                                        edi ^= ebx;
                                                        edi ^= 0xADDR;
                                                        ecx = edi;
                                                        ecx = ror(ecx, 3);
                                                        edx = ((ecx * 0xADDR) - 0xADDR);
                                                        r12d ^= edx;
                                                        ecx = (r12 - 0xADDR);
                                                        ecx = ror(ecx, 1);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = r12d;
                                                        ecx = ror(ecx, 6);
                                                        edx -= ecx;
                                                        edi ^= edx;
                                                        ecx = (rdi - 0xADDR);
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = edi;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx += ecx;
                                                        r12d ^= edx;
                                                        ecx = (r12 - 0xADDR);
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = r12d;
                                                        ecx = ror(ecx, 0xADDR);
                                                        edx -= ecx;
                                                        edi ^= edx;
                                                        ecx = (edi ^ 0xADDR);
                                                        edx = (ecx * 0xADDR);
                                                        ecx = (edi >> 3);
                                                        edx ^= ecx;
                                                        r12d ^= edx;
                                                        ecx = (r12d ^ 0xADDR);
                                                        ecx = ror(ecx, 0xADDR);
                                                        ebx = (ecx * 0xADDR);
                                                        ecx = r12d;
                                                        ecx = ror(ecx, 0xADDR);
                                                        r12d ^= eax;
                                                        eax = r9d;
                                                        ebx ^= ecx;
                                                        ebx ^= edi;
                                                        edi = r12d;
                                                        ebx ^= r13d;
                                                        edi <<= 0xADDR;
                                                        *(u8*)((r14 - 4)) = bl;
                                                        edx = ebx;
                                                        *(u8*)(r14) = r12b;
                                                        ecx = (ebx >> 8);
                                                        r14 += 8;
                                                        edx <<= 0xADDR;
                                                        r13d = r10d;
                                                        edx |= ecx;
                                                        ecx = r12d;
                                                        *(u8*)((r14 - 0xADDR)) = dl;
                                                        r8d = edx;
                                                        ecx >>= 8;
                                                        edi |= ecx;
                                                        r8d <<= 0xADDR;
                                                        *(u8*)((r14 - 9)) = dil;
                                                        ecx = (edi << 0xADDR);
                                                        edx >>= 8;
                                                        r8d |= edx;
                                                        edi >>= 8;
                                                        *(u8*)((r14 - 0xADDR)) = r8b;
                                                        ecx |= edi;
                                                        *(u8*)((r14 - 0xADDR)) = cl;
                                                        r8d = ror(r8d, 8);
                                                        ecx = ror(ecx, 8);
                                                        *(u8*)((r14 - 0xADDR)) = r8b;
                                                        *(u8*)((r14 - 0xADDR)) = cl;
                                                        rsi -= r11;
                                                    } while (rsi != 0);
                                                    esi = var_f0;
                                                    r10d = 0;
                                                    r13d = var_d8;
                                                    ecx = r10d;
                                                    r11 = var_e8;
                                                    al = r10b;
                                                    r14 = 0xADDR;
                                                    edi = 0xADDR;
                                                    ebx = (r10 + 1);
                                                    do {
                                                        al ^= *(u8*)((rcx + r11));
                                                        rcx += rbx;
                                                    } while (rcx < rdi);
                                                    ecx = (uint32)(al);
                                                    if (rcx == qword_ADDR) {
                                                        var_258 = r11;
                                                        r12d = r10d;
                                                        var_260 = rdi;
                                                        if (r11 != 0) {
                                                            var_260 = rdi;
                                                        } else {
                                                            r11 = r10;
                                                        }
                                                        r9d = var_c0;
                                                        rdx = &var_c90;
                                                        r8d = 6;
                                                        do {
                                                            ecx = r10d;
                                                            do {
                                                                if (r9d == ecx) {
                                                                    *(u64*)(rdx) = r11;
                                                                }
                                                                rax = -1;
                                                                do {
                                                                    rax += 1;
                                                                } while (*(u8*)((r11 + rax)) != r10b);
                                                                r11 += 1;
                                                                ecx += ebx;
                                                                r11 += rax;
                                                            } while (ecx < 0xADDR);
                                                            rdx += 8;
                                                            r8 -= rbx;
                                                        } while (r8 != 0);
                                                        sub_140024c2c();
                                                        sub_140024d58(&var_268, rax);
                                                        rbx = var_268;
                                                        r10d = 0;
                                                        if (rbx == 0) {
                                                            r12d = 0xADDR;
                                                        } else {
                                                            ecx = 0;
                                                            var_c0 = r10d;
                                                            rax = rdi;
                                                            var_e8 = rbx;
                                                            xmm0 = 0;
                                                            *(__m128*)(rbx) = xmm0;
                                                            *(u64*)((rbx + 0xADDR)) = rcx;
                                                            r11d = (r10 + 1);
                                                            rcx = &var_da0;
                                                            var_260 = rax;
                                                            var_128 = rcx;
                                                            r8d = r10d;
                                                            rdx = rbx;
                                                            do {
                                                                r9d = (uint32)(*(u16*)(rcx));
                                                                rcx = 0xADDR;
                                                                eax = r10d;
                                                                while (r9w != *(u16*)(rcx)) {
                                                                    eax += r11d;
                                                                    rcx += 2;
                                                                    if (eax < 6) continue;
                                                                }
                                                                rax = *(u64*)(((rbp + (rax * 8)) + 0xADDR));
                                                                r9 = -1;
                                                                var_c8 = rax;
                                                                do {
                                                                    r9 += 1;
                                                                } while (*(u8*)((rax + r9)) != r10b);
                                                                r8 = rax;
                                                                var_28 = r10d;
                                                                rax = qword_ADDR;
                                                                r9d += r11d;
                                                                edx = 0;
                                                                var_20 = r10;
                                                                sub_140028010(0xADDR);
                                                                var_f0 = eax;
                                                                if (eax != 0) {
                                                                    rcx = (int64)(eax);
                                                                    rcx += rcx;
                                                                    sub_140024bf8(rcx);
                                                                    rcx = var_e8;
                                                                    r10d = 0;
                                                                    *(u64*)(rcx) = rax;
                                                                    if (rax != 0) {
                                                                        r8 = var_c8;
                                                                        r9 = -1;
                                                                        do {
                                                                            r9 += 1;
                                                                        } while (*(u8*)((r8 + r9)) != r10b);
                                                                        ecx = var_f0;
                                                                        r9d += 1;
                                                                        var_28 = ecx;
                                                                        edx = 0;
                                                                        var_20 = rax;
                                                                        rax = qword_ADDR;
                                                                        sub_140028010(0xADDR);
                                                                        r10d = 0;
                                                                        if (eax != 0) {
                                                                            rdx = var_e8;
                                                                            r11d = (r10 + 1);
                                                                            r8d = var_c0;
                                                                        }
                                                                    }
                                                                }
                                                                loc_ADDR:
                                                                rcx = var_128;
                                                                r8d += r11d;
                                                                rcx += 2;
                                                                var_c0 = r8d;
                                                                rdx += 8;
                                                                var_128 = rcx;
                                                                var_e8 = rdx;
                                                            } while (r8d < 3);
                                                                goto loc_ADDR;
                                                            rax = rbx;
                                                            rbx = r10;
                                                            var_238 = rax;
                                                            var_810 = var_f8;
                                                            var_268 = rbx;
                                                        }
                                                        loc_ADDR:
                                                        if (rbx != 0) {
                                                            r10 = var_a0;
                                                            rcx = rdi;
                                                            r9 = var_b8;
                                                            ecx = 3;
                                                            r11 = var_e0;
                                                            r8 = r10;
                                                            eax = (rcx - 2);
                                                            var_90 = r10;
                                                            r10d = 0;
                                                            var_b8 = rcx;
                                                            var_88 = r11;
                                                            var_68 = r9;
                                                            var_260 = rdi;
                                                            do {
                                                                rdi = *(u64*)(rbx);
                                                                if (rdi != 0) {
                                                                    rcx = -1;
                                                                    do {
                                                                        rcx += 1;
                                                                    } while (*(u16*)((rdi + (rcx * 2))) != r10w);
                                                                    rcx = ((rcx * 2) + 2);
                                                                    eax = 0;
                                                                    __asm("rep stosb [rdi]");
                                                                    sub_140024e2c(*(u64*)(rbx));
                                                                    rcx = var_b8;
                                                                    r10d = 0;
                                                                    eax = (r10 + 1);
                                                                }
                                                                rbx += 8;
                                                                rcx -= rax;
                                                                var_b8 = rcx;
                                                            } while (rcx != 0);
                                                        }
                                                    } else {
                                                        r12d = 0xADDR;
                                                        sub_140024e2c(r11);
                                                    }
                                                    rbx = var_88;
                                                } else {
                                                    r12d = 0xADDR;
                                                }
                                                r8 = var_90;
                                            }
                                            rdi = var_258;
                                            rcx = var_260;
                                            var_88 = rbx;
                                            var_90 = r8;
                                            if (rdi != 0) {
                                                eax = 0;
                                                __asm("rep stosb [rdi]");
                                            }
                                            rdi = var_68;
                                            var_68 = rdi;
                                            sub_140024c8c(&var_268);
                                            sub_140024c8c(&var_258);
                                            sub_140024c8c(&var_250);
                                            edx = 0;
                                            if (r12d < 0 /* signed */) {
                                                edi = 0;
                                            } else {
                                                rcx = var_90;
                                                if (var_238 != rdx) {
                                                    rax = var_238;
                                                    var_88 = rbx;
                                                    var_90 = rcx;
                                                    var_68 = rdi;
                                                } else {
                                                    eax = edx;
                                                }
                                                rax = *(u64*)(rax);
                                                var_818 = rax;
                                                __asm("if (jne short 0xADDR /* unmodelled flags */) goto 0xADDR");
                                                rax = rdx;
                                                rax = *(u64*)((rax + 8));
                                                var_820 = rax;
                                                __asm("if (jne short 0xADDR /* unmodelled flags */) goto 0xADDR");
                                                edi = 0;
                                                eax = edi;
                                                rax = *(u64*)((rax + 0xADDR));
                                                r9d = 0;
                                                var_828 = rax;
                                                rax = qword_ADDR;
                                                xmm0 = 0;
                                                var_330 = 0xADDR;
                                                sub_140028010((r9 + 0xADDR), rdx, &var_330);
                                                if (eax != 0) {
                                                    eax = var_334;
                                                    ebx = var_148;
                                                    eax &= 1;
                                                    ecx = ebx;
                                                    var_814 = eax;
                                                    ecx -= 1;
                                                    if (ecx == 0) {
                                                        var_78 = 0xADDR;
                                                        loc_ADDR:
                                                        var_108 = 0xADDR;
                                                    } else {
                                                        ecx -= 1;
                                                        if (ecx == 0) {
                                                            var_78 = 0xADDR;
                                                            goto loc_ADDR;
                                                        } else if (ecx == 1) {
                                                            r12d = 0xADDR;
                                                            var_78 = r12d;
                                                            var_108 = r12d;
                                                        }
                                                    }
                                                    edi = 0xADDR;
                                                    edx = 0;
                                                    var_c0 = edi;
                                                    r12d = (rdi + 0xADDR);
                                                    r8d = r12d;
                                                    sub_1400029dc(&var_bb0);
                                                    rax = qword_ADDR;
                                                    edx = -1;
                                                    var_bf4 = r12w;
                                                    ecx = 0;
                                                    sub_140028010(0, rdx, &var_bb0);
                                                    r12d = 0;
                                                    if (eax != 0) {
                                                        eax = var_c5c;
                                                        edi = (uint32)(var_c56);
                                                        var_328 = eax;
                                                        var_32c = var_c60;
                                                        eax = (r12 + 0xADDR);
                                                        var_320 = r12;
                                                        edi = ((edi < eax) ? eax : edi);
                                                        var_c0 = edi;
                                                    }
                                                    rax = var_88;
                                                    ecx = 1;
                                                    rax = ((ebx == ecx) ? var_68 : rax);
                                                    edx = 0;
                                                    r8d = (rcx + 0xADDR);
                                                    var_b8 = rax;
                                                    sub_1400029dc(&var_a90, 0, (rcx + 0xADDR));
                                                    rax = qword_ADDR;
                                                    edx = 0;
                                                    ecx = 0;
                                                    sub_140028010(0, 0, 0xADDR);
                                                    var_a0 = rax;
                                                    if (rax != 0) {
                                                        rax = qword_ADDR;
                                                        sub_140028010(var_78, edi, 0xADDR);
                                                        eax = -(eax);
                                                        var_aa0 = 0xADDR;
                                                        var_a90 = eax;
                                                        var_aaa = 5;
                                                        sub_140008384(&var_aac, 0xADDR, var_b8);
                                                        rax = qword_ADDR;
                                                        sub_140028010(&var_a90);
                                                        if (rax != 0) {
                                                            var_830 = rax;
                                                        } else {
                                                            GetLastError();
                                                            r12d = eax;
                                                            if (eax > 0 /* signed */) {
                                                                r12d = (uint32)(r12w);
                                                                r12d |= 0xADDR;
                                                            }
                                                            __asm("if (js short 0xADDR /* unmodelled flags */) goto 0xADDR");
                                                            r12d = 0xADDR;
                                                        }
                                                        ecx = 0;
                                                        rax = qword_ADDR;
                                                        sub_140028010(0, var_a0);
                                                        if (r12d >= 0 /* signed */) {
                                                            eax = 1;
                                                            rbx = var_88;
                                                            rbx = ((ebx == eax) ? var_68 : rbx);
                                                            edx = 0;
                                                            sub_1400029dc(&var_af0, 0, (rax + 0xADDR));
                                                            eax = 0;
                                                            edx = 0;
                                                            r12d = eax;
                                                            ecx = 0;
                                                            rax = qword_ADDR;
                                                            sub_140028010(0, 0, 0xADDR);
                                                            var_68 = rax;
                                                            if (rax == 0) {
                                                                GetLastError();
                                                                edi = 0;
                                                                r12d = eax;
                                                                if (eax > 0 /* signed */) {
                                                                    r12d = (uint32)(ax);
                                                                    r12d |= 0xADDR;
                                                                }
                                                                if (r12d >= 0 /* signed */) {
                                                                    r12d = 0xADDR;
                                                                }
                                                            } else {
                                                                rax = qword_ADDR;
                                                                sub_140028010(var_108, edi, 0xADDR);
                                                                eax = -(eax);
                                                                var_b00 = 0xADDR;
                                                                r8 = rbx;
                                                                var_af0 = eax;
                                                                var_b0a = 5;
                                                                sub_140008384(&var_b0c, 0xADDR);
                                                                rax = qword_ADDR;
                                                                sub_140028010(&var_af0);
                                                                edi = 0;
                                                                if (rax != 0) {
                                                                    var_838 = rax;
                                                                } else {
                                                                    GetLastError();
                                                                    r12d = eax;
                                                                    if (eax > 0 /* signed */) {
                                                                        r12d = (uint32)(ax);
                                                                        r12d |= 0xADDR;
                                                                    }
                                                                    if (r12d >= 0 /* signed */) {
                                                                        r12d = 0xADDR;
                                                                    }
                                                                }
                                                                ecx = 0;
                                                                rax = qword_ADDR;
                                                                sub_140028010(0, var_68);
                                                                if (r12d >= 0 /* signed */) {
                                                                    ebx = edi;
                                                                    edx = edi;
                                                                    edi = var_148;
                                                                    ecx = (edi - 1);
                                                                    if (ecx == 0) {
                                                                        ebx = var_328;
                                                                        edx = var_32c;
                                                                        loc_ADDR:
                                                                        r8d = 0;
                                                                    } else {
                                                                        ecx -= 1;
                                                                        if (ecx == 0) {
                                                                            eax = var_328;
                                                                            ebx = ((rdx + rax) >> 2);
                                                                            r8d = 0;
                                                                            edx = r8d;
                                                                        } else {
                                                                            if (ecx != 1) goto loc_ADDR;
                                                                        }
                                                                    }
                                                                    var_318 = ebx;
                                                                    var_31c = edx;
                                                                    ecx = (edi - 1);
                                                                    if (ecx == 0) {
                                                                        eax = 0xADDR;
                                                                        var_118 = 0xADDR;
                                                                        var_a8 = eax;
                                                                        var_130 = 0xADDR;
                                                                    } else {
                                                                        ecx -= 1;
                                                                        if (ecx == 0) {
                                                                            eax = r8d;
                                                                            var_118 = r8d;
                                                                            var_a8 = eax;
                                                                            var_130 = r8d;
                                                                        } else if (ecx != 1) {
                                                                            eax = var_a8;
                                                                        }
                                                                    }
                                                                    ebx -= eax;
                                                                    rax = qword_ADDR;
                                                                    rdi = var_818;
                                                                    var_864 = ebx;
                                                                    var_854 = ebx;
                                                                    sub_140028010(var_80, var_830);
                                                                    var_68 = rax;
                                                                    r8d = -1;
                                                                    eax = 0;
                                                                    var_28 = rax;
                                                                    rax = qword_ADDR;
                                                                    var_20 = 0xADDR;
                                                                    sub_140028010(var_80, rdi, r8, &var_84c, 0xADDR, 0);
                                                                    edi = eax;
                                                                    ecx = eax;
                                                                    rax = var_68;
                                                                    ecx = -(ecx);
                                                                    r12d = ((ecx != 0) ? -1 : 0);
                                                                    r12d = (~(r12d) & 0xADDR);
                                                                    if (rax != 0) {
                                                                        rdx = rax;
                                                                        rax = qword_ADDR;
                                                                        sub_140028010(var_80);
                                                                    }
                                                                    if (edi != 0) {
                                                                        rax = qword_ADDR;
                                                                        rdi = var_820;
                                                                        sub_140028010(var_80, var_838);
                                                                        var_68 = rax;
                                                                        r8d = -1;
                                                                        eax = 0;
                                                                        var_28 = rax;
                                                                        rax = qword_ADDR;
                                                                        var_20 = 0xADDR;
                                                                        sub_140028010(var_80, rdi, r8, &var_85c, 0xADDR, 0);
                                                                        edi = eax;
                                                                        ecx = eax;
                                                                        rax = var_68;
                                                                        ecx = -(ecx);
                                                                        r12d = ((ecx != 0) ? -1 : 0);
                                                                        edx = 0;
                                                                        r12d = (~(r12d) & 0xADDR);
                                                                        if (rax != 0) {
                                                                            rdx = rax;
                                                                            rax = qword_ADDR;
                                                                            sub_140028010(var_80);
                                                                            edx = 0;
                                                                        }
                                                                        if (edi != 0) {
                                                                            edi = var_148;
                                                                            ecx = var_854;
                                                                            eax = (rdi - 2);
                                                                            if (eax <= 1) {
                                                                                ebx = var_864;
                                                                                ebx = ((ecx > ebx /* signed */) ? ecx : ebx);
                                                                            }
                                                                            r8d = var_118;
                                                                            rax = qword_ADDR;
                                                                            if (var_810 == edx) {
                                                                                sub_140028010(&var_84c, var_a8);
                                                                                edx = var_a8;
                                                                            } else {
                                                                                edx = ebx;
                                                                                edx -= ecx;
                                                                                sub_140028010(&var_84c);
                                                                                edx = ebx;
                                                                                edx -= var_864;
                                                                            }
                                                                            r8d = var_858;
                                                                            r8d += var_130;
                                                                            rax = qword_ADDR;
                                                                            sub_140028010(&var_85c);
                                                                            eax = (rdi - 2);
                                                                            ecx = 1;
                                                                            if (eax <= 1) {
                                                                                eax = var_868;
                                                                                var_31c = eax;
                                                                                var_318 = ebx;
                                                                                var_848 = 0xADDR;
                                                                                loc_ADDR:
                                                                                eax = var_810;
                                                                                r12d = var_31c;
                                                                                ebx = var_318;
                                                                                var_c0 = eax;
                                                                                edi -= 1;
                                                                                if (edi == 0) {
                                                                                    edi = 0;
                                                                                    var_310 = rdi;
                                                                                } else {
                                                                                    edi -= 1;
                                                                                    if (edi == 0) {
                                                                                        eax = 0;
                                                                                        xmm0 = 0;
                                                                                        var_940 = rax;
                                                                                        edx = ecx;
                                                                                        rax = qword_ADDR;
                                                                                        ecx = 0;
                                                                                        var_920 = xmm0;
                                                                                        var_930 = xmm0;
                                                                                        sub_140028010();
                                                                                        rcx = rax;
                                                                                        var_920 = 0xADDR;
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010(rax, &var_920);
                                                                                        edi = 0;
                                                                                        if (eax != 0) {
                                                                                            rax = qword_ADDR;
                                                                                            ecx = 0;
                                                                                            sub_140028010(0, &var_934);
                                                                                            rax = qword_ADDR;
                                                                                            ecx = 0;
                                                                                            sub_140028010(0, &var_93c);
                                                                                            eax = var_93c;
                                                                                            if (var_c0 == edi) {
                                                                                                eax -= ebx;
                                                                                                ecx = (eax * 0xADDR);
                                                                                                eax = 0xADDR;
                                                                                                __asm("imul ecx");
                                                                                                edx >>= 5;
                                                                                                eax = (edx >> 0xADDR);
                                                                                                edx += eax;
                                                                                            } else {
                                                                                                ecx = (rax + (rax * 4));
                                                                                                eax = 0xADDR;
                                                                                                __asm("imul ecx");
                                                                                                edx >>= 5;
                                                                                                eax = (edx >> 0xADDR);
                                                                                                edx += eax;
                                                                                                edx += var_934;
                                                                                            }
                                                                                            rax = (var_93c >> 0xADDR);
                                                                                            eax -= r12d;
                                                                                            ecx = (eax * 0xADDR);
                                                                                            loc_ADDR:
                                                                                            var_310 = edx;
                                                                                            eax = 0xADDR;
                                                                                            __asm("imul ecx");
                                                                                            edx >>= 5;
                                                                                            eax = (edx >> 0xADDR);
                                                                                            edx += eax;
                                                                                            var_314 = edx;
                                                                                        }
                                                                                    } else if (edi != 1) {
                                                                                        edi = 0;
                                                                                    } else {
                                                                                        eax = 0;
                                                                                        xmm0 = 0;
                                                                                        var_968 = rax;
                                                                                        edx = ecx;
                                                                                        rax = qword_ADDR;
                                                                                        ecx = 0;
                                                                                        var_948 = xmm0;
                                                                                        var_958 = xmm0;
                                                                                        sub_140028010();
                                                                                        rcx = rax;
                                                                                        var_948 = 0xADDR;
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010(rax, &var_948);
                                                                                        edi = 0;
                                                                                        if (eax != 0) {
                                                                                            rax = qword_ADDR;
                                                                                            ecx = 0;
                                                                                            sub_140028010(0, &var_95c);
                                                                                            rax = qword_ADDR;
                                                                                            ecx = 0;
                                                                                            sub_140028010(0, &var_964);
                                                                                            eax = var_964;
                                                                                            eax -= ebx;
                                                                                            ecx = (eax * 0xADDR);
                                                                                            eax = 0xADDR;
                                                                                            __asm("imul ecx");
                                                                                            edx >>= 5;
                                                                                            ecx = (edx >> 0xADDR);
                                                                                            edx += ecx;
                                                                                            rcx = (var_964 >> 0xADDR);
                                                                                            ecx -= r12d;
                                                                                            ecx *= 0xADDR;
                                                                                            goto loc_ADDR;
                                                                                        }
                                                                                    }
                                                                                }
                                                                                r12d = edi;
                                                                            } else if (edi == ecx) {
                                                                                r8d = (rcx + 0xADDR);
                                                                                var_848 = 0xADDR;
                                                                                edx = 0;
                                                                                sub_1400029dc(&var_b50, 0, (rcx + 0xADDR));
                                                                                eax = 0;
                                                                                edx = 0;
                                                                                r12d = eax;
                                                                                ecx = 0;
                                                                                rax = qword_ADDR;
                                                                                sub_140028010(0, 0, 0xADDR);
                                                                                rdi = rax;
                                                                                if (rax != 0) {
                                                                                    r8d = 0xADDR;
                                                                                    rax = qword_ADDR;
                                                                                    sub_140028010((r8 - 0xADDR), var_c0, 0xADDR);
                                                                                    eax = -(eax);
                                                                                    var_b60 = 0xADDR;
                                                                                    var_b50 = eax;
                                                                                    var_b6a = 5;
                                                                                    sub_140008384(&var_b6c, 0xADDR, var_90);
                                                                                    rax = qword_ADDR;
                                                                                    sub_140028010(&var_b50);
                                                                                    if (rax != 0) {
                                                                                        var_840 = rax;
                                                                                    } else {
                                                                                        GetLastError();
                                                                                        r12d = eax;
                                                                                        if (eax > 0 /* signed */) {
                                                                                            r12d = (uint32)(r12w);
                                                                                            r12d |= 0xADDR;
                                                                                        }
                                                                                        __asm("if (js short 0xADDR /* unmodelled flags */) goto 0xADDR");
                                                                                        r12d = 0xADDR;
                                                                                    }
                                                                                    rax = qword_ADDR;
                                                                                    rdx = rdi;
                                                                                    ecx = 0;
                                                                                    sub_140028010(0, rdi);
                                                                                    edi = 0;
                                                                                    if (r12d >= 0 /* signed */) {
                                                                                        rax = qword_ADDR;
                                                                                        rdi = var_828;
                                                                                        var_874 = ebx;
                                                                                        sub_140028010(var_80, var_840);
                                                                                        rbx = rax;
                                                                                        r8d = -1;
                                                                                        eax = 0;
                                                                                        var_28 = rax;
                                                                                        rax = qword_ADDR;
                                                                                        var_20 = 0xADDR;
                                                                                        sub_140028010(var_80, rdi, r8, &var_86c, 0xADDR, 0);
                                                                                        edi = eax;
                                                                                        ecx = eax;
                                                                                        ecx = -(ecx);
                                                                                        r12d = ((ecx != 0) ? -1 : 0);
                                                                                        r12d = (~(r12d) & 0xADDR);
                                                                                        if (rbx != 0) {
                                                                                            rdx = rbx;
                                                                                            rax = qword_ADDR;
                                                                                            sub_140028010(var_80);
                                                                                        }
                                                                                        ebx = 0;
                                                                                        if (edi != 0) {
                                                                                            eax = var_874;
                                                                                            ecx = (rbx + 0xADDR);
                                                                                            edi = var_c0;
                                                                                            r12d = (rbx + 0xADDR);
                                                                                            eax += 0xADDR;
                                                                                            r8d = r12d;
                                                                                            edx = edi;
                                                                                            eax = ((eax < ecx /* signed */) ? ecx : eax);
                                                                                            var_874 = eax;
                                                                                            eax = (rbx + 0xADDR);
                                                                                            var_878 = eax;
                                                                                            rax = qword_ADDR;
                                                                                            sub_140028010(var_86c);
                                                                                            r8d = r12d;
                                                                                            var_86c = eax;
                                                                                            edx = edi;
                                                                                            rax = qword_ADDR;
                                                                                            sub_140028010(var_870);
                                                                                            r8d = r12d;
                                                                                            var_870 = eax;
                                                                                            edx = edi;
                                                                                            rax = qword_ADDR;
                                                                                            sub_140028010(var_874);
                                                                                            r8d = r12d;
                                                                                            var_874 = eax;
                                                                                            edx = edi;
                                                                                            rax = qword_ADDR;
                                                                                            sub_140028010(var_878);
                                                                                            r8d = var_868;
                                                                                            rcx = &var_86c;
                                                                                            r8d += 0xADDR;
                                                                                            var_878 = eax;
                                                                                            rax = qword_ADDR;
                                                                                            if (var_810 == ebx) {
                                                                                                edx = var_854;
                                                                                                edx = ((edx <= var_864 /* signed */) ? var_864 : edx);
                                                                                                edx -= var_874;
                                                                                                edx += var_86c;
                                                                                            } else {
                                                                                                edx = var_84c;
                                                                                                edx = ((edx >= var_85c /* signed */) ? var_85c : edx);
                                                                                            }
                                                                                            sub_140028010();
                                                                                            edi = var_148;
                                                                                            ecx = 1;
                                                                                            goto loc_ADDR;
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        GetLastError();
                                                        r12d = eax;
                                                        if (eax > 0 /* signed */) {
                                                            r12d = (uint32)(r12w);
                                                            r12d |= 0xADDR;
                                                        }
                                                        __asm("if (js 0xADDR /* unmodelled flags */) goto 0xADDR");
                                                        r12d = 0xADDR;
                                                    }
                                                } else {
                                                    GetLastError();
                                                    r12d = eax;
                                                    if (eax > 0 /* signed */) {
                                                        r12d = (uint32)(ax);
                                                        r12d |= 0xADDR;
                                                    }
                                                    var_814 = edi;
                                                    eax = 0xADDR;
                                                    r12d = ((r12d >= 0 /* signed */) ? eax : r12d);
                                                }
                                            }
                                            ecx = 0;
                                            rax = qword_ADDR;
                                            sub_140028010(0, var_80);
                                        } else {
                                            GetLastError();
                                            edi = 0;
                                            r12d = eax;
                                            if (eax > 0 /* signed */) {
                                                r12d = (uint32)(ax);
                                                r12d |= 0xADDR;
                                            }
                                            if (r12d >= 0 /* signed */) {
                                                r12d = 0xADDR;
                                            }
                                        }
                                        ebx = 1;
                                        do {
                                            eax = 0;
                                            __asm("lock cmpxchg [0xADDR], ebx");
                                        } while (dword_ADDR != eax);
                                        eax = dword_ADDR;
                                        if (eax > 0 /* signed */) {
                                            eax -= ebx;
                                            dword_ADDR = eax;
                                            if (eax == 0) {
                                                ebx = 4;
                                                rdi = off_1400360c0;
                                                r14d = (rbx - 3);
                                                do {
                                                    rcx = *(u64*)(rdi);
                                                    if (rcx != 0) {
                                                        FreeLibrary(*(u64*)(rdi));
                                                    }
                                                    rdi += 0xADDR;
                                                    rbx -= r14;
                                                } while (rbx != 0);
                                                edx = 0;
                                                sub_1400029dc(off_1400360c0, 0, (rbx + 0xADDR));
                                                edx = (rbx + 2);
                                                r14 = 0xADDR;
                                                rax = off_140034000;
                                                rcx = off_140029260;
                                                edi = (rdx + 0xADDR);
                                                do {
                                                    xmm0 = *(__m128*)(rcx);
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)(rax) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    rcx += rdi;
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    rax += rdi;
                                                    *(__m128*)((rax - 0xADDR)) = xmm1;
                                                    rdx -= 1;
                                                } while (rdx != 0);
                                                xmm0 = *(__m128*)(rcx);
                                                edi = 0;
                                                ebx = (rdx + 1);
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)(rax) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                            }
                                        }
                                        eax = edi;
                                        rcx = &var_238;
                                        __asm("xchg [0xADDR], eax");
                                        sub_140024c8c(&var_238);
                                        if (r12d < 0 /* signed */) {
                                            r12d = 0;
                                        } else {
                                            rdi = var_310;
                                            eax = 0;
                                            var_290 = rdi;
                                            esi &= 0xADDR;
                                            var_5e0 = eax;
                                            edx = 0;
                                            var_f0 = esi;
                                            sub_1400029dc(&var_5e4, 0, (rax + 0xADDR));
                                            rax = qword_ADDR;
                                            sub_140028010(var_d0, 7);
                                            r12d = 0;
                                            if (rax == 0) {
                                                ebx = r12d;
                                                loc_ADDR:
                                                if (esi != 1) {
                                                    edi = 1;
                                                } else {
                                                    rax = qword_ADDR;
                                                    sub_140028010(&var_84c, edi, var_294);
                                                    rax = qword_ADDR;
                                                    sub_140028010(&var_85c, edi, var_294);
                                                    edi = 1;
                                                    if (esi == 1) {
                                                        ecx = var_814;
                                                        rax = qword_ADDR;
                                                        ecx = -(ecx);
                                                        ecx = ((ecx != 0) ? -1 : 0);
                                                        ecx = (~(ecx) & 0xADDR);
                                                        sub_140028010(ecx);
                                                        var_98 = rax;
                                                    }
                                                }
                                            } else {
                                                rcx = rax;
                                                rax = qword_ADDR;
                                                sub_140028010(rax, (r12 + 0xADDR), &var_5e0);
                                                if (eax != 0) {
                                                    eax = (r12 + 0xADDR);
                                                    if (var_5f2 != ax) {
                                                        ebx = r12d;
                                                    }
                                                    if (ebx == 0) {
                                                        goto loc_ADDR;
                                                    }
                                                }
                                            }
                                            rax = qword_ADDR;
                                            edx = edi;
                                            rdi = var_d0;
                                            rcx = rdi;
                                            sub_140028010(rdi, edi);
                                            ecx = 0xADDR;
                                            var_c0 = eax;
                                            eax = 0xADDR;
                                            eax = ((var_810 != r12d) ? ecx : eax);
                                            var_148 = eax;
                                            if (ebx == 0) {
                                                ebx = 1;
                                                if (var_814 != r12d) {
                                                    al = 0xADDR;
                                                } else {
                                                    al = 0xADDR;
                                                }
                                                var_fa = al;
                                                rcx = rdi;
                                                rax = qword_ADDR;
                                                var_f8 = r12w;
                                                var_fb = bl;
                                                var_78 = r12d;
                                                var_108 = r12d;
                                                var_a8 = r12d;
                                                sub_140028010();
                                                var_110 = rax;
                                                if (rax != 0) {
                                                    if (esi != ebx) {
                                                        xmm2 = var_84c;
                                                        ebx = var_318;
                                                        xmm3 = var_85c;
                                                        ecx = var_31c;
                                                        xmm0 = xmm2;
                                                        var_118 = ebx;
                                                        xmm0 >>= 0xADDR;
                                                        xmm1 = xmm2;
                                                        var_c8 = xmm0;
                                                        xmm0 = (xmm2 >> 4);
                                                        var_108 = xmm0;
                                                        xmm0 = (xmm3 >> 0xADDR);
                                                        xmm1 >>= 8;
                                                        var_e0 = xmm0;
                                                        xmm0 = xmm3;
                                                        edi = xmm1;
                                                        xmm1 = xmm3;
                                                        xmm0 >>= 4;
                                                        xmm1 >>= 8;
                                                        var_e8 = xmm0;
                                                        var_130 = ecx;
                                                        var_78 = xmm2;
                                                        r12d = xmm1;
                                                        var_a8 = xmm3;
                                                    } else {
                                                        edi = var_854;
                                                        eax = edi;
                                                        r12d = var_864;
                                                        ebx = r12d;
                                                        edx = var_84c;
                                                        eax -= edx;
                                                        r8d = var_85c;
                                                        ebx -= r8d;
                                                        ecx = var_868;
                                                        r9d = var_850;
                                                        ebx = ((ebx <= eax /* signed */) ? eax : ebx);
                                                        ecx -= r9d;
                                                        var_118 = ebx;
                                                        r10d = eax;
                                                        var_130 = ecx;
                                                        r11d = eax;
                                                        if (var_810 != 0) {
                                                            ecx = edx;
                                                            eax = r8d;
                                                            ecx -= edi;
                                                            eax -= r12d;
                                                            ecx += ebx;
                                                            eax += ebx;
                                                            var_78 = ecx;
                                                            r10d = ecx;
                                                            var_a8 = eax;
                                                            ecx = var_130;
                                                            r11d = eax;
                                                        }
                                                        eax = var_858;
                                                        edi -= edx;
                                                        eax -= r9d;
                                                        r12d -= r8d;
                                                        var_c8 = eax;
                                                        edi += r10d;
                                                        eax = var_860;
                                                        r12d += r11d;
                                                        eax -= r9d;
                                                        var_e8 = eax;
                                                        eax -= var_860;
                                                        eax += var_868;
                                                        var_e0 = eax;
                                                    }
                                                    rax = qword_ADDR;
                                                    edx = 0;
                                                    var_128 = r12d;
                                                    ecx = -(ecx);
                                                    var_28 = edx;
                                                    var_20 = rdx;
                                                    xmm0 = 0;
                                                    var_2a8 = rdx;
                                                    r8d = 0;
                                                    var_a50 = rdx;
                                                    var_a58 = edx;
                                                    var_a38 = ecx;
                                                    var_a40 = xmm0;
                                                    var_a30 = 0xADDR;
                                                    var_a34 = ebx;
                                                    var_a3c = 0xADDR;
                                                    sub_140028010(var_110, &var_a30, 0, &var_2a8);
                                                    var_100 = rax;
                                                    if (rax != 0) {
                                                        rdx = rax;
                                                        rax = qword_ADDR;
                                                        sub_140028010(var_110);
                                                        if (esi == 1) {
                                                            r8 = var_98;
                                                            eax = 0;
                                                            if (r8 != 0) {
                                                                var_8d0 = rax;
                                                                eax = var_130;
                                                                var_8dc = eax;
                                                                rax = qword_ADDR;
                                                                var_8d8 = ebx;
                                                                sub_140028010(var_110, &var_8d0);
                                                            }
                                                        }
                                                        var_a0 = var_814;
                                                        edx = 0;
                                                        var_68 = var_830;
                                                        rax = var_818;
                                                        var_90 = rax;
                                                        eax = 0;
                                                        var_6c0 = eax;
                                                        sub_1400029dc(&var_6c4, 0, (rax + 0xADDR));
                                                        eax = 0;
                                                        edx = 0;
                                                        var_730 = eax;
                                                        sub_1400029dc(&var_734, 0, (rax + 0xADDR));
                                                        ecx = 0;
                                                        eax = 0;
                                                        xmm0 = 0;
                                                        var_88 = rcx;
                                                        var_b8 = ecx;
                                                        var_80 = ecx;
                                                        var_a00 = xmm0;
                                                        var_a20 = rax;
                                                        var_a10 = xmm0;
                                                        var_a28 = eax;
                                                        var_8e0 = xmm0;
                                                        if (var_90 == rcx) {
                                                            rdi = var_110;
                                                            loc_ADDR:
                                                            r12d = 0;
                                                            rcx = var_100;
                                                            loc_ADDR:
                                                            rax = qword_ADDR;
                                                            sub_140028010();
                                                        } else {
                                                            rax = qword_ADDR;
                                                            edx = (rcx + 7);
                                                            sub_140028010(var_110, (rcx + 7));
                                                            if (rax != 0) {
                                                                rcx = rax;
                                                                rax = qword_ADDR;
                                                                sub_140028010(rax, 0xADDR, &var_6c0);
                                                                if (eax != 0) {
                                                                    var_88 = var_6d8;
                                                                    var_b8 = var_6c4;
                                                                    var_80 = var_6c8;
                                                                }
                                                                rax = qword_ADDR;
                                                                sub_140028010(var_110);
                                                                edx = 0;
                                                                var_140 = rax;
                                                                rcx = rax;
                                                                if (rax == 0) {
                                                                    GetLastError();
                                                                } else {
                                                                    var_28 = edx;
                                                                    eax = edi;
                                                                    var_2b0 = rdx;
                                                                    eax -= var_78;
                                                                    xmm0 = 0;
                                                                    var_a04 = eax;
                                                                    r8d = 0;
                                                                    eax = var_108;
                                                                    eax -= var_c8;
                                                                    var_a08 = eax;
                                                                    rax = qword_ADDR;
                                                                    var_a20 = rdx;
                                                                    var_a28 = edx;
                                                                    var_20 = rdx;
                                                                    var_a10 = xmm0;
                                                                    var_a00 = 0xADDR;
                                                                    var_a0c = 0xADDR;
                                                                    sub_140028010(rcx, &var_a00, 0, &var_2b0);
                                                                    var_b0 = rax;
                                                                    if (rax != 0) {
                                                                        rcx = rax;
                                                                        rax = qword_ADDR;
                                                                        sub_140028010(rax, 0xADDR, &var_730);
                                                                        edx = 0;
                                                                        ecx = edx;
                                                                        eax = var_c8;
                                                                        rcx = ((eax != 0) ? var_748 : rcx);
                                                                        eax -= var_108;
                                                                        edi -= var_78;
                                                                        var_8e8 = edi;
                                                                        rdi = var_140;
                                                                        var_70 = rcx;
                                                                        var_8ec = eax;
                                                                        rax = qword_ADDR;
                                                                        sub_140028010(rdi, var_b0);
                                                                        rax = qword_ADDR;
                                                                        sub_140028010(rdi, 1);
                                                                        rax = qword_ADDR;
                                                                        sub_140028010(rdi, var_68);
                                                                        rdi = qword_ADDR;
                                                                        ecx = 0;
                                                                        var_68 = rax;
                                                                        rax = qword_ADDR;
                                                                        sub_140028010();
                                                                        r8 = rax;
                                                                        rax = rdi;
                                                                        sub_140028010(var_140, &var_8e0);
                                                                        eax = var_148;
                                                                        edi = 0;
                                                                        r8d = -1;
                                                                        var_28 = rdi;
                                                                        var_20 = eax;
                                                                        rax = qword_ADDR;
                                                                        sub_140028010(var_140, var_90, r8, &var_8e0, var_148, 0);
                                                                        eax = (rdi + 1);
                                                                        if (esi != eax) {
                                                                            eax = 0xADDR;
                                                                        } else if (var_a0 == edi) {
                                                                            eax = 0xADDR;
                                                                        } else {
                                                                            rax = qword_ADDR;
                                                                            sub_140028010((rdi + 8));
                                                                        }
                                                                        r11 = (int64)(var_b8);
                                                                        r10d = edi;
                                                                        edx = var_8e8;
                                                                        r8d = edi;
                                                                        var_a0 = eax;
                                                                        eax = var_78;
                                                                        r9d = eax;
                                                                        r9d = -(r9d);
                                                                        r10d = ((eax >= 0 /* signed */) ? eax : r10d);
                                                                        r9d = ((eax >= 0 /* signed */) ? edi : r9d);
                                                                        eax = var_108;
                                                                        ecx = eax;
                                                                        ecx = -(ecx);
                                                                        r8d = ((eax >= 0 /* signed */) ? eax : r8d);
                                                                        ecx = ((eax >= 0 /* signed */) ? edi : ecx);
                                                                        edi = var_80;
                                                                        edx -= r9d;
                                                                        eax = r11d;
                                                                        eax -= r10d;
                                                                        edx = ((edx >= eax /* signed */) ? eax : edx);
                                                                        eax = var_8ec;
                                                                        edi -= r8d;
                                                                        var_b8 = edx;
                                                                        eax -= ecx;
                                                                        eax = ((eax >= edi /* signed */) ? edi : eax);
                                                                        edi = 0;
                                                                        var_80 = eax;
                                                                        if (edx > 0 /* signed */) {
                                                                            edx = 0;
                                                                            if (eax > 0 /* signed */) {
                                                                                ecx *= var_8e8;
                                                                                esi = (rdi + 1);
                                                                                r15d = var_b8;
                                                                                ebx = 0;
                                                                                r14d = var_a0;
                                                                                r12 = r11;
                                                                                r8d *= r11d;
                                                                                r11d = ebx;
                                                                                rax = (int64)(r9d);
                                                                                rcx = (int64)(ecx);
                                                                                rcx += rax;
                                                                                var_c8 = ebx;
                                                                                rax = var_70;
                                                                                var_78 = ebx;
                                                                                rdx = (rax + (rcx * 4));
                                                                                rax = (int64)(r10d);
                                                                                rcx = (int64)(r8d);
                                                                                rcx += rax;
                                                                                var_70 = rdx;
                                                                                rax = var_88;
                                                                                rax += (rcx * 4);
                                                                                rcx = ((int64)(var_8e8) << 2);
                                                                                var_88 = rcx;
                                                                                r13 = rcx;
                                                                                r12 <<= 2;
                                                                                var_90 = rax;
                                                                                do {
                                                                                    r10 = rax;
                                                                                    var_b8 = ebx;
                                                                                    r10 -= rdx;
                                                                                    r9 = (rdx + 2);
                                                                                    r8d = ebx;
                                                                                    do {
                                                                                        ecx = (uint32)(*(u8*)((r9 - 1)));
                                                                                        edx = (rax + (rcx * 2));
                                                                                        eax = (uint32)(*(u8*)(r9));
                                                                                        edx += eax;
                                                                                        edx >>= 2;
                                                                                        dl = ~(dl);
                                                                                        if (dl != 0) {
                                                                                            edi = (uint32)(*(u8*)((r9 + r10)));
                                                                                            r11d = 0xADDR;
                                                                                            r8d = (uint32)(dl);
                                                                                            eax = r11d;
                                                                                            ecx = (uint32)(r14b);
                                                                                            ecx -= edi;
                                                                                            ecx *= r8d;
                                                                                            __asm("imul ecx");
                                                                                            edx += ecx;
                                                                                            ecx = (uint32)(var_a1);
                                                                                            edx >>= 7;
                                                                                            eax = (edx >> 0xADDR);
                                                                                            edx += eax;
                                                                                            eax = r11d;
                                                                                            dl += dil;
                                                                                            *(u8*)((r9 + r10)) = dl;
                                                                                            edi = (uint32)(*(u8*)(((r10 + r9) - 1)));
                                                                                            ecx -= edi;
                                                                                            ecx *= r8d;
                                                                                            __asm("imul ecx");
                                                                                            edx += ecx;
                                                                                            ecx = (uint32)(var_a2);
                                                                                            edx >>= 7;
                                                                                            eax = (edx >> 0xADDR);
                                                                                            edx += eax;
                                                                                            eax = r11d;
                                                                                            dl += dil;
                                                                                            *(u8*)(((r10 + r9) - 1)) = dl;
                                                                                            edi = (uint32)(*(u8*)(((r10 + r9) - 2)));
                                                                                            ecx -= edi;
                                                                                            ecx *= r8d;
                                                                                            __asm("imul ecx");
                                                                                            edx += ecx;
                                                                                            ecx = 0xADDR;
                                                                                            edx >>= 7;
                                                                                            eax = (edx >> 0xADDR);
                                                                                            edx += eax;
                                                                                            eax = r11d;
                                                                                            r11d = var_78;
                                                                                            dl += dil;
                                                                                            *(u8*)(((r10 + r9) - 2)) = dl;
                                                                                            edi = (uint32)(*(u8*)(((r10 + r9) + 1)));
                                                                                            ecx -= edi;
                                                                                            ecx *= r8d;
                                                                                            r8d = var_b8;
                                                                                            __asm("imul ecx");
                                                                                            edx += ecx;
                                                                                            edx >>= 7;
                                                                                            eax = (edx >> 0xADDR);
                                                                                            edx += eax;
                                                                                            dl += dil;
                                                                                            *(u8*)(((r10 + r9) + 1)) = dl;
                                                                                        }
                                                                                        r8d += esi;
                                                                                        r9 += 4;
                                                                                        var_b8 = r8d;
                                                                                        edi = r11d;
                                                                                    } while (r8d < r15d /* signed */);
                                                                                    rdx = var_70;
                                                                                    rax = var_90;
                                                                                    rdx += r13;
                                                                                    ecx = var_c8;
                                                                                    rax += r12;
                                                                                    ecx += esi;
                                                                                    var_70 = rdx;
                                                                                    var_90 = rax;
                                                                                    var_c8 = ecx;
                                                                                    var_78 = r11d;
                                                                                } while (ecx < var_80 /* signed */);
                                                                                esi = var_f0;
                                                                                r14 = 0xADDR;
                                                                                r13d = var_d8;
                                                                                ebx = var_118;
                                                                                r12d = var_128;
                                                                            }
                                                                        }
                                                                        rax = qword_ADDR;
                                                                        sub_140028010(var_b0);
                                                                        rax = var_68;
                                                                        if (rax != 0) {
                                                                            rdx = rax;
                                                                            rax = qword_ADDR;
                                                                            sub_140028010(var_140);
                                                                        }
                                                                    } else {
                                                                        GetLastError();
                                                                        edi = eax;
                                                                        if (eax > 0 /* signed */) {
                                                                            edi = (uint32)(di);
                                                                            edi |= 0xADDR;
                                                                        }
                                                                        eax = 0xADDR;
                                                                        __asm("cmovns edi, eax");
                                                                    }
                                                                    rax = qword_ADDR;
                                                                    sub_140028010(var_140);
                                                                    ecx = 0;
                                                                    if (edi >= 0 /* signed */) {
                                                                        eax = var_814;
                                                                        r8d = (rcx + 0xADDR);
                                                                        rdi = var_820;
                                                                        edx = 0;
                                                                        var_80 = eax;
                                                                        rax = var_838;
                                                                        var_490 = ecx;
                                                                        var_70 = rax;
                                                                        var_b0 = rdi;
                                                                        sub_1400029dc(&var_494, 0, (rcx + 0xADDR));
                                                                        eax = 0;
                                                                        edx = 0;
                                                                        var_7a0 = eax;
                                                                        sub_1400029dc(&var_7a4, 0, (rax + 0xADDR));
                                                                        ecx = 0;
                                                                        eax = 0;
                                                                        xmm0 = 0;
                                                                        var_140 = rcx;
                                                                        var_b8 = ecx;
                                                                        rdi = var_110;
                                                                        var_a0 = ecx;
                                                                        var_a60 = xmm0;
                                                                        var_a80 = rax;
                                                                        var_a70 = xmm0;
                                                                        var_a88 = eax;
                                                                        var_8f0 = xmm0;
                                                                        if (rdi != 0) {
                                                                            rax = qword_ADDR;
                                                                            edx = (rcx + 7);
                                                                            sub_140028010(rdi, (rcx + 7));
                                                                            if (rax != 0) {
                                                                                rcx = rax;
                                                                                rax = qword_ADDR;
                                                                                sub_140028010(rax, 0xADDR, &var_490);
                                                                                if (eax != 0) {
                                                                                    var_140 = var_4a8;
                                                                                    var_b8 = var_494;
                                                                                    var_a0 = var_498;
                                                                                }
                                                                                rax = qword_ADDR;
                                                                                rcx = rdi;
                                                                                sub_140028010(rdi);
                                                                                ecx = 0;
                                                                                var_68 = rax;
                                                                                rdi = rax;
                                                                                if (rax != 0) {
                                                                                    var_28 = ecx;
                                                                                    eax = r12d;
                                                                                    var_2c0 = rcx;
                                                                                    eax -= var_a8;
                                                                                    var_a64 = eax;
                                                                                    xmm0 = 0;
                                                                                    eax = var_e8;
                                                                                    r8d = 0;
                                                                                    eax -= var_e0;
                                                                                    var_a68 = eax;
                                                                                    rax = qword_ADDR;
                                                                                    var_a80 = rcx;
                                                                                    var_a88 = ecx;
                                                                                    var_20 = rcx;
                                                                                    var_a70 = xmm0;
                                                                                    var_a60 = 0xADDR;
                                                                                    var_a6c = 0xADDR;
                                                                                    sub_140028010(rdi, &var_a60, 0, &var_2c0);
                                                                                    var_90 = rax;
                                                                                    if (rax != 0) {
                                                                                        rcx = rax;
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010(rax, 0xADDR, &var_7a0);
                                                                                        edx = 0;
                                                                                        ecx = edx;
                                                                                        eax = var_e0;
                                                                                        rcx = ((eax != 0) ? var_7b8 : rcx);
                                                                                        eax -= var_e8;
                                                                                        r12d -= var_a8;
                                                                                        var_8fc = eax;
                                                                                        rax = qword_ADDR;
                                                                                        var_88 = rcx;
                                                                                        var_8f8 = r12d;
                                                                                        sub_140028010(rdi, var_90);
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010(rdi, 1);
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010(rdi, var_70);
                                                                                        rdi = qword_ADDR;
                                                                                        ecx = 0;
                                                                                        var_70 = rax;
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010();
                                                                                        r12 = var_68;
                                                                                        r8 = rax;
                                                                                        rax = rdi;
                                                                                        sub_140028010(r12, &var_8f0);
                                                                                        eax = var_148;
                                                                                        edi = 0;
                                                                                        var_28 = rdi;
                                                                                        r8d = -1;
                                                                                        var_20 = eax;
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010(r12, var_b0, r8, &var_8f0, var_148, 0);
                                                                                        eax = (rdi + 1);
                                                                                        if (esi != eax) {
                                                                                            eax = 0xADDR;
                                                                                            while (1) {
                                                                                                var_80 = eax;
                                                                                                var_120 = eax;
                                                                                            } else if (var_80 == edi) {
                                                                                                var_80 = 0xADDR;
                                                                                            } else {
                                                                                                rax = qword_ADDR;
                                                                                                sub_140028010((rdi + 8));
                                                                                            }
                                                                                        }
                                                                                        eax = var_a8;
                                                                                        r10d = edi;
                                                                                        ecx = var_8f8;
                                                                                        r8d = eax;
                                                                                        r11d = var_8fc;
                                                                                        r8d = -(r8d);
                                                                                        edx = edi;
                                                                                        r10d = ((eax >= 0 /* signed */) ? eax : r10d);
                                                                                        r8d = ((eax >= 0 /* signed */) ? edi : r8d);
                                                                                        eax = var_e8;
                                                                                        r9d = eax;
                                                                                        r9d = -(r9d);
                                                                                        edx = ((eax >= 0 /* signed */) ? eax : edx);
                                                                                        r9d = ((eax >= 0 /* signed */) ? edi : r9d);
                                                                                        eax = var_b8;
                                                                                        ecx -= r8d;
                                                                                        eax -= r10d;
                                                                                        ecx = ((ecx >= eax /* signed */) ? eax : ecx);
                                                                                        eax = var_a0;
                                                                                        eax -= edx;
                                                                                        var_e0 = ecx;
                                                                                        r11d -= r9d;
                                                                                        r11d = ((r11d >= eax /* signed */) ? eax : r11d);
                                                                                        eax = 0;
                                                                                        if (ecx > 0 /* signed */ && r11d > 0 /* signed */) {
                                                                                            rdi = (int64)(var_b8);
                                                                                            ebx = 0;
                                                                                            eax = var_8f8;
                                                                                            r14d = var_e0;
                                                                                            esi = var_120;
                                                                                            eax *= r9d;
                                                                                            edx *= edi;
                                                                                            var_b8 = ebx;
                                                                                            var_a0 = ebx;
                                                                                            rcx = (int64)(eax);
                                                                                            rax = (int64)(r8d);
                                                                                            rcx += rax;
                                                                                            rax = var_88;
                                                                                            r8 = (rax + (rcx * 4));
                                                                                            rax = (int64)(r10d);
                                                                                            rcx = (int64)(edx);
                                                                                            rcx += rax;
                                                                                            var_b0 = r8;
                                                                                            rax = var_140;
                                                                                            rax += (rcx * 4);
                                                                                            rcx = ((int64)(var_8f8) << 2);
                                                                                            var_88 = rcx;
                                                                                            rcx = (rdi << 2);
                                                                                            var_e8 = rcx;
                                                                                            var_140 = rax;
                                                                                            do {
                                                                                                r13d = var_a0;
                                                                                                r9 = (r8 + 2);
                                                                                                r10 = rax;
                                                                                                r15d = ebx;
                                                                                                r10 -= r8;
                                                                                                r12d = 1;
                                                                                                do {
                                                                                                    ecx = (uint32)(*(u8*)((r9 - 1)));
                                                                                                    edx = (rax + (rcx * 2));
                                                                                                    eax = (uint32)(*(u8*)(r9));
                                                                                                    edx += eax;
                                                                                                    edx >>= 2;
                                                                                                    dl = ~(dl);
                                                                                                    if (dl != 0) {
                                                                                                        edi = (uint32)(*(u8*)((r10 + r9)));
                                                                                                        eax = 0xADDR;
                                                                                                        r8d = (uint32)(dl);
                                                                                                        ecx = (uint32)(sil);
                                                                                                        ecx -= edi;
                                                                                                        ecx *= r8d;
                                                                                                        __asm("imul ecx");
                                                                                                        edx += ecx;
                                                                                                        ecx = (uint32)(var_81);
                                                                                                        edx >>= 7;
                                                                                                        eax = (edx >> 0xADDR);
                                                                                                        edx += eax;
                                                                                                        eax = 0xADDR;
                                                                                                        dl += dil;
                                                                                                        *(u8*)((r10 + r9)) = dl;
                                                                                                        edi = (uint32)(*(u8*)(((r10 + r9) - 1)));
                                                                                                        ecx -= edi;
                                                                                                        ecx *= r8d;
                                                                                                        __asm("imul ecx");
                                                                                                        edx += ecx;
                                                                                                        ecx = (uint32)(var_82);
                                                                                                        edx >>= 7;
                                                                                                        eax = (edx >> 0xADDR);
                                                                                                        edx += eax;
                                                                                                        eax = 0xADDR;
                                                                                                        dl += dil;
                                                                                                        *(u8*)(((r10 + r9) - 1)) = dl;
                                                                                                        edi = (uint32)(*(u8*)(((r10 + r9) - 2)));
                                                                                                        ecx -= edi;
                                                                                                        ecx *= r8d;
                                                                                                        __asm("imul ecx");
                                                                                                        edx += ecx;
                                                                                                        ecx = 0xADDR;
                                                                                                        edx >>= 7;
                                                                                                        eax = (edx >> 0xADDR);
                                                                                                        edx += eax;
                                                                                                        eax = 0xADDR;
                                                                                                        dl += dil;
                                                                                                        *(u8*)(((r10 + r9) - 2)) = dl;
                                                                                                        edi = (uint32)(*(u8*)(((r10 + r9) + 1)));
                                                                                                        ecx -= edi;
                                                                                                        ecx *= r8d;
                                                                                                        __asm("imul ecx");
                                                                                                        edx += ecx;
                                                                                                        edx >>= 7;
                                                                                                        eax = (edx >> 0xADDR);
                                                                                                        edx += eax;
                                                                                                        dl += dil;
                                                                                                        *(u8*)(((r10 + r9) + 1)) = dl;
                                                                                                    }
                                                                                                    r9 += 4;
                                                                                                    r15d += r12d;
                                                                                                    edi = r13d;
                                                                                                } while (r15d < r14d /* signed */);
                                                                                                r8 = var_b0;
                                                                                                rax = var_140;
                                                                                                ecx = var_b8;
                                                                                                r8 += var_88;
                                                                                                ecx += 1;
                                                                                                rax += var_e8;
                                                                                                var_b0 = r8;
                                                                                                var_140 = rax;
                                                                                                var_b8 = ecx;
                                                                                                var_a0 = r13d;
                                                                                            } while (ecx < r11d /* signed */);
                                                                                            esi = var_f0;
                                                                                            r14 = 0xADDR;
                                                                                            r13d = var_d8;
                                                                                            ebx = var_118;
                                                                                            r12 = var_68;
                                                                                        }
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010(var_90);
                                                                                        rax = var_70;
                                                                                        if (rax != 0) {
                                                                                            rdx = rax;
                                                                                            rcx = r12;
                                                                                            rax = qword_ADDR;
                                                                                            sub_140028010();
                                                                                        }
                                                                                    } else {
                                                                                        GetLastError();
                                                                                        edi = eax;
                                                                                        if (eax > 0 /* signed */) {
                                                                                            edi = (uint32)(di);
                                                                                            edi |= 0xADDR;
                                                                                        }
                                                                                        eax = 0xADDR;
                                                                                        __asm("cmovns edi, eax");
                                                                                        r12 = var_68;
                                                                                    }
                                                                                    rax = qword_ADDR;
                                                                                    rcx = r12;
                                                                                    sub_140028010(r12);
                                                                                    r12d = 0;
                                                                                    rdi = var_110;
                                                                                    if (edi >= 0 /* signed */) {
                                                                                        r9d = ebx;
                                                                                        eax = var_130;
                                                                                        if (esi != eax) {
                                                                                            ecx = var_f8;
                                                                                            var_50 = ecx;
                                                                                            var_48 = eax;
                                                                                            var_40 = ebx;
                                                                                            var_38 = r12d;
                                                                                            var_30 = r12d;
                                                                                            r12 = var_d0;
                                                                                            var_28 = rdi;
                                                                                            var_20 = eax;
                                                                                            rax = qword_ADDR;
                                                                                            sub_140028010(r12, var_310, var_314);
                                                                                        } else {
                                                                                            var_40 = 0xADDR;
                                                                                            var_38 = r12d;
                                                                                            var_30 = r12d;
                                                                                            r12 = var_d0;
                                                                                            var_28 = rdi;
                                                                                            var_20 = eax;
                                                                                            rax = qword_ADDR;
                                                                                            sub_140028010(r12, var_84c, var_850);
                                                                                        }
                                                                                        loc_ADDR:
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010(r12, var_c0);
                                                                                        rcx = var_100;
                                                                                        r12d = 0;
                                                                                        if (rcx != 0) {
                                                                                            goto loc_ADDR;
                                                                                        }
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                GetLastError();
                                                                                goto loc_ADDR;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        if (rdi != 0) {
                                                            rax = qword_ADDR;
                                                            rcx = rdi;
                                                            sub_140028010(rdi);
                                                        }
                                                    } else {
                                                        GetLastError();
                                                        rdi = var_110;
                                                        r12d = 0;
                                                        rax = qword_ADDR;
                                                        rcx = rdi;
                                                        sub_140028010(rdi);
                                                    }
                                                } else {
                                                    GetLastError();
                                                }
                                            } else {
                                                eax = var_814;
                                                rdi = var_818;
                                                ebx = 0xADDR;
                                                var_f8 = eax;
                                                r8d = ebx;
                                                rax = var_830;
                                                edx = 0;
                                                var_68 = rax;
                                                var_90 = rdi;
                                                var_570 = r12d;
                                                sub_1400029dc(&var_574);
                                                r8d = ebx;
                                                var_500 = r12d;
                                                edx = 0;
                                                sub_1400029dc(&var_504);
                                                eax = 0;
                                                var_88 = r12;
                                                var_78 = r12d;
                                                xmm0 = 0;
                                                var_108 = r12d;
                                                var_9c0 = rax;
                                                var_9c8 = eax;
                                                var_9a0 = xmm0;
                                                var_9b0 = xmm0;
                                                var_8b0 = xmm0;
                                                if (rdi != 0) {
                                                    rdi = var_d0;
                                                    rax = qword_ADDR;
                                                    sub_140028010(rdi, (rbx - 0xADDR));
                                                    if (rax != 0) {
                                                        rcx = rax;
                                                        rax = qword_ADDR;
                                                        sub_140028010(rax, 0xADDR, &var_570);
                                                        if (eax != 0) {
                                                            var_88 = var_588;
                                                            var_78 = var_574;
                                                            var_108 = var_578;
                                                        }
                                                        rax = qword_ADDR;
                                                        rcx = rdi;
                                                        sub_140028010(rdi);
                                                        var_80 = rax;
                                                        rbx = rax;
                                                        if (rax == 0) {
                                                            GetLastError();
                                                        } else {
                                                            eax = var_854;
                                                            eax -= var_84c;
                                                            var_9a4 = eax;
                                                            xmm0 = 0;
                                                            eax = var_850;
                                                            r8d = 0;
                                                            eax -= var_858;
                                                            var_9a8 = eax;
                                                            rax = qword_ADDR;
                                                            var_28 = r12d;
                                                            var_298 = r12;
                                                            var_9b0 = xmm0;
                                                            var_9c0 = r12;
                                                            var_9c8 = r12d;
                                                            var_9a0 = 0xADDR;
                                                            var_9ac = 0xADDR;
                                                            var_20 = r12;
                                                            sub_140028010(rbx, &var_9a0, 0, &var_298, r12);
                                                            var_b8 = rax;
                                                            rdi = rax;
                                                            r12d = 0xADDR;
                                                            if (rax != 0) {
                                                                rax = qword_ADDR;
                                                                sub_140028010(rdi, 0xADDR, &var_500);
                                                                edx = 0;
                                                                ecx = edx;
                                                                eax = var_854;
                                                                rdx = rdi;
                                                                rcx = ((eax != 0) ? var_518 : rcx);
                                                                eax -= var_84c;
                                                                var_8b8 = eax;
                                                                eax = var_858;
                                                                eax -= var_850;
                                                                var_8bc = eax;
                                                                rax = qword_ADDR;
                                                                var_a0 = rcx;
                                                                rcx = rbx;
                                                                sub_140028010(rbx, rdi);
                                                                rax = qword_ADDR;
                                                                sub_140028010(rbx, 1);
                                                                rax = qword_ADDR;
                                                                sub_140028010(rbx, var_68);
                                                                rdi = qword_ADDR;
                                                                ecx = 0;
                                                                var_68 = rax;
                                                                rax = qword_ADDR;
                                                                sub_140028010();
                                                                r8 = rax;
                                                                rax = rdi;
                                                                sub_140028010(rbx, &var_8b0);
                                                                eax = var_148;
                                                                edi = 0;
                                                                var_28 = rdi;
                                                                r8d = -1;
                                                                var_20 = eax;
                                                                rax = qword_ADDR;
                                                                sub_140028010(rbx, var_90, r8, &var_8b0, var_148, 0);
                                                                eax = (rdi + 1);
                                                                if (esi != eax) {
                                                                    var_f8 = 0xADDR;
                                                                } else if (var_f8 == edi) {
                                                                    var_f8 = 0xADDR;
                                                                } else {
                                                                    rax = qword_ADDR;
                                                                    sub_140028010((rdi + 8));
                                                                    var_f8 = eax;
                                                                }
                                                                eax = var_84c;
                                                                r9d = edi;
                                                                ecx = var_850;
                                                                edx = eax;
                                                                r11d = var_8bc;
                                                                edx = -(edx);
                                                                r8d = ecx;
                                                                r10d = edi;
                                                                r9d = ((eax >= 0 /* signed */) ? eax : r9d);
                                                                edx = ((eax >= 0 /* signed */) ? edi : edx);
                                                                eax = var_78;
                                                                r8d = -(r8d);
                                                                r10d = ((ecx >= 0 /* signed */) ? ecx : r10d);
                                                                r8d = ((ecx >= 0 /* signed */) ? edi : r8d);
                                                                ecx = var_8b8;
                                                                eax -= r9d;
                                                                ecx -= edx;
                                                                ecx = ((ecx >= eax /* signed */) ? eax : ecx);
                                                                eax = var_108;
                                                                eax -= r10d;
                                                                var_a8 = ecx;
                                                                r11d -= r8d;
                                                                r11d = ((r11d >= eax /* signed */) ? eax : r11d);
                                                                eax = 0;
                                                                if (ecx > 0 /* signed */ && r11d > 0 /* signed */) {
                                                                    rdi = (int64)(var_78);
                                                                    eax = var_8b8;
                                                                    r14d = var_a8;
                                                                    r15d = var_f8;
                                                                    eax *= r8d;
                                                                    r8d = 0;
                                                                    var_78 = r8d;
                                                                    esi = r8d;
                                                                    rcx = (int64)(eax);
                                                                    rax = (int64)(edx);
                                                                    rcx += rax;
                                                                    rax = var_a0;
                                                                    rdx = (rax + (rcx * 4));
                                                                    eax = edi;
                                                                    eax *= r10d;
                                                                    var_90 = rdx;
                                                                    rcx = (int64)(eax);
                                                                    rax = (int64)(r9d);
                                                                    rcx += rax;
                                                                    rax = var_88;
                                                                    rax += (rcx * 4);
                                                                    rcx = ((int64)(var_8b8) << 2);
                                                                    var_a0 = rcx;
                                                                    rcx = (rdi << 2);
                                                                    var_e0 = rcx;
                                                                    var_88 = rax;
                                                                    r10 = rax;
                                                                    var_108 = r8d;
                                                                    r10 -= rdx;
                                                                    r9 = (rdx + 2);
                                                                    r13d = 1;
                                                                }
                                                                rax = qword_ADDR;
                                                                sub_140028010(var_b8);
                                                                rax = var_68;
                                                                if (rax != 0) {
                                                                    rdx = rax;
                                                                    rcx = rbx;
                                                                    rax = qword_ADDR;
                                                                    sub_140028010();
                                                                }
                                                            } else {
                                                                GetLastError();
                                                                edi = eax;
                                                                if (eax > 0 /* signed */) {
                                                                    edi = (uint32)(di);
                                                                    edi |= 0xADDR;
                                                                }
                                                                eax = 0xADDR;
                                                                __asm("cmovns edi, eax");
                                                            }
                                                            rax = qword_ADDR;
                                                            rcx = rbx;
                                                            sub_140028010(rbx);
                                                            ebx = 0;
                                                            if (edi < 0 /* signed */) {
                                                                r12d = 0;
                                                            } else {
                                                                eax = var_814;
                                                                rdi = var_820;
                                                                var_f8 = eax;
                                                                edx = 0;
                                                                rax = var_838;
                                                                var_68 = rax;
                                                                var_90 = rdi;
                                                                var_420 = ebx;
                                                                sub_1400029dc(&var_424, 0, (rbx + 0xADDR));
                                                                edx = 0;
                                                                var_650 = ebx;
                                                                sub_1400029dc(&var_654, 0, (rbx + 0xADDR));
                                                                eax = 0;
                                                                var_88 = rbx;
                                                                var_f0 = ebx;
                                                                xmm0 = 0;
                                                                var_78 = ebx;
                                                                var_9f0 = rax;
                                                                var_9f8 = eax;
                                                                var_9d0 = xmm0;
                                                                var_9e0 = xmm0;
                                                                var_8c0 = xmm0;
                                                                if (rdi != 0) {
                                                                    rdi = var_d0;
                                                                    rax = qword_ADDR;
                                                                    sub_140028010(rdi, (rbx + 7));
                                                                    if (rax != 0) {
                                                                        rcx = rax;
                                                                        rax = qword_ADDR;
                                                                        sub_140028010(rax, 0xADDR, &var_420);
                                                                        if (eax != 0) {
                                                                            var_88 = var_438;
                                                                            var_f0 = var_424;
                                                                            var_78 = var_428;
                                                                        }
                                                                        rax = qword_ADDR;
                                                                        rcx = rdi;
                                                                        sub_140028010(rdi);
                                                                        ecx = 0;
                                                                        var_80 = rax;
                                                                        rbx = rax;
                                                                        if (rax == 0) {
                                                                            GetLastError();
                                                                        } else {
                                                                            eax = var_864;
                                                                            eax -= var_85c;
                                                                            var_9d4 = eax;
                                                                            xmm0 = 0;
                                                                            eax = var_860;
                                                                            r8d = 0;
                                                                            eax -= var_868;
                                                                            var_28 = ecx;
                                                                            var_9d8 = eax;
                                                                            rax = qword_ADDR;
                                                                            var_2a0 = rcx;
                                                                            var_9f0 = rcx;
                                                                            var_9f8 = ecx;
                                                                            var_20 = rcx;
                                                                            var_9e0 = xmm0;
                                                                            var_9d0 = 0xADDR;
                                                                            var_9dc = 0xADDR;
                                                                            sub_140028010(rbx, &var_9d0, 0, &var_2a0);
                                                                            var_b8 = rax;
                                                                            rdi = rax;
                                                                            if (rax != 0) {
                                                                                rax = qword_ADDR;
                                                                                sub_140028010(rdi, 0xADDR, &var_650);
                                                                                edx = 0;
                                                                                ecx = edx;
                                                                                eax = var_864;
                                                                                rdx = rdi;
                                                                                rcx = ((eax != 0) ? var_668 : rcx);
                                                                                eax -= var_85c;
                                                                                var_8c8 = eax;
                                                                                eax = var_868;
                                                                                eax -= var_860;
                                                                                var_8cc = eax;
                                                                                rax = qword_ADDR;
                                                                                var_a0 = rcx;
                                                                                rcx = rbx;
                                                                                sub_140028010(rbx, rdi);
                                                                                rax = qword_ADDR;
                                                                                sub_140028010(rbx, 1);
                                                                                rax = qword_ADDR;
                                                                                sub_140028010(rbx, var_68);
                                                                                rdi = qword_ADDR;
                                                                                ecx = 0;
                                                                                var_68 = rax;
                                                                                rax = qword_ADDR;
                                                                                sub_140028010();
                                                                                r8 = rax;
                                                                                rax = rdi;
                                                                                sub_140028010(rbx, &var_8c0);
                                                                                eax = var_148;
                                                                                edi = 0;
                                                                                var_28 = rdi;
                                                                                r8d = -1;
                                                                                var_20 = eax;
                                                                                rax = qword_ADDR;
                                                                                sub_140028010(rbx, var_90, r8, &var_8c0, var_148, 0);
                                                                                eax = (rdi + 1);
                                                                                if (esi != eax) {
                                                                                    eax = 0xADDR;
                                                                                    while (1) {
                                                                                        var_148 = eax;
                                                                                        var_120 = eax;
                                                                                    } else if (var_f8 == edi) {
                                                                                        var_148 = 0xADDR;
                                                                                    } else {
                                                                                        rax = qword_ADDR;
                                                                                        sub_140028010((rdi + 8));
                                                                                    }
                                                                                }
                                                                                eax = var_85c;
                                                                                r9d = edi;
                                                                                ecx = var_860;
                                                                                edx = eax;
                                                                                esi = var_8c8;
                                                                                edx = -(edx);
                                                                                r11d = var_8cc;
                                                                                r8d = ecx;
                                                                                r10d = edi;
                                                                                r9d = ((eax >= 0 /* signed */) ? eax : r9d);
                                                                                edx = ((eax >= 0 /* signed */) ? edi : edx);
                                                                                eax = var_f0;
                                                                                r8d = -(r8d);
                                                                                r8d = ((ecx >= 0 /* signed */) ? edi : r8d);
                                                                                r10d = ((ecx >= 0 /* signed */) ? ecx : r10d);
                                                                                eax -= r9d;
                                                                                esi -= edx;
                                                                                esi = ((esi >= eax /* signed */) ? eax : esi);
                                                                                eax = var_78;
                                                                                eax -= r10d;
                                                                                r11d -= r8d;
                                                                                r11d = ((r11d >= eax /* signed */) ? eax : r11d);
                                                                                eax = 0;
                                                                                if (esi > 0 /* signed */ && r11d > 0 /* signed */) {
                                                                                    eax = var_8c8;
                                                                                    ebx = 0;
                                                                                    rdi = (int64)(var_f0);
                                                                                    eax *= r8d;
                                                                                    var_f8 = ebx;
                                                                                    rcx = (int64)(eax);
                                                                                    rax = (int64)(edx);
                                                                                    rcx += rax;
                                                                                    rax = var_a0;
                                                                                    rdx = (rax + (rcx * 4));
                                                                                    eax = edi;
                                                                                    eax *= r10d;
                                                                                    var_90 = rdx;
                                                                                    rcx = (int64)(eax);
                                                                                    rax = (int64)(r9d);
                                                                                    rcx += rax;
                                                                                    rax = var_88;
                                                                                    rax += (rcx * 4);
                                                                                    rcx = ((int64)(var_8c8) << 2);
                                                                                    var_a0 = rcx;
                                                                                    rcx = (rdi << 2);
                                                                                    var_e0 = rcx;
                                                                                    var_88 = rax;
                                                                                    do {
                                                                                        r15d = var_120;
                                                                                        r9 = (rdx + 2);
                                                                                        r10 = rax;
                                                                                        var_f0 = ebx;
                                                                                        r10 -= rdx;
                                                                                        r8d = ebx;
                                                                                        r14d = 1;
                                                                                        do {
                                                                                            ecx = (uint32)(*(u8*)((r9 - 1)));
                                                                                            edx = (rax + (rcx * 2));
                                                                                            eax = (uint32)(*(u8*)(r9));
                                                                                            edx += eax;
                                                                                            edx >>= 2;
                                                                                            dl = ~(dl);
                                                                                            if (dl != 0) {
                                                                                                edi = (uint32)(*(u8*)((r9 + r10)));
                                                                                                eax = r12d;
                                                                                                r8d = (uint32)(dl);
                                                                                                ecx = (uint32)(r15b);
                                                                                                ecx -= edi;
                                                                                                ecx *= r8d;
                                                                                                __asm("imul ecx");
                                                                                                edx += ecx;
                                                                                                ecx = (uint32)(var_149);
                                                                                                edx >>= 7;
                                                                                                eax = (edx >> 0xADDR);
                                                                                                edx += eax;
                                                                                                eax = r12d;
                                                                                                dl += dil;
                                                                                                *(u8*)((r9 + r10)) = dl;
                                                                                                edi = (uint32)(*(u8*)(((r10 + r9) - 1)));
                                                                                                ecx -= edi;
                                                                                                ecx *= r8d;
                                                                                                __asm("imul ecx");
                                                                                                edx += ecx;
                                                                                                ecx = (uint32)(var_14a);
                                                                                                edx >>= 7;
                                                                                                eax = (edx >> 0xADDR);
                                                                                                edx += eax;
                                                                                                eax = r12d;
                                                                                                dl += dil;
                                                                                                *(u8*)(((r10 + r9) - 1)) = dl;
                                                                                                edi = (uint32)(*(u8*)(((r10 + r9) - 2)));
                                                                                                ecx -= edi;
                                                                                                ecx *= r8d;
                                                                                                __asm("imul ecx");
                                                                                                edx += ecx;
                                                                                                ecx = 0xADDR;
                                                                                                edx >>= 7;
                                                                                                eax = (edx >> 0xADDR);
                                                                                                edx += eax;
                                                                                                eax = r12d;
                                                                                                dl += dil;
                                                                                                *(u8*)(((r10 + r9) - 2)) = dl;
                                                                                                edi = (uint32)(*(u8*)(((r10 + r9) + 1)));
                                                                                                ecx -= edi;
                                                                                                ecx *= r8d;
                                                                                                r8d = var_f0;
                                                                                                __asm("imul ecx");
                                                                                                edx += ecx;
                                                                                                edx >>= 7;
                                                                                                eax = (edx >> 0xADDR);
                                                                                                edx += eax;
                                                                                                dl += dil;
                                                                                                *(u8*)(((r10 + r9) + 1)) = dl;
                                                                                            }
                                                                                            r8d += r14d;
                                                                                            r9 += 4;
                                                                                            var_f0 = r8d;
                                                                                            edi = ebx;
                                                                                        } while (r8d < esi /* signed */);
                                                                                        rdx = var_90;
                                                                                        rax = var_88;
                                                                                        ecx = var_f8;
                                                                                        rdx += var_a0;
                                                                                        ecx += 1;
                                                                                        rax += var_e0;
                                                                                        var_90 = rdx;
                                                                                        var_88 = rax;
                                                                                        var_f8 = ecx;
                                                                                    } while (ecx < r11d /* signed */);
                                                                                    r13d = var_d8;
                                                                                    r14 = 0xADDR;
                                                                                    rbx = var_80;
                                                                                }
                                                                                rax = qword_ADDR;
                                                                                sub_140028010(var_b8);
                                                                                rax = var_68;
                                                                                r12d = 0;
                                                                                if (rax != 0) {
                                                                                    rdx = rax;
                                                                                    rcx = rbx;
                                                                                    rax = qword_ADDR;
                                                                                    sub_140028010();
                                                                                }
                                                                            } else {
                                                                                GetLastError();
                                                                                r12d = 0;
                                                                                edi = eax;
                                                                                if (eax > 0 /* signed */) {
                                                                                    edi = (uint32)(ax);
                                                                                    edi |= 0xADDR;
                                                                                }
                                                                                eax = 0xADDR;
                                                                                edi = ((edi >= 0 /* signed */) ? eax : edi);
                                                                            }
                                                                            rax = qword_ADDR;
                                                                            rcx = rbx;
                                                                            sub_140028010(rbx);
                                                                            if (edi >= 0 /* signed */) {
                                                                                rdi = var_110;
                                                                                r12 = var_d0;
                                                                                goto loc_ADDR;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            rax = var_98;
                                            if (rax != 0) {
                                                rcx = rax;
                                                rax = qword_ADDR;
                                                sub_140028010();
                                            }
                                        }
                                        rax = var_818;
                                        rbx = -1;
                                        if (rax != 0) {
                                            rcx = rbx;
                                            do {
                                                rcx += 1;
                                            } while (*(u16*)((rax + (rcx * 2))) != r12w);
                                            rdi = var_818;
                                            rcx = ((rcx * 2) + 2);
                                            eax = 0;
                                            __asm("rep stosb [rdi]");
                                            sub_140024e2c(var_818);
                                            var_818 = r12;
                                        }
                                        rax = var_820;
                                        if (rax != 0) {
                                            rcx = rbx;
                                            do {
                                                rcx += 1;
                                            } while (*(u16*)((rax + (rcx * 2))) != r12w);
                                            rdi = var_820;
                                            rcx = ((rcx * 2) + 2);
                                            eax = 0;
                                            __asm("rep stosb [rdi]");
                                            sub_140024e2c(var_820);
                                            var_820 = r12;
                                        }
                                        rdi = var_828;
                                        if (rdi != 0) {
                                            rcx = rbx;
                                            do {
                                                rcx += 1;
                                            } while (*(u16*)((rdi + (rcx * 2))) != r12w);
                                            rcx = ((rcx * 2) + 2);
                                            eax = 0;
                                            __asm("rep stosb [rdi]");
                                            sub_140024e2c(var_828);
                                            var_828 = r12;
                                        }
                                        rcx = var_830;
                                        if (rcx != 0) {
                                            rax = qword_ADDR;
                                            sub_140028010();
                                            var_830 = r12;
                                        }
                                        rcx = var_838;
                                        if (rcx != 0) {
                                            rax = qword_ADDR;
                                            sub_140028010();
                                            var_838 = r12;
                                        }
                                        rcx = var_840;
                                        if (rcx != 0) {
                                            rax = qword_ADDR;
                                            sub_140028010();
                                            var_840 = r12;
                                        }
                                        edi = 1;
                                        do {
                                            eax = 0;
                                            __asm("lock cmpxchg [0xADDR], edi");
                                        } while (dword_ADDR != eax);
                                        eax = dword_ADDR;
                                        if (eax > 0 /* signed */) {
                                            eax -= edi;
                                            dword_ADDR = eax;
                                            if (eax == 0) {
                                                ebx = 4;
                                                rsi = off_1400360c0;
                                                rdi = rsi;
                                                eax = (rbx - 3);
                                                do {
                                                    rcx = *(u64*)(rdi);
                                                    if (rcx != 0) {
                                                        FreeLibrary(*(u64*)(rdi));
                                                        eax = 1;
                                                    }
                                                    rdi += 0xADDR;
                                                    rbx -= rax;
                                                } while (rbx != 0);
                                                edx = 0;
                                                sub_1400029dc(rsi, 0, (rbx + 0xADDR));
                                                edx = (rbx + 2);
                                                rax = off_140034000;
                                                rcx = off_140029260;
                                                edi = (rdx + 0xADDR);
                                                do {
                                                    xmm0 = *(__m128*)(rcx);
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)(rax) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    rcx += rdi;
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    rax += rdi;
                                                    *(__m128*)((rax - 0xADDR)) = xmm1;
                                                    rdx -= 1;
                                                } while (rdx != 0);
                                                xmm0 = *(__m128*)(rcx);
                                                edi = (rdx + 1);
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)(rax) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                            }
                                        }
                                        eax = r12d;
                                        __asm("xchg [0xADDR], eax");
                                        eax = dword_ADDR;
                                        var_80 = eax;
                                        rax = qword_ADDR;
                                        sub_140028010();
                                        rsi = rax;
                                        var_cc0 = r12d;
                                        esi -= dword_ADDR;
                                        xmm0 = 0;
                                        eax = 0;
                                        var_70 = rsi;
                                        var_cf8 = rax;
                                        var_cc8 = xmm0;
                                        var_cd8 = xmm0;
                                        var_ce8 = xmm0;
                                        do {
                                            eax = 0;
                                            __asm("lock cmpxchg [0xADDR], edi");
                                        } while (dword_ADDR != eax);
                                        eax = dword_ADDR;
                                        if (eax != 0) {
                                            eax += edi;
                                            dword_ADDR = eax;
                                        } else {
                                            edi = 0;
                                            var_b8 = r12d;
                                            var_e0 = edi;
                                            var_a0 = r12d;
                                            sub_140024bf8(0xADDR);
                                            rbx = rax;
                                            if (rax == 0) {
                                                ebx = 4;
                                                rdi = off_1400360c0;
                                                r12d = 0;
                                                eax = (rbx - 3);
                                                do {
                                                    rcx = *(u64*)(rdi);
                                                    if (rcx != 0) {
                                                        FreeLibrary(*(u64*)(rdi));
                                                        eax = 1;
                                                    }
                                                    rdi += 0xADDR;
                                                    rbx -= rax;
                                                } while (rbx != 0);
                                                edx = 0;
                                                sub_1400029dc(off_1400360c0, 0, (rbx + 0xADDR));
                                                edx = (rbx + 2);
                                                rax = off_140034000;
                                                rcx = off_140029260;
                                                edi = (rdx + 0xADDR);
                                                do {
                                                    xmm0 = *(__m128*)(rcx);
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)(rax) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    rcx += rdi;
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    rax += rdi;
                                                    *(__m128*)((rax - 0xADDR)) = xmm1;
                                                    rdx -= 1;
                                                } while (rdx != 0);
                                                xmm0 = *(__m128*)(rcx);
                                                edi = (rdx + 1);
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)(rax) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                            } else {
                                                r14d = edi;
                                                r15 = (rax + 7);
                                                r13d = -1;
                                                r12 = off_14002da82;
                                                esi = edi;
                                                ecx = (rdi + 0xADDR);
                                                r11d = edi;
                                                do {
                                                    eax = (uint32)(*(u8*)((r12 - 1)));
                                                    r10d = (uint32)(*(u8*)((r12 - 2)));
                                                    r9d = (uint32)(*(u8*)((r12 + 2)));
                                                    r10d <<= 8;
                                                    r10d |= eax;
                                                    r9d <<= 8;
                                                    eax = (uint32)(*(u8*)(r12));
                                                    r12 += 8;
                                                    r10d <<= 8;
                                                    r10d |= eax;
                                                    eax = (uint32)(*(u8*)((r12 - 7)));
                                                    r10d <<= 8;
                                                    r10d |= eax;
                                                    eax = (uint32)(*(u8*)((r12 - 5)));
                                                    r9d |= eax;
                                                    r11d ^= r10d;
                                                    eax = (uint32)(*(u8*)((r12 - 4)));
                                                    r9d <<= 8;
                                                    r9d |= eax;
                                                    eax = (uint32)(*(u8*)((r12 - 3)));
                                                    r9d <<= 8;
                                                    r9d |= eax;
                                                    esi ^= r9d;
                                                    esi ^= r11d;
                                                    esi ^= 0xADDR;
                                                    r8d = esi;
                                                    eax = (rsi + 0xADDR);
                                                    esi = ror(esi, 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    edx += esi;
                                                    r11d ^= edx;
                                                    eax = (r11 + 0xADDR);
                                                    eax = ror(eax, 9);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    r8d ^= edx;
                                                    edx = (r8d * 0xADDR);
                                                    eax = (r8d >> 0xADDR);
                                                    edx -= eax;
                                                    edx -= 0xADDR;
                                                    r11d ^= edx;
                                                    eax = (r11d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    eax = 0xADDR;
                                                    r8d ^= edx;
                                                    edx = (r8d ^ 0xADDR);
                                                    eax -= edx;
                                                    r11d ^= eax;
                                                    eax = r11d;
                                                    edi = r11d;
                                                    eax ^= 0xADDR;
                                                    edi = ror(edi, 6);
                                                    edx = (eax * 0xADDR);
                                                    edi ^= edx;
                                                    r8d ^= edi;
                                                    edi = 0xADDR;
                                                    eax = (r8 + 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r8d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    r11d ^= edx;
                                                    eax = (r11 + 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    r8d ^= edx;
                                                    eax = r8d;
                                                    eax ^= edi;
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r8d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx ^= eax;
                                                    r11d ^= edx;
                                                    eax = (r11d ^ 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = (r11d >> 0xADDR);
                                                    edx ^= eax;
                                                    r8d ^= edx;
                                                    eax = r8d;
                                                    eax = ~(eax);
                                                    eax = ror(eax, 5);
                                                    eax += 0xADDR;
                                                    eax *= 0xADDR;
                                                    r11d ^= eax;
                                                    esi = (r11 - 0xADDR);
                                                    esi ^= r8d;
                                                    esi ^= edi;
                                                    eax = (esi ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = (esi >> 2);
                                                    edx += eax;
                                                    r11d ^= edx;
                                                    eax = (r11 - 0xADDR);
                                                    eax = ror(eax, 6);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    esi ^= edx;
                                                    eax = esi;
                                                    edi = esi;
                                                    eax ^= 0xADDR;
                                                    edi = ror(edi, 9);
                                                    edx = (eax * 0xADDR);
                                                    edi += edx;
                                                    r11d ^= edi;
                                                    eax = (r11d ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = r11d;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    esi ^= edx;
                                                    edi = esi;
                                                    edi ^= r11d;
                                                    edi ^= 0xADDR;
                                                    eax = edi;
                                                    eax = ror(eax, 3);
                                                    edx = ((eax * 0xADDR) - 0xADDR);
                                                    esi ^= edx;
                                                    eax = (rsi - 0xADDR);
                                                    eax = ror(eax, 1);
                                                    edx = (eax * 0xADDR);
                                                    eax = esi;
                                                    eax = ror(eax, 6);
                                                    edx -= eax;
                                                    edi ^= edx;
                                                    eax = (rdi - 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = edi;
                                                    eax = ror(eax, 0xADDR);
                                                    edx += eax;
                                                    esi ^= edx;
                                                    eax = (rsi - 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = esi;
                                                    eax = ror(eax, 0xADDR);
                                                    edx -= eax;
                                                    edi ^= edx;
                                                    eax = (edi ^ 0xADDR);
                                                    edx = (eax * 0xADDR);
                                                    eax = (edi >> 3);
                                                    edx ^= eax;
                                                    esi ^= edx;
                                                    eax = (esi ^ 0xADDR);
                                                    eax = ror(eax, 0xADDR);
                                                    r11d = (eax * 0xADDR);
                                                    eax = esi;
                                                    eax = ror(eax, 0xADDR);
                                                    esi ^= r13d;
                                                    r13d = r9d;
                                                    r11d ^= eax;
                                                    r11d ^= edi;
                                                    edi = esi;
                                                    r11d ^= r14d;
                                                    edi <<= 0xADDR;
                                                    *(u8*)((r15 - 4)) = r11b;
                                                    edx = r11d;
                                                    *(u8*)(r15) = sil;
                                                    eax = (r11d >> 8);
                                                    r15 += 8;
                                                    edx <<= 0xADDR;
                                                    r14d = r10d;
                                                    edx |= eax;
                                                    eax = esi;
                                                    *(u8*)((r15 - 0xADDR)) = dl;
                                                    r8d = edx;
                                                    eax >>= 8;
                                                    edi |= eax;
                                                    r8d <<= 0xADDR;
                                                    *(u8*)((r15 - 9)) = dil;
                                                    eax = (edi << 0xADDR);
                                                    edi >>= 8;
                                                    eax |= edi;
                                                    edx >>= 8;
                                                    r8d |= edx;
                                                    edi = 1;
                                                    *(u8*)((r15 - 0xADDR)) = r8b;
                                                    *(u8*)((r15 - 0xADDR)) = al;
                                                    r8d = ror(r8d, 8);
                                                    eax = ror(eax, 8);
                                                    *(u8*)((r15 - 0xADDR)) = r8b;
                                                    *(u8*)((r15 - 0xADDR)) = al;
                                                    rcx -= 1;
                                                } while (rcx != 0);
                                                r13d = var_d8;
                                                edx = 0;
                                                r12d = var_a0;
                                                eax = edx;
                                                cl = dl;
                                                r14 = 0xADDR;
                                                r8d = 0xADDR;
                                                do {
                                                    cl ^= *(u8*)((rax + rbx));
                                                    rax += rdi;
                                                } while (rax < r8);
                                                eax = (uint32)(cl);
                                                if (rax == *(u64*)("@")) {
                                                    *(u8*)(((rbx + r8) - 1)) = dl;
                                                    sub_1400029dc(off_1400360c0, rdx, 0xADDR);
                                                    r8d = 0;
                                                    rsi = rbx;
                                                    if (*(u8*)(rbx) == 0xADDR) {
                                                        GetProcessHeap();
                                                        HeapFree(rax, 0, rsi);
                                                    } else {
                                                        do {
                                                            rdx = rbx;
                                                            rax = -1;
                                                            do {
                                                                rax += 1;
                                                            } while (*(u16*)((rbx + (rax * 2))) != r8w);
                                                            rbx += (rax * 2);
                                                            eax = var_e0;
                                                            rcx = (rax + (rax * 2));
                                                            rax = off_1400360c0;
                                                            rdi = (rax + (rcx * 8));
                                                            GetModuleHandleExW(0, rdx, rdi);
                                                            r8d = 0;
                                                            if (eax != 0) {
                                                                rax = *(u64*)(rdi);
                                                                r12d = r8d;
                                                                if (*(u16*)(rax) == 0xADDR) {
                                                                    rcx = (int64)(*(u32*)((rax + 0xADDR)));
                                                                    if (ecx >= 0xADDR) {
                                                                        r12d = 0xADDR;
                                                                    } else {
                                                                        rcx += *(u64*)(rdi);
                                                                        if (rcx >= *(u64*)(rdi) && *(u32*)(rcx) == 0xADDR) {
                                                                            eax = (uint32)(*(u16*)((rcx + 0xADDR)));
                                                                            ax -= dx;
                                                                            if ((0xADDR & ax) == 0) {
                                                                                *(u64*)((rdi + 0xADDR)) = *(u64*)((rcx + 0xADDR));
                                                                                *(u32*)((rdi + 8)) = *(u32*)((rcx + 0xADDR));
                                                                            } else {
                                                                                r12d = 0xADDR;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                eax = *(u32*)((rbx + 2));
                                                                edi = r8d;
                                                                rbx += 6;
                                                                var_a0 = eax;
                                                                if (eax != 0) {
                                                                    eax = var_e0;
                                                                    rcx = (rax + (rax * 2));
                                                                    var_b0 = rcx;
                                                                    do {
                                                                        rdx = rbx;
                                                                        rax = -1;
                                                                        do {
                                                                            rax += 1;
                                                                        } while (*(u8*)((rbx + rax)) != r8b);
                                                                        rbx += 1;
                                                                        rbx += rax;
                                                                        if (r12d >= 0 /* signed */) {
                                                                            rax = off_1400360c0;
                                                                            rcx = *(u64*)((rax + (rcx * 8)));
                                                                            GetProcAddress(*(u64*)((rax + (rcx * 8))), rdx);
                                                                            r8d = 0;
                                                                            if (rax != 0) {
                                                                                ecx = var_b8;
                                                                                rdx = off_140034000;
                                                                                *(u64*)((rdx + (rcx * 8))) = rax;
                                                                                rcx = var_b0;
                                                                            }
                                                                        }
                                                                        var_b8 += 1;
                                                                        edi += 1;
                                                                    } while (edi < var_a0);
                                                                }
                                                            }
                                                            edi = 1;
                                                            var_e0 += 1;
                                                        } while (*(u8*)(rbx) != r8b);
                                                    }
                                                    if (r12d < 0 /* signed */) {
                                                        rsi = var_70;
                                                    } else {
                                                        eax = dword_ADDR;
                                                        r12d = 0;
                                                        rsi = var_70;
                                                    }
                                                } else {
                                                    sub_140024e2c(rbx);
                                                }
                                            }
                                        }
                                        eax = r12d;
                                        r9 = &var_cc0;
                                        __asm("xchg [0xADDR], eax");
                                        eax = var_80;
                                        ebx = 4;
                                        var_cf8 = eax;
                                        rax = qword_ADDR;
                                        ecx = 0;
                                        var_cc8 = esi;
                                        var_cc0 = edi;
                                        var_cd8 = 0xADDR;
                                        var_cd0 = edi;
                                        var_ce8 = edi;
                                        var_ce0 = edi;
                                        var_cf0 = edi;
                                        sub_140028010(0, 0xADDR, ebx, &var_cc0);
                                        do {
                                            eax = 0;
                                            __asm("lock cmpxchg [0xADDR], edi");
                                        } while (dword_ADDR != eax);
                                        eax = dword_ADDR;
                                        if (eax > 0 /* signed */) {
                                            eax -= edi;
                                            dword_ADDR = eax;
                                            if (eax == 0) {
                                                rsi = off_1400360c0;
                                                eax = 1;
                                                rdi = rsi;
                                                do {
                                                    rcx = *(u64*)(rdi);
                                                    if (rcx != 0) {
                                                        FreeLibrary(*(u64*)(rdi));
                                                        eax = 1;
                                                    }
                                                    rdi += 0xADDR;
                                                    rbx -= rax;
                                                } while (rbx != 0);
                                                edx = 0;
                                                sub_1400029dc(rsi, 0, (rbx + 0xADDR));
                                                edx = (rbx + 2);
                                                rax = off_140034000;
                                                rcx = off_140029260;
                                                edi = (rdx + 0xADDR);
                                                do {
                                                    xmm0 = *(__m128*)(rcx);
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)(rax) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    xmm0 = *(__m128*)((rcx + 0xADDR));
                                                    *(__m128*)((rax + 0xADDR)) = xmm1;
                                                    xmm1 = *(__m128*)((rcx + 0xADDR));
                                                    rcx += rdi;
                                                    *(__m128*)((rax + 0xADDR)) = xmm0;
                                                    rax += rdi;
                                                    *(__m128*)((rax - 0xADDR)) = xmm1;
                                                    rdx -= 1;
                                                } while (rdx != 0);
                                                xmm0 = *(__m128*)(rcx);
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)(rax) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                xmm1 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                                xmm0 = *(__m128*)((rcx + 0xADDR));
                                                *(__m128*)((rax + 0xADDR)) = xmm1;
                                                *(__m128*)((rax + 0xADDR)) = xmm0;
                                            }
                                        }
                                        rdx = var_d0;
                                        eax = r12d;
                                        __asm("xchg [0xADDR], eax");
                                        rax = qword_ADDR;
                                        ecx = 0;
                                        sub_140028010(0, var_d0);
                                        var_148 = var_138;
                                    } else {
                                        GetLastError();
                                    }
                                } else {
                                    GetLastError();
                                }
                            }
                            sub_140024cd8(&var_220);
                            xmm0 = 0;
                            xmm1 = 0;
                            var_340 = xmm0;
                            rsi = r12;
                            var_350 = xmm1;
                            GetProcessHeap();
                            edi = 8;
                            HeapAlloc(rax, edi, 0xADDR);
                            rbx = rax;
                            if (rax == 0) {
                                eax = r12d;
                                rbx = r12;
                                var_80 = eax;
                                var_b8 = eax;
                                eax = 0xADDR;
                                var_70 = rbx;
                                if (eax >= 0 /* signed */) {
                                    loc_ADDR:
                                    GetProcessHeap();
                                    HeapAlloc(rax, edi, rdi);
                                    rdi = rax;
                                    if (rax == 0) {
                                        rsi = r12;
                                        r12d = 0;
                                    } else {
                                        rcx = qword_ADDR;
                                        r12d = 8;
                                        *(u64*)(rax) = rcx;
                                        __asm("rdtsc");
                                        rdx <<= 0xADDR;
                                        rax |= rdx;
                                        var_b0 = rax;
                                        rax = var_70;
                                        rsi = rdi;
                                        ecx = var_80;
                                        rbx = rax;
                                        ecx += 0xADDR;
                                        if (ecx >= r12d) {
                                            edx = (r12 + 4);
                                            edx += ecx;
                                            if (edx >= ecx) {
                                                edi = (rdx + 0xADDR);
                                                if (edi >= edx) {
                                                    var_344 = edi;
                                                    GetProcessHeap();
                                                    HeapAlloc(rax, r12d, edi);
                                                    r11d = 0;
                                                    if (rax != 0) {
                                                        ecx = r11d;
                                                        var_348 = rax;
                                                        var_340 = ecx;
                                                        edx = var_b8;
                                                        r8 = (rax + 4);
                                                        r10d = (r12 - 7);
                                                        rdi = rax;
                                                        if (r8 >= rax) {
                                                            ecx = var_344;
                                                            rcx += rax;
                                                            rax += 8;
                                                            if (rax <= rcx) {
                                                                *(u32*)(rdi) = 4;
                                                                *(u32*)(r8) = 4;
                                                                ecx = var_340;
                                                                r9d = var_344;
                                                                ecx += r10d;
                                                                var_340 = ecx;
                                                                if (rbx != 0 && edx != 0) {
                                                                    if (var_348 != r11) {
                                                                        rdi = var_348;
                                                                        r8d = r11d;
                                                                        if (ecx == 0) {
                                                                            r10 = (rdi + 4);
                                                                            if (r10 >= rdi) {
                                                                                r8d = edx;
                                                                                ecx = r9d;
                                                                                rcx += var_348;
                                                                                rax = (r8 + 4);
                                                                                rax += rdi;
                                                                                if (rax <= rcx) {
                                                                                    *(u32*)(rdi) = edx;
                                                                                    if (rbx != 0) {
                                                                                        rdx = rbx;
                                                                                        rcx = r10;
                                                                                        sub_1400274ac(r10, rbx);
                                                                                        r11d = 0;
                                                                                    }
                                                                                    ecx = var_340;
                                                                                    r8d = var_344;
                                                                                    loc_ADDR:
                                                                                    ecx += 1;
                                                                                    var_340 = ecx;
                                                                                    if (rsi != 0) {
                                                                                        if (var_348 != 0) {
                                                                                            rdx = var_348;
                                                                                            edi = r11d;
                                                                                            if (ecx == 0) {
                                                                                                r9 = (rdx + 4);
                                                                                                if (r9 >= rdx) {
                                                                                                    ecx = r8d;
                                                                                                    rax = (r12 + 4);
                                                                                                    rcx += var_348;
                                                                                                    rax += rdx;
                                                                                                    edi = r12d;
                                                                                                    if (rax <= rcx) {
                                                                                                        *(u32*)(rdx) = r12d;
                                                                                                        r12d = 0;
                                                                                                        if (rsi != 0) {
                                                                                                            r8d = edi;
                                                                                                            rdx = rsi;
                                                                                                            rcx = r9;
                                                                                                            sub_1400274ac(r9, rsi, edi);
                                                                                                            r10d = (r12 + 1);
                                                                                                        }
                                                                                                        ecx = var_340;
                                                                                                        edi = var_344;
                                                                                                        ecx += r10d;
                                                                                                        loc_ADDR:
                                                                                                        var_340 = ecx;
                                                                                                        if (var_348 != r12) {
                                                                                                            rdx = var_348;
                                                                                                            r8d = r12d;
                                                                                                            if (ecx == 0) {
                                                                                                                r8 = (rdx + 4);
                                                                                                                if (r8 >= rdx) {
                                                                                                                    ecx = edi;
                                                                                                                    rax = (rdx + 0xADDR);
                                                                                                                    rcx += var_348;
                                                                                                                    if (rax <= rcx) {
                                                                                                                        rax = var_b0;
                                                                                                                        *(u32*)(rdx) = 8;
                                                                                                                        *(u64*)(r8) = rax;
                                                                                                                        var_340 += r10d;
                                                                                                                        r8d = var_344;
                                                                                                                        loc_ADDR:
                                                                                                                        var_80 = r12d;
                                                                                                                        rdi = r12;
                                                                                                                        var_b0 = r12;
                                                                                                                        var_140 = r12;
                                                                                                                        var_d0 = r12;
                                                                                                                        var_110 = r12;
                                                                                                                        var_100 = r12;
                                                                                                                        __asm("rdtsc");
                                                                                                                        rdx <<= 0xADDR;
                                                                                                                        rax |= rdx;
                                                                                                                        var_70 = rax;
                                                                                                                        r8d += 8;
                                                                                                                        if (r8d >= 8) {
                                                                                                                            rdx = (r8 + 7);
                                                                                                                            var_98 = rbx;
                                                                                                                            edx &= -0xADDR;
                                                                                                                            var_68 = rsi;
                                                                                                                            if (edx >= r8d) {
                                                                                                                                r12d = edx;
                                                                                                                                var_90 = r12;
                                                                                                                                GetProcessHeap();
                                                                                                                                HeapAlloc(rax, 8, r12d);
                                                                                                                                r8d = 0;
                                                                                                                                var_88 = rax;
                                                                                                                                r12 = rax;
                                                                                                                                if (rax == 0) {
                                                                                                                                    r14d = (0xADDR | 0xADDR);
                                                                                                                                    r12d = 0;
                                                                                                                                    loc_ADDR:
                                                                                                                                    if (r14d >= 0 /* signed */) {
                                                                                                                                        r15d = var_350;
                                                                                                                                        r10d = 1;
                                                                                                                                        if (r15d >= 1) {
                                                                                                                                            rdi = var_358;
                                                                                                                                            if (rdi != 0 && r15d != 0) {
                                                                                                                                                rcx = (rdi + 4);
                                                                                                                                                if (rcx >= rdi) {
                                                                                                                                                    rax = r12;
                                                                                                                                                    rax = ((*(u32*)(rdi) != r12d) ? rcx : rax);
                                                                                                                                                    if (*(u32*)(rdi) == 4 && *(u32*)(rax) >= r12d /* signed */ && rdi != r12 && r15d > r10d) {
                                                                                                                                                        rcx = rdi;
                                                                                                                                                        edi = r12d;
                                                                                                                                                        while (1) {
                                                                                                                                                            rdx = (rcx + 4);
                                                                                                                                                            if (rdx < rcx) break;
                                                                                                                                                            ecx = *(u32*)(rcx);
                                                                                                                                                            rcx += rdx;
                                                                                                                                                            if (rcx < rdx) break;
                                                                                                                                                            edi += r10d;
                                                                                                                                                            if (edi < r10d) continue;
                                                                                                                                                        }
                                                                                                                                                    }
                                                                                                                                                }
                                                                                                                                            }
                                                                                                                                        }
                                                                                                                                    }
                                                                                                                                } else {
                                                                                                                                    eax = var_340;
                                                                                                                                    rdx = (r12 + 4);
                                                                                                                                    *(u32*)(r12) = eax;
                                                                                                                                    if (rdx < r12) {
                                                                                                                                        eax = 0xADDR;
                                                                                                                                        var_70 = r12;
                                                                                                                                        var_a0 = eax;
                                                                                                                                        r12 = r8;
                                                                                                                                        var_98 = rbx;
                                                                                                                                        var_68 = rsi;
                                                                                                                                        var_88 = r8;
                                                                                                                                        GetProcessHeap();
                                                                                                                                        HeapFree(rax, 0, var_70);
                                                                                                                                        var_a0 |= 0xADDR;
                                                                                                                                        if ((var_a0 | 0xADDR) < 0 /* signed */) {
                                                                                                                                            r14d = var_a0;
                                                                                                                                            loc_ADDR:
                                                                                                                                            if (r12 != 0) {
                                                                                                                                                loc_ADDR:
                                                                                                                                                GetProcessHeap();
                                                                                                                                                HeapFree(rax, 0, r12);
                                                                                                                                            }
                                                                                                                                            r12d = 0;
                                                                                                                                            if (rdi != 0) {
                                                                                                                                                r15 = *(u64*)((rdi + 8));
                                                                                                                                                if (r15 != 0) {
                                                                                                                                                    GetProcessHeap();
                                                                                                                                                    HeapFree(rax, 0, r15);
                                                                                                                                                    *(u64*)((rdi + 8)) = r12;
                                                                                                                                                }
                                                                                                                                                r15 = *(u64*)((rdi + 0xADDR));
                                                                                                                                                if (r15 != 0) {
                                                                                                                                                    GetProcessHeap();
                                                                                                                                                    HeapFree(rax, 0, r15);
                                                                                                                                                    *(u64*)((rdi + 0xADDR)) = r12;
                                                                                                                                                }
                                                                                                                                                r15 = *(u64*)((rdi + 0xADDR));
                                                                                                                                                if (r15 != 0) {
                                                                                                                                                    GetProcessHeap();
                                                                                                                                                    HeapFree(rax, 0, r15);
                                                                                                                                                    *(u64*)((rdi + 0xADDR)) = r12;
                                                                                                                                                }
                                                                                                                                                GetProcessHeap();
                                                                                                                                                HeapFree(rax, 0, rdi);
                                                                                                                                            }
                                                                                                                                            rdi = var_140;
                                                                                                                                            if (rdi != 0) {
                                                                                                                                                GetProcessHeap();
                                                                                                                                                HeapFree(rax, 0, rdi);
                                                                                                                                            }
                                                                                                                                            rdi = var_d0;
                                                                                                                                            if (rdi != 0) {
                                                                                                                                                GetProcessHeap();
                                                                                                                                                HeapFree(rax, 0, rdi);
                                                                                                                                            }
                                                                                                                                            rdi = var_110;
                                                                                                                                            if (rdi != 0) {
                                                                                                                                                r15 = *(u64*)((rdi + 8));
                                                                                                                                                if (r15 != 0) {
                                                                                                                                                    GetProcessHeap();
                                                                                                                                                    HeapFree(rax, 0, r15);
                                                                                                                                                    *(u64*)((rdi + 8)) = r12;
                                                                                                                                                }
                                                                                                                                                r15 = *(u64*)((rdi + 0xADDR));
                                                                                                                                                if (r15 != 0) {
                                                                                                                                                    GetProcessHeap();
                                                                                                                                                    HeapFree(rax, 0, r15);
                                                                                                                                                    *(u64*)((rdi + 0xADDR)) = r12;
                                                                                                                                                }
                                                                                                                                                r15 = *(u64*)((rdi + 0xADDR));
                                                                                                                                                if (r15 != 0) {
                                                                                                                                                    GetProcessHeap();
                                                                                                                                                    HeapFree(rax, 0, r15);
                                                                                                                                                    *(u64*)((rdi + 0xADDR)) = r12;
                                                                                                                                                }
                                                                                                                                                GetProcessHeap();
                                                                                                                                                HeapFree(rax, 0, rdi);
                                                                                                                                            }
                                                                                                                                            rdi = var_100;
                                                                                                                                            if (rdi == 0) goto loc_ADDR;
                                                                                                                                        } else {
                                                                                                                                            eax = var_80;
                                                                                                                                            loc_ADDR:
                                                                                                                                            ecx = 0;
                                                                                                                                            var_70 = rcx;
                                                                                                                                            if (r12 != 0) {
                                                                                                                                                if (rax == 0) goto loc_ADDR;
                                                                                                                                            }
                                                                                                                                        }
                                                                                                                                    } else {
                                                                                                                                        eax = var_344;
                                                                                                                                        rcx = (rdx + 4);
                                                                                                                                        *(u32*)(rdx) = eax;
                                                                                                                                        if (rcx >= rdx) {
                                                                                                                                            eax = var_90;
                                                                                                                                            rdx = var_70;
                                                                                                                                            *(u64*)(((rax + r12) - 8)) = rdx;
                                                                                                                                            sub_1400274ac(rcx, var_348, var_344);
                                                                                                                                            eax = var_90;
                                                                                                                                            var_80 = eax;
                                                                                                                                            goto loc_ADDR;
                                                                                                                                        }
                                                                                                                                    }
                                                                                                                                }
                                                                                                                            }
                                                                                                                        }
                                                                                                                    }
                                                                                                                }
                                                                                                            } else {
                                                                                                                while (1) {
                                                                                                                    eax = (*(u32*)(rdx) + 4);
                                                                                                                    if (eax < 4) break;
                                                                                                                    r9d = eax;
                                                                                                                    r9 += rdx;
                                                                                                                    if (r9 < rdx) break;
                                                                                                                    r8d += r10d;
                                                                                                                    rdx = r9;
                                                                                                                    if (r8d < ecx) continue;
                                                                                                                }
                                                                                                            }
                                                                                                        } else {
                                                                                                            r8d = (rdi + 0xADDR);
                                                                                                            if (r8d >= edi) {
                                                                                                                ecx += r10d;
                                                                                                                var_344 = r8d;
                                                                                                                var_340 = ecx;
                                                                                                                goto loc_ADDR;
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            } else {
                                                                                                while (1) {
                                                                                                    eax = (*(u32*)(rdx) + 4);
                                                                                                    if (eax < 4) break;
                                                                                                    r9d = eax;
                                                                                                    r9 += rdx;
                                                                                                    if (r9 < rdx) break;
                                                                                                    edi += r10d;
                                                                                                    rdx = r9;
                                                                                                    if (edi < ecx) continue;
                                                                                                }
                                                                                            }
                                                                                        } else {
                                                                                            edi = (r12 + 4);
                                                                                            edi += r8d;
                                                                                            if (edi >= r8d) {
                                                                                                ecx += r10d;
                                                                                                var_344 = edi;
                                                                                                r12d = 0;
                                                                                                goto loc_ADDR;
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        } else {
                                                                            while (1) {
                                                                                eax = (*(u32*)(rdi) + 4);
                                                                                if (eax < 4) break;
                                                                                r10d = eax;
                                                                                r10 += rdi;
                                                                                if (r10 < rdi) break;
                                                                                r8d += 1;
                                                                                rdi = r10;
                                                                                if (r8d < ecx) continue;
                                                                            }
                                                                        }
                                                                    } else {
                                                                        edx += 4;
                                                                        if (edx >= 4) {
                                                                            r8d = (r9 + rdx);
                                                                            if (r8d >= r9d) {
                                                                                var_344 = r8d;
                                                                                goto loc_ADDR;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                edx = (rdi + 0xADDR);
                                var_70 = rbx;
                                r12d = (rdx + 0xADDR);
                                rax = 0xADDR;
                                var_80 = r12d;
                                rax += 0xADDR;
                                var_b8 = r12d;
                                xmm0 = *(__m128*)(rax);
                                r12d = 0;
                                *(__m128*)(rbx) = xmm0;
                                *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                                *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                                *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                                *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                                *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                                *(__m128*)((rbx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
                                *(__m128*)(((rbx + rdx) - 0xADDR)) = *(__m128*)((rax + 0xADDR));
                                xmm0 = *(__m128*)((rax + rdx));
                                *(__m128*)((rbx + rdx)) = xmm0;
                                xmm1 = *(__m128*)(((rax + rdx) + 0xADDR));
                                *(__m128*)(((rbx + rdx) + 0xADDR)) = xmm1;
                                goto loc_ADDR;
                            }
                            loc_ADDR:
                            var_340 = 0;
                            rdi = var_348;
                            if (rdi != 0) {
                                GetProcessHeap();
                                HeapFree(rax, 0, rdi);
                                var_348 = r12;
                            }
                            var_350 = 0;
                            rdi = var_358;
                            if (rdi != 0) {
                                GetProcessHeap();
                                HeapFree(rax, 0, rdi);
                                var_358 = r12;
                            }
                            if (rbx != 0) {
                                GetProcessHeap();
                                HeapFree(rax, 0, rbx);
                            }
                            if (rsi != 0) {
                                GetProcessHeap();
                                HeapFree(rax, 0, rsi);
                            }
                        }
                        rax = var_148;
                        esi = var_188;
                        var_1b8 = rax;
                        var_138 = r12;
                    }
                } else {
                    r13d = 0xADDR;
                    r12d = 0;
                }
                r14d = 1;
                do {
                    eax = 0;
                    __asm("lock cmpxchg [0xADDR], r14d");
                } while (dword_ADDR != eax);
                eax = dword_ADDR;
                if (eax > 0 /* signed */) {
                    eax -= r14d;
                    dword_ADDR = eax;
                    if (eax == 0) {
                        r15 = off_1400360c0;
                        ebx = 4;
                        rdi = r15;
                        do {
                            rcx = *(u64*)(rdi);
                            if (rcx != 0) {
                                FreeLibrary(*(u64*)(rdi));
                            }
                            rdi += 0xADDR;
                            rbx -= r14;
                        } while (rbx != 0);
                        edx = 0;
                        sub_1400029dc(r15, 0, (rbx + 0xADDR));
                        edx = (rbx + 2);
                        rax = off_140034000;
                        rcx = off_140029260;
                        edi = (rdx + 0xADDR);
                        do {
                            xmm0 = *(__m128*)(rcx);
                            xmm1 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)(rax) = xmm0;
                            xmm0 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm1;
                            xmm1 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm0;
                            xmm0 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm1;
                            xmm1 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm0;
                            xmm0 = *(__m128*)((rcx + 0xADDR));
                            *(__m128*)((rax + 0xADDR)) = xmm1;
                            xmm1 = *(__m128*)((rcx + 0xADDR));
                            rcx += rdi;
                            *(__m128*)((rax + 0xADDR)) = xmm0;
                            rax += rdi;
                            *(__m128*)((rax - 0xADDR)) = xmm1;
                            rdx -= r14;
                        } while (rdx != 0);
                        xmm0 = *(__m128*)(rcx);
                        xmm1 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)(rax) = xmm0;
                        xmm0 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm1;
                        xmm1 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm0;
                        xmm0 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm1;
                        xmm1 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm0;
                        xmm0 = *(__m128*)((rcx + 0xADDR));
                        *(__m128*)((rax + 0xADDR)) = xmm1;
                        *(__m128*)((rax + 0xADDR)) = xmm0;
                    }
                }
                eax = r12d;
                rcx = &var_138;
                __asm("xchg [0xADDR], eax");
                sub_140024c5c(&var_138);
                edi = 0xADDR;
                ebx = (rdi + 0xADDR);
                if (r13d >= 0 /* signed */) {
                    r13d = 0xADDR;
                    eax = r12d;
                    eax = ((esi != 4) ? r13d : eax);
                    r13d = eax;
                } else if (r13d != 0xADDR) {
                    if (r13d == 0xADDR) {
                        r13d = ebx;
                    } else if (r13d != 0xADDR) {
                        if (r13d != 0xADDR) {
                            if (r13d == 0xADDR) {
                                r13d = 0xADDR;
                            }
                        } else {
                            r13d = 0xADDR;
                        }
                    }
                } else {
                    r13d = edi;
                }
                sub_140024c5c(&var_1b8);
                if (r13d != ebx && r13d != 0xADDR) {
                    eax = (r13 + 0xADDR);
                    if (eax > r14d && r13d < 0 /* signed */ && r13d != edi) {
                        ecx = r13d;
                        sub_140024df4(r13d);
                    }
                }
                eax = r14d;
            } else {
                var_1b0 = rax;
                sub_14001278c(rsi, &var_1b0);
                rdi = var_1b0;
                r9d = 0;
                var_30 = r14;
                var_28 = 0xADDR;
                var_20 = 3;
                CreateFileW(rdi, 0xADDR, (r9 + 3), 0, 3, 0xADDR, r14);
                qword_ADDR = rax;
                if (rax != -1) {
                    loc_ADDR:
                    if (qword_ADDR != -1) {
                        r8d = 0;
                        sub_140010f48(&var_1b0, dword_ADDR, 0);
                        rdi = var_1b0;
                        if (al == 0) {
                            sub_140010b30(rdi);
                        }
                    }
                    if (rdi == 0) goto loc_ADDR;
                } else {
                    GetLastError();
                    if (eax != ebx) {
                        sub_140010b30(rdi);
                        rsi = qword_ADDR;
                        eax = 0;
                        if (rsi != 0) {
                            GetLastError();
                            ebx = eax;
                            CoTaskMemFree(rsi);
                            SetLastError(ebx);
                            eax = 0;
                        }
                        qword_ADDR = rax;
                        sub_14000fe3c(rcx);
                    } else {
                        var_20 = 0xADDR;
                        sub_14000c3a4(qword_ADDR, qword_ADDR, qword_ADDR, rdi, 0xADDR);
                        if (eax != ebx) {
                            if (eax != 6) goto loc_ADDR;
                        } else {
                            if (rdi == 0) {
                                eax = 0;
                                return;
                            }
                            CoTaskMemFree(rdi);
                            eax = 0;
                            return;
                        }
                    }
                }
            }
        } else {
            sub_14001e370(rsi);
            if (eax == 0) {
                sub_14001e5ac(rsi, r12d);
                if (eax == 0) {
                    sub_14001e2bc(rsi);
                } else {
                    r9d = 0;
                    PostMessageW(qword_ADDR, (r9 + 0xADDR), 0, 0);
                    eax = r13d;
                }
            } else if (eax == ebx) {
                eax = 0;
                return;
            }
        }
    }
    return;
    do {
        rdi = off_1400360c0;
        do {
            rax += 1;
        } while (*(u16*)((rbx + (rax * 2))) != r8w);
        rbx += (rax * 2);
        eax = r13d;
        rcx = (rax + (rax * 2));
        rdi += (rcx * 8);
        GetModuleHandleExW(0, rdx, rdi);
        r8d = 0;
        if (eax == 0) goto loc_ADDR;
        rax = *(u64*)(rdi);
        ecx = 0xADDR;
        r14d = r8d;
        if (*(u16*)(rax) == cx) goto loc_ADDR;
        loc_ADDR:
        do {
            r14d = 0xADDR;
            goto loc_ADDR;
            loc_ADDR:
            rcx = (int64)(*(u32*)((rax + 0xADDR)));
            if (ecx >= 0xADDR) goto loc_ADDR;
            rcx += *(u64*)(rdi);
            if (rcx < *(u64*)(rdi)) goto loc_ADDR;
        } while (*(u32*)(rcx) != 0xADDR);
        eax = (uint32)(*(u16*)((rcx + 0xADDR)));
        ax -= dx;
        if ((0xADDR & ax) != 0) {
            r14d = 0xADDR;
            goto loc_ADDR;
        }
        rax = *(u64*)((rcx + 0xADDR));
        *(u64*)((rdi + 0xADDR)) = rax;
        eax = *(u32*)((rcx + 0xADDR));
        *(u32*)((rdi + 8)) = eax;
        loc_ADDR:
        r15d = *(u32*)((rbx + 2));
        edi = r8d;
        rbx += 6;
        if (r15d != 0) {
            eax = r13d;
            r12 = (rax + (rax * 2));
            do {
                rdx = rbx;
                rax = -1;
                do {
                    rax += 1;
                } while (*(u8*)((rbx + rax)) != r8b);
                rbx += 1;
                rbx += rax;
                if (r14d >= 0 /* signed */) {
                    rax = off_1400360c0;
                    GetProcAddress(*(u64*)((rax + (r12 * 8))), rdx);
                    r8d = 0;
                    if (rax == 0) goto loc_ADDR;
                    ecx = var_a8;
                    rdx = off_140034000;
                    *(u64*)((rdx + (rcx * 8))) = rax;
                }
                var_a8 += 1;
                edi += 1;
            } while (edi < r15d);
            goto loc_ADDR;
        }
        eax = 1;
        loc_ADDR:
        r13d += 1;
    } while (*(u8*)(rbx) != r8b);
    goto loc_ADDR;
    loc_ADDR:
    r14d = 0xADDR;
    loc_ADDR:
    if (rsi == 0) goto loc_ADDR;
    ebx = 0xADDR;
    goto loc_ADDR;
    ebx = 0xADDR;
    loc_ADDR:
    r14d = 0;
    goto loc_ADDR;
    r14d = 0;
    if (eax == 0) {
        ebx = 0xADDR;
        goto loc_ADDR;
    }
    if (eax <= 0 /* signed */) {
        ebx = eax;
        goto loc_ADDR;
    }
    ebx = (uint32)(ax);
    ebx |= r13d;
    goto loc_ADDR;
    ebx = 0xADDR;
    r14d = 0;
    goto loc_ADDR;
    r12d = 0xADDR;
    r14d = 0;
    ebx = r12d;
    goto loc_ADDR;
    r14d = 0;
    r12d = 0xADDR;
    if (eax == 0) {
        ebx = r12d;
        goto loc_ADDR;
    }
    if (eax <= 0 /* signed */) {
        ebx = eax;
        goto loc_ADDR;
    }
    ebx = (uint32)(ax);
    ebx |= r13d;
    goto loc_ADDR;
    eax = (r11 + rdi);
    r9d = 1;
    if (eax >= r11d) {
        r10d += r9d;
        var_2f4 = eax;
        var_2f0 = r10d;
        r11d = eax;
        goto loc_ADDR;
    }
    ecx = -1;
    r13d = 0xADDR;
    r11d = ecx;
    var_2f4 = ecx;
    goto loc_ADDR;
    rax = (r13 + 8);
    var_88 = rax;
    sub_140024bf8(rax);
    ecx = 0;
    var_90 = rax;
    r8 = rax;
    if (rax == 0) goto loc_ADDR;
    var_60 = cl;
    eax = ecx;
    if (r13 != 0) {
        edx = (rcx + 1);
        do {
            cl ^= *(u8*)((rax + r12));
            rax += rdx;
        } while (rax < r13);
        var_60 = cl;
        ecx = 0;
    }
    rax = var_f0;
    r9 = r8;
    var_68 = r8;
    r11d = -1;
    r8 = rax;
    var_890 = r14;
    var_78 = ecx;
    rdx = r12;
    r10d = ecx;
    r13d = ecx;
    r8d &= 7;
    if (r8d == 0) goto loc_ADDR;
    var_d8 = ecx;
    r9d = ecx;
    r11d = ecx;
    if (r8d == 0) goto loc_ADDR;
    edi = 1;
    r10d = (rdi + 0xADDR);
    do {
        eax = (uint32)(*(u8*)(rdx));
        if (r11d < 4) {
            eax <<= cl;
            ecx = var_d8;
            ecx |= eax;
            var_d8 = ecx;
            goto loc_ADDR;
        }
        eax <<= cl;
        ecx = var_d8;
        r9d |= eax;
        loc_ADDR:
        rdx += 1;
        r11d += edi;
        r10d -= 8;
    } while (r11d < r8d /* signed */);
    r14 = var_68;
    r11d = (r9d ^ 0xADDR);
    var_78 = r9d;
    r15d = var_78;
    r10d = (ecx ^ 0xADDR);
    r13d = r11d;
    eax = 0;
    r9d = r10d;
    var_118 = eax;
    edi = eax;
    do {
        var_d8 = ecx;
        if (eax < 4 /* signed */) {
            ecx = r9d;
            eax = r9d;
            ecx >>= 0xADDR;
            eax <<= 8;
            r9d = ecx;
            r9d |= eax;
            eax = r9d;
            goto loc_ADDR;
        }
        ecx = r13d;
        eax = r13d;
        ecx >>= 0xADDR;
        eax <<= 8;
        r13d = ecx;
        r13d |= eax;
        eax = r13d;
        loc_ADDR:
        *(u8*)(r14) = al;
        rcx = (r14 + 1);
        eax = var_118;
        r14 = rcx;
        ecx = var_d8;
        eax += 1;
        var_118 = eax;
    } while (eax < r8d /* signed */);
    rax = var_f0;
    r9 = r14;
    r13d = ecx;
    var_78 = r15d;
    r14 = 0xADDR;
    var_f8 = r12;
    ecx = 0;
    var_120 = rsi;
    var_c0 = rbx;
    goto loc_ADDR;
    loc_ADDR:
    r9 = var_68;
    r10d = 0xADDR;
    r11d = 0xADDR;
    var_78 = ecx;
    loc_ADDR:
    if (r8d > 4) {
        ecx = 8;
        ecx -= r8d;
        ecx <<= 3;
        r11d >>= cl;
        r11d <<= cl;
        goto loc_ADDR;
    }
    r11d = ecx;
    __asm("if (jae short 0xADDR /* unmodelled flags */) goto 0xADDR");
    ecx = 4;
    ecx -= r8d;
    ecx <<= 3;
    r10d >>= cl;
    r10d <<= cl;
    loc_ADDR:
    rax >>= 3;
    var_68 = rax;
    if (rax == 0) goto loc_ADDR;
    eax = (uint32)(var_896);
    rcx = (rdx + 2);
    edi = var_894;
    r9 += 7;
    r15d = var_78;
    r12 = var_68;
    var_d8 = eax;
    var_118 = r9;
    ebx = (uint32)(r14w);
    esi = (uint32)(di);
    do {
        eax = (uint32)(*(u8*)((rcx - 1)));
        r9d = (uint32)(*(u8*)((rcx - 2)));
        r8d = (uint32)(*(u8*)((rcx + 2)));
        edx = (uint32)(var_892);
        r9d <<= 8;
        r9d |= eax;
        r8d <<= 8;
        eax = (uint32)(*(u8*)(rcx));
        r9d <<= 8;
        r9d |= eax;
        eax = (uint32)(*(u8*)((rcx + 1)));
        r9d <<= 8;
        r9d |= eax;
        eax = (uint32)(*(u8*)((rcx + 3)));
        r8d |= eax;
        eax = (uint32)(*(u8*)((rcx + 4)));
        r8d <<= 8;
        r8d |= eax;
        eax = (uint32)(*(u8*)((rcx + 5)));
        rcx += 8;
        r8d <<= 8;
        r8d |= eax;
        var_68 = rcx;
        r11d ^= r8d;
        ecx = r11d;
        ecx -= ebx;
        ecx ^= r9d;
        ecx ^= r10d;
        r10d = ecx;
        ecx = ror(ecx, 0xADDR);
        ecx *= edx;
        r10d ^= edi;
        eax = r10d;
        eax = ror(eax, 7);
        ecx += eax;
        r11d ^= ecx;
        eax = r11d;
        eax = ror(eax, 0xADDR);
        ecx = (r14 + r11);
        ecx = ror(ecx, 9);
        ecx *= esi;
        ecx -= eax;
        eax = (uint32)(var_896);
        r10d ^= ecx;
        ecx = r10d;
        ecx ^= esi;
        ecx = ror(ecx, 0xADDR);
        ecx *= eax;
        eax = r10d;
        eax = ror(eax, 0xADDR);
        ecx += eax;
        eax = edi;
        r11d ^= ecx;
        ecx = r14d;
        ecx ^= r11d;
        eax -= ecx;
        r10d ^= eax;
        ecx = r10d;
        eax = (r10d >> 6);
        ecx -= ebx;
        ecx *= edx;
        ecx -= eax;
        r11d ^= ecx;
        ecx = (uint32)(var_896);
        eax = r11d;
        eax = ror(eax, 0xADDR);
        eax ^= esi;
        eax *= ebx;
        r10d ^= eax;
        eax = r10d;
        eax = ~(eax);
        eax = ror(eax, 3);
        eax += ecx;
        eax *= esi;
        r11d ^= eax;
        eax = r11d;
        eax -= ebx;
        eax -= edi;
        r10d ^= eax;
        ecx = r10d;
        ecx = ror(ecx, 0xADDR);
        eax = r10d;
        eax ^= var_d8;
        eax *= edx;
        ecx ^= eax;
        r11d ^= ecx;
        ecx = ebx;
        ecx ^= r11d;
        eax = r11d;
        ecx = ror(ecx, 0xADDR);
        ecx *= esi;
        eax = ror(eax, 3);
        ecx ^= eax;
        r10d ^= ecx;
        ecx = (uint32)(var_896);
        eax = r10d;
        eax = ror(eax, 0xADDR);
        eax -= ecx;
        eax *= ebx;
        r11d ^= eax;
        eax = (uint32)(var_896);
        ecx = r11d;
        ecx ^= eax;
        eax = r11d;
        ecx *= ebx;
        eax >>= 1;
        ecx ^= eax;
        eax = r11d;
        r10d ^= ecx;
        eax -= esi;
        ecx = eax;
        eax <<= 3;
        ecx >>= 0xADDR;
        ecx |= eax;
        eax = r11d;
        ecx *= ebx;
        eax >>= 0xADDR;
        ecx ^= eax;
        r10d ^= ecx;
        ecx = r10d;
        eax = r10d;
        ecx -= esi;
        eax >>= 0xADDR;
        ecx *= edx;
        ecx -= eax;
        r11d ^= ecx;
        ecx = r14d;
        ecx -= r11d;
        eax = r11d;
        ecx = ror(ecx, 9);
        ecx *= esi;
        eax = ror(eax, 0xADDR);
        ecx ^= eax;
        r10d ^= ecx;
        ecx = (uint32)(var_896);
        eax = r10d;
        eax -= r14d;
        eax -= ecx;
        r11d ^= eax;
        eax = r11d;
        ecx = r11d;
        ecx = ror(ecx, 7);
        eax ^= edx;
        eax *= ebx;
        eax -= ecx;
        r10d ^= eax;
        eax = (uint32)(var_896);
        ecx = r10d;
        ecx ^= eax;
        eax = r10d;
        ecx = ror(ecx, 0xADDR);
        ecx *= edx;
        eax = ror(eax, 0xADDR);
        ecx -= eax;
        r11d ^= ecx;
        ecx = r14d;
        ecx -= r11d;
        eax = r11d;
        ecx = ror(ecx, 0xADDR);
        ecx *= esi;
        eax = ror(eax, 4);
        ecx += eax;
        eax = (uint32)(var_896);
        r10d ^= ecx;
        ecx = r10d;
        ecx -= r14d;
        ecx = ror(ecx, 4);
        ecx *= eax;
        eax = r10d;
        eax = ror(eax, 9);
        ecx ^= eax;
        r11d ^= ecx;
        ecx = edi;
        ecx ^= r11d;
        eax = r11d;
        ecx = ror(ecx, 0xADDR);
        ecx *= ebx;
        eax = ror(eax, 0xADDR);
        ecx -= eax;
        r10d ^= ecx;
        ecx = edi;
        ecx -= r10d;
        eax = r10d;
        ecx = ror(ecx, 0xADDR);
        ecx *= edx;
        eax = ror(eax, 0xADDR);
        ecx -= eax;
        r11d ^= ecx;
        ecx = edx;
        ecx ^= r11d;
        eax = (r11d >> 8);
        r11d ^= r14d;
        r11d ^= edi;
        ecx *= esi;
        ecx ^= eax;
        r10d ^= ecx;
        r11d ^= r10d;
        r10d ^= r13d;
        r11d ^= r15d;
        ecx = r10d;
        r15 = var_118;
        eax = r10d;
        var_118 += 8;
        edx = r11d;
        eax >>= 8;
        r13d = r9d;
        ecx <<= 0xADDR;
        ecx |= eax;
        *(u8*)((r15 - 4)) = r10b;
        *(u8*)(r15) = r11b;
        edi = ecx;
        *(u8*)((r15 - 5)) = cl;
        eax = (r11d >> 8);
        edi <<= 0xADDR;
        edx <<= 0xADDR;
        edx |= eax;
        ecx >>= 8;
        *(u8*)((r15 - 1)) = dl;
        edi |= ecx;
        rcx = var_68;
        eax = edx;
        *(u8*)((r15 - 6)) = dil;
        eax <<= 0xADDR;
        edi = ror(edi, 8);
        edx >>= 8;
        eax |= edx;
        *(u8*)((r15 - 2)) = al;
        *(u8*)((r15 - 7)) = dil;
        edi = var_894;
        eax = ror(eax, 8);
        *(u8*)((r15 - 3)) = al;
        r15d = r8d;
        r12 -= 1;
    } while (r12 != 0);
    rbx = var_c0;
    rsi = var_120;
    r12 = var_f8;
    rdi = var_80;
    loc_ADDR:
    rcx = var_f0;
    rdx = var_90;
    eax = (uint32)(var_60);
    *(u64*)((rdx + rcx)) = rax;
    GetProcessHeap();
    edx = 8;
    HeapAlloc(rax, 8, (rdx + 0xADDR));
    var_68 = rax;
    r13 = rax;
    if (rax == 0) goto loc_ADDR;
    rax = var_88;
    *(u32*)(r13) = eax;
    GetProcessHeap();
    r8 = var_88;
    HeapAlloc(rax, 8, var_88);
    if (rax == 0) goto loc_ADDR;
    *(u64*)((r13 + 8)) = rax;
    r8 = var_88;
    sub_1400274ac(rax, var_90);
    var_68 = r13;
    var_80 = rbx;
    *(u32*)((r13 + 0xADDR)) = 0xADDR;
    GetProcessHeap();
    HeapAlloc(rax, 8, 0xADDR);
    if (rax == 0) goto loc_ADDR;
    *(u64*)((r13 + 0xADDR)) = rax;
    rcx = rax;
    edx = 0xADDR;
    rax = (0xADDR + 0xADDR);
    *(__m128*)(rcx) = *(__m128*)(rax);
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)(((rcx + rdx) - 0xADDR)) = *(__m128*)((rax + 0xADDR));
    xmm0 = *(__m128*)((rax + rdx));
    *(__m128*)((rcx + rdx)) = xmm0;
    xmm1 = *(__m128*)(((rax + rdx) + 0xADDR));
    *(__m128*)(((rcx + rdx) + 0xADDR)) = xmm1;
    *(u32*)((r13 + 0xADDR)) = 8;
    GetProcessHeap();
    rcx = rax;
    eax = 8;
    HeapAlloc(rax, eax, eax);
    ecx = 0;
    if (rax == 0) {
        var_68 = r13;
        rbx = var_80;
        loc_ADDR:
        rax = var_68;
        r13d = 0xADDR;
        rcx = *(u64*)((rax + 8));
        var_88 = rcx;
        if (rcx != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_88);
            rax = var_68;
            ecx = 0;
            *(u64*)((rax + 8)) = rcx;
        }
        rcx = *(u64*)((rax + 0xADDR));
        var_88 = rcx;
        if (rcx != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_88);
            rax = var_68;
            ecx = 0;
            *(u64*)((rax + 0xADDR)) = rcx;
        }
        rax = *(u64*)((rax + 0xADDR));
        var_88 = rax;
        if (rax != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_88);
            rax = var_68;
            ecx = 0;
            *(u64*)((rax + 0xADDR)) = rcx;
        }
        GetProcessHeap();
        HeapFree(rax, 0, var_68);
        goto loc_ADDR;
    }
    *(u64*)((r13 + 0xADDR)) = rax;
    rdx = var_68;
    r13d = ecx;
    rax = qword_ADDR;
    rcx = *(u64*)((rdx + 0xADDR));
    *(u64*)(rcx) = rax;
    rax = rdx;
    goto loc_ADDR;
    loc_ADDR:
    r13d = 0xADDR;
    loc_ADDR:
    if (r13d >= 0 /* signed */) {
        rax = var_c8;
        loc_ADDR:
        rdi = rax;
        eax = 0;
        var_c8 = rax;
    }
    GetProcessHeap();
    HeapFree(rax, 0, var_90);
    rax = var_c8;
    if (rax != 0) {
        rcx = *(u64*)((rax + 8));
        var_68 = rcx;
        if (rcx != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_68);
            rax = var_c8;
            edx = 0;
            *(u64*)((rax + 8)) = rdx;
        }
        rcx = *(u64*)((rax + 0xADDR));
        var_68 = rcx;
        if (rcx != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_68);
            rax = var_c8;
            edx = 0;
            *(u64*)((rax + 0xADDR)) = rdx;
        }
        rax = *(u64*)((rax + 0xADDR));
        var_68 = rax;
        if (rax != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_68);
            rax = var_c8;
            ecx = 0;
            *(u64*)((rax + 0xADDR)) = rcx;
        }
        GetProcessHeap();
        HeapFree(rax, 0, var_c8);
    }
    r13d |= 0xADDR;
    var_d8 = r13d;
    if (r13d < 0 /* signed */) goto loc_ADDR;
    eax = *(u32*)((rdi + 0xADDR));
    r11d = *(u32*)((rdi + 0xADDR));
    var_78 = eax;
    eax = (*(u32*)(rdi) + 4);
    if (eax < 4) goto loc_ADDR;
    edx = (rax + 4);
    if (edx < eax) goto loc_ADDR;
    ecx = (r11 + rdx);
    if (ecx < edx) goto loc_ADDR;
    eax = (rcx + 4);
    if (eax < ecx) goto loc_ADDR;
    ecx = var_78;
    ecx += eax;
    if (ecx < eax) goto loc_ADDR;
    eax = ecx;
    var_68 = rax;
    GetProcessHeap();
    HeapAlloc(rax, 8, var_68);
    var_90 = rax;
    rcx = rax;
    if (rax == 0) goto loc_ADDR;
    eax = *(u32*)(rdi);
    *(u32*)(rcx) = eax;
    rax = (rcx + 4);
    var_88 = rax;
    if (rax < rcx) goto loc_ADDR;
    sub_1400274ac(rax, *(u64*)((rdi + 8)), *(u32*)(rdi));
    edx = *(u32*)(rdi);
    rdx += var_88;
    if (rdx < var_88) goto loc_ADDR;
    eax = *(u32*)((rdi + 0xADDR));
    *(u32*)(rdx) = eax;
    rax = (rdx + 4);
    var_88 = rax;
    if (rax < rdx) goto loc_ADDR;
    sub_1400274ac(rax, *(u64*)((rdi + 0xADDR)), *(u32*)((rdi + 0xADDR)));
    ecx = *(u32*)((rdi + 0xADDR));
    rcx += var_88;
    if (rcx < var_88) goto loc_ADDR;
    eax = *(u32*)((rdi + 0xADDR));
    *(u32*)(rcx) = eax;
    rax = (rcx + 4);
    var_88 = rax;
    if (rax < rcx) goto loc_ADDR;
    sub_1400274ac(rax, *(u64*)((rdi + 0xADDR)), *(u32*)((rdi + 0xADDR)));
    eax = *(u32*)((rdi + 0xADDR));
    rax += var_88;
    if (rax >= var_88) {
        rcx = var_90;
        eax = var_68;
        var_a0 = rcx;
        goto loc_ADDR;
        loc_ADDR:
        rax = var_90;
        var_90 = rax;
        goto loc_ADDR;
        loc_ADDR:
        var_90 = rcx;
    }
    loc_ADDR:
    rax = var_a0;
    r13d = 0xADDR;
    var_a0 = rax;
    GetProcessHeap();
    HeapFree(rax, 0, var_90);
    rcx = var_a0;
    r13d |= 0xADDR;
    var_d8 = r13d;
    if (r13d < 0 /* signed */) goto loc_ADDR;
    eax = ecx;
    loc_ADDR:
    r13d = 0;
    if (var_2f8 != 0) {
        edx = (r13 + 1);
        if (var_2f0 > edx) goto loc_ADDR;
    }
    do {
        r13d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        rdx = var_2f8;
        r9d = r13d;
        do {
            r8 = (rdx + 4);
            var_a8 = eax;
            var_a0 = rcx;
            r11 = r12;
            if (r8 < rdx) goto loc_ADDR;
            edx = *(u32*)(rdx);
            rdx += r8;
            if (rdx < r8) goto loc_ADDR;
            r9d += 1;
            eax = var_a8;
        } while (r9d < eax);
        r13d = *(u32*)(rdx);
        rax = (rdx + 4);
        if (rax < rdx) goto loc_ADDR;
        rax = rcx;
        var_a0 = rcx;
        ecx = 0;
    } while (var_2f8 == rcx);
    if (var_2f0 <= 2) {
        r13d = 0xADDR;
        loc_ADDR:
        var_d8 = r13d;
        var_a0 = rax;
        GetProcessHeap();
        HeapFree(rax, 0, rdi);
        goto loc_ADDR;
    }
    rdx = var_2f8;
    r8 = rdi;
    var_78 = ecx;
    r9 = rsi;
    r10 = rbx;
    do {
        ecx = var_a8;
        r12 = r11;
        var_118 = r13d;
        rbx = r10;
        r13 = (rdx + 4);
        var_a8 = ecx;
        var_a0 = rax;
        rcx = rax;
        rsi = r9;
        rdi = r8;
        if (r13 < rdx) goto loc_ADDR;
        edx = *(u32*)(rdx);
        rdx += r13;
        if (rdx < r13) goto loc_ADDR;
        r13d = (var_78 + 1);
        var_78 = r13d;
        r13d = var_118;
    } while (r13d < 2);
    r8d = *(u32*)(rdx);
    rax = (rdx + 4);
    var_78 = r8d;
    if (rax < rdx) goto loc_ADDR;
    eax = (r13 + 0xADDR);
    if (eax < 0xADDR) goto loc_ADDR;
    edx = (rax + 4);
    if (edx < eax) goto loc_ADDR;
    eax = (rdx + r8);
    if (eax < edx) goto loc_ADDR;
    if (eax > 0xADDR) {
        r13d = 0xADDR;
        goto loc_ADDR;
    }
    r13d = eax;
    var_68 = rdi;
    var_88 = rbx;
    var_c8 = rcx;
    GetProcessHeap();
    HeapAlloc(rax, 8, r13d);
    r10 = var_e0;
    r9 = rsi;
    rcx = rax;
    r8 = rbx;
    rdx = r12;
    r10 = ((rax != 0) ? rax : r10);
    rcx = var_c8;
    r13d = ((rcx != 0) ? -1 : 0);
    var_e0 = r10;
    r13d &= 0xADDR;
    var_a0 = rcx;
    r13d += 0xADDR;
    var_d8 = r13d;
    if (rax == 0) goto loc_ADDR;
    ebx = 0;
    xmm0 = 0;
    var_218 = rbx;
    var_380 = xmm0;
    var_390 = xmm0;
    if (rcx != 0) goto loc_ADDR;
    do {
        r13d = 0xADDR;
        rbx = r8;
        goto loc_ADDR;
        loc_ADDR:
    } while (r10 == 0);
    eax = var_a8;
    edi = var_118;
    var_380 = rcx;
    edi += 0xADDR;
    edi += var_78;
    var_394 = edi;
    var_390 = eax;
    var_388 = r10;
    var_398 = ebx;
    GetModuleHandleExW(1, "ntdll.dll", &var_218, rsi);
    if (eax != 0) goto loc_ADDR;
    loc_ADDR:
    GetLastError();
    var_d8 = eax;
    r13d = eax;
    if (eax > 0 /* signed */) {
        r13d = (uint32)(ax);
        r13d |= 0xADDR;
        var_d8 = r13d;
    }
    edx = edi;
    if (r13d >= 0 /* signed */) {
        r13d = 0xADDR;
        var_d8 = r13d;
    }
    do {
        if (r13d != 0xADDR) goto loc_ADDR;
        rdi = var_68;
        r13d = 0xADDR;
        rbx = var_88;
        rcx = var_c8;
        loc_ADDR:
        var_d8 = r13d;
        var_a0 = rcx;
        goto loc_ADDR;
        loc_ADDR:
        GetProcAddress(var_218, "NtQuerySystemInformation");
        if (rax == 0) goto loc_ADDR;
        r9d = 0;
        r8d = (r9 + 0xADDR);
        sub_140028010((r8 + 0xADDR), &var_380, (r9 + 0xADDR));
        r13d = eax;
        edx = edi;
        r13d |= 0xADDR;
        var_d8 = r13d;
    } while (r13d < 0 /* signed */);
    edx = var_394;
    r9 = r12;
    r8 = var_c8;
    r11 = rsi;
    r10 = var_88;
    rax = var_68;
    goto loc_ADDR;
    loc_ADDR:
    rax = var_68;
    r11 = rsi;
    r10 = var_88;
    rdi = rax;
    r8 = var_c8;
    rbx = r10;
    var_a0 = r8;
    r9 = r12;
    if (r13d < 0 /* signed */) goto loc_ADDR;
    ebx = 0;
    loc_ADDR:
    var_90 = rbx;
    rbx = r10;
    rsi = r11;
    r12 = r9;
    if (edx >= 4) goto loc_ADDR;
    rdi = rax;
    loc_ADDR:
    do {
        r13d = 0xADDR;
        loc_ADDR:
        var_d8 = r13d;
        var_a0 = r8;
        goto loc_ADDR;
        loc_ADDR:
        rdi = var_e0;
        r13d = *(u32*)(rdi);
        rcx = (rdi + 4);
        var_78 = r13d;
        rdi = rax;
        if (rcx < rdi) goto loc_ADDR;
        eax = (rdx - 4);
        rcx = r8;
        var_a0 = rcx;
        if (eax < r13d) {
            r13d = 0xADDR;
            loc_ADDR:
            rdi = var_68;
            goto loc_ADDR;
        }
        r14 = var_e0;
        r15 = 0xADDR;
        r14 += 4;
        rax = (r14 + r13);
        var_88 = rax;
        r14 = 0xADDR;
        if (rax < r14) goto loc_ADDR;
        r13d += 4;
        if (r13d < 4) goto loc_ADDR;
        eax = edx;
        eax -= r13d;
        if (eax >= 4) goto loc_ADDR;
        loc_ADDR:
        do {
            r13d = 0xADDR;
            goto loc_ADDR;
            loc_ADDR:
            rax = var_88;
            var_a0 = rcx;
            r8d = *(u32*)(rax);
            r11 = (rax + 4);
            var_d8 = r8d;
            var_88 = r11;
            if (r11 < rax) goto loc_ADDR;
            r9d = (r13 + 4);
            rax = rbx;
            if (r9d < r13d) goto loc_ADDR;
            eax = edx;
            eax -= r9d;
            if (eax < r8d) goto loc_ADDR;
            r10 = (r11 + r8);
            var_a0 = rcx;
            if (r10 < r11) goto loc_ADDR;
            r8d += r9d;
            rax = r12;
            var_a8 = r8d;
            r11 = rbx;
            if (r8d < r9d) goto loc_ADDR;
            eax = edx;
            eax -= r8d;
            if (eax < 4) goto loc_ADDR;
            eax = *(u32*)(r10);
            r9 = (r10 + 4);
            var_118 = eax;
            var_98 = r9;
            var_a0 = rcx;
            if (r9 < r10) goto loc_ADDR;
            eax = (r8 + 4);
            r9 = rbx;
            r10 = rsi;
            if (eax < r8d) goto loc_ADDR;
            r14d = var_a8;
            eax = edx;
            r14d += 4;
            var_68 = rcx;
            eax -= r14d;
            r8 = rcx;
            r14 = 0xADDR;
        } while (eax < var_118);
        r14d = var_a8;
        eax = (var_118 + 4);
        r14d += eax;
        eax = (var_a8 + 4);
        var_f0 = r14d;
        r14 = 0xADDR;
        if (r14d < eax) goto loc_ADDR;
        var_a0 = rcx;
        var_f8 = r12;
        var_c0 = rbx;
        var_120 = rsi;
        var_80 = rdi;
        if (edx != var_f0) goto loc_ADDR;
        ecx = var_d8;
        ecx += var_78;
        ecx += var_118;
        rcx += 0xADDR;
        eax = edx;
    } while (rcx != rax);
    GetProcessHeap();
    edx = 8;
    HeapAlloc(rax, 8, (rdx + 0xADDR));
    r8d = 0;
    var_e8 = rax;
    rcx = rax;
    if (rax == 0) goto loc_ADDR;
    rax = var_e0;
    r13d = r8d;
    rax += 4;
    if (rax != 0) {
        eax = var_78;
        *(u32*)(rcx) = eax;
        r13d = eax;
        var_f0 = rax;
        GetProcessHeap();
        HeapAlloc(rax, 8, r13d);
        edx = 0;
        rcx = rax;
        if (rax == 0) {
            rax = var_68;
            var_a0 = rax;
            var_80 = rdi;
            var_120 = rsi;
            var_c0 = rbx;
            var_f8 = r12;
            goto loc_ADDR;
        }
        rax = var_e8;
        *(u64*)((rax + 8)) = rcx;
        r13d = edx;
        rdx = (var_e0 + 4);
        sub_1400274ac(rcx, rdx, var_f0);
        rcx = var_e8;
        r8d = 0;
        rdx = var_98;
        goto loc_ADDR;
    }
    rdx = var_98;
    rax = var_68;
    *(u32*)(rcx) = r8d;
    var_98 = rdx;
    var_a0 = rax;
    *(u64*)((rcx + 8)) = r8;
    var_80 = rdi;
    var_120 = rsi;
    var_c0 = rbx;
    var_f8 = r12;
    loc_ADDR:
    var_e8 = rcx;
    if (var_88 != r8) {
        eax = var_d8;
        *(u32*)((rcx + 0xADDR)) = eax;
        r13d = eax;
        var_d0 = rax;
        GetProcessHeap();
        HeapAlloc(rax, 8, r13d);
        edx = 0;
        rcx = rax;
        if (rax == 0) goto loc_ADDR;
        rax = var_e8;
        *(u64*)((rax + 0xADDR)) = rcx;
        r13d = edx;
        sub_1400274ac(rcx, var_88, var_d0);
        rcx = var_e8;
        r8d = 0;
        rdx = var_98;
        rax = var_a0;
        var_e8 = rcx;
        var_98 = rdx;
        var_a0 = rax;
        var_80 = rdi;
        var_120 = rsi;
        var_c0 = rbx;
        var_f8 = r12;
        goto loc_ADDR;
    }
    *(u32*)((rcx + 0xADDR)) = r8d;
    *(u64*)((rcx + 0xADDR)) = r8;
    loc_ADDR:
    if (rdx != 0) {
        eax = var_118;
        *(u32*)((rcx + 0xADDR)) = eax;
        r13d = eax;
        var_d0 = rax;
        GetProcessHeap();
        HeapAlloc(rax, 8, r13d);
        edx = 0;
        rcx = rax;
        if (rax == 0) {
            loc_ADDR:
            rax = var_e8;
            r13d = 0xADDR;
            rcx = *(u64*)((rax + 8));
            var_d0 = rcx;
            if (rcx != 0) {
                GetProcessHeap();
                HeapFree(rax, 0, var_d0);
                rax = var_e8;
                edx = 0;
                *(u64*)((rax + 8)) = rdx;
            }
            rcx = *(u64*)((rax + 0xADDR));
            var_d0 = rcx;
            if (rcx != 0) {
                GetProcessHeap();
                HeapFree(rax, 0, var_d0);
                rax = var_e8;
                edx = 0;
                *(u64*)((rax + 0xADDR)) = rdx;
            }
            rax = *(u64*)((rax + 0xADDR));
            var_d0 = rax;
            if (rax != 0) {
                GetProcessHeap();
                HeapFree(rax, 0, var_d0);
                rax = var_e8;
                ecx = 0;
                *(u64*)((rax + 0xADDR)) = rcx;
            }
            GetProcessHeap();
            HeapFree(rax, 0, var_e8);
            goto loc_ADDR;
        }
        rax = var_e8;
        *(u64*)((rax + 0xADDR)) = rcx;
        r13d = edx;
        sub_1400274ac(rcx, var_98, var_d0);
        rax = var_a0;
        rcx = var_e8;
        var_a0 = rax;
        var_80 = rdi;
        var_120 = rsi;
        var_c0 = rbx;
        var_f8 = r12;
        goto loc_ADDR;
    }
    *(u32*)((rcx + 0xADDR)) = r8d;
    *(u64*)((rcx + 0xADDR)) = r8;
    loc_ADDR:
    var_90 = rcx;
    goto loc_ADDR;
    loc_ADDR:
    rax = var_68;
    r13d = 0xADDR;
    var_a0 = rax;
    var_80 = rdi;
    var_120 = rsi;
    var_c0 = rbx;
    var_f8 = r12;
    loc_ADDR:
    rcx = var_90;
    loc_ADDR:
    eax = 0;
    if (r13d >= 0 /* signed */) goto loc_ADDR;
    var_e8 = rax;
    if (rcx != 0) {
        rax = *(u64*)((rcx + 8));
        var_d0 = rax;
        if (rax != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_d0);
            rcx = var_90;
            eax = 0;
            *(u64*)((rcx + 8)) = rax;
        }
        rax = *(u64*)((rcx + 0xADDR));
        var_d0 = rax;
        if (rax != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_d0);
            rcx = var_90;
            eax = 0;
            *(u64*)((rcx + 0xADDR)) = rax;
        }
        rax = *(u64*)((rcx + 0xADDR));
        var_d0 = rax;
        if (rax != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_d0);
            rax = var_90;
            ecx = 0;
            *(u64*)((rax + 0xADDR)) = rcx;
        }
        GetProcessHeap();
        HeapFree(rax, 0, var_90);
        ecx = 0;
        loc_ADDR:
        var_e8 = rcx;
    }
    r13d |= 0xADDR;
    var_d8 = r13d;
    if (r13d < 0 /* signed */) goto loc_ADDR;
    r13 = var_e8;
    if (r13 != 0) goto loc_ADDR;
    loc_ADDR:
    r13d = 0xADDR;
    loc_ADDR:
    while (1) {
        var_d8 = r13d;
        goto loc_ADDR;
        loc_ADDR:
        rax = *(u64*)((r13 + 8));
        ecx = 0;
        var_d0 = rax;
        if (rax == 0) goto loc_ADDR;
        if (*(u32*)(r13) == ecx) goto loc_ADDR;
        eax = *(u32*)(r13);
        rax -= 0xADDR;
        var_110 = rax;
        sub_140024bf8(rax);
        r9d = 0;
        var_128 = rax;
        r8 = rax;
        if (rax == 0) goto loc_ADDR;
        r10 = var_110;
        al = r9b;
        rcx = var_d0;
        rdx = r8;
        var_60 = al;
        var_888 = r15;
        var_100 = rcx;
        var_98 = rdx;
        r10d &= 7;
        if (r10d == 0) goto loc_ADDR;
        var_f0 = r9d;
        r11d = r9d;
        var_78 = r9d;
        if (r10d == 0) goto loc_ADDR;
        rdx = var_a0;
        var_d8 = 0xADDR;
        do {
            r9d = (uint32)(*(u8*)(rcx));
            ecx = var_d8;
            if (var_78 < 4) {
                ecx -= 0xADDR;
                r9d <<= cl;
                r11d |= r9d;
                goto loc_ADDR;
            }
            r9d <<= cl;
            var_f0 |= r9d;
            loc_ADDR:
            rcx = var_100;
            r9d = var_78;
            rcx += 1;
            var_d8 -= 8;
            r9d += 1;
            var_100 = rcx;
            var_78 = r9d;
        } while (r9d < r10d /* signed */);
        r8d = var_f0;
        r9d = r8d;
        var_78 = r11d;
        r9d ^= 0xADDR;
        r11d ^= 0xADDR;
        r15d = r9d;
        eax = 0;
        r14d = r11d;
        var_f0 = eax;
        eax = var_78;
        do {
            var_a0 = rdx;
            var_e8 = r13;
            var_100 = rcx;
            if (var_f0 < 4 /* signed */) {
                edx = r14d;
                ecx = r14d;
                edx >>= 0xADDR;
                r14d = edx;
                ecx <<= 8;
                r14d |= ecx;
                edx = r14d;
                goto loc_ADDR;
            }
            edx = r15d;
            ecx = r15d;
            edx >>= 0xADDR;
            r15d = edx;
            ecx <<= 8;
            r15d |= ecx;
            edx = r15d;
            loc_ADDR:
            rcx = var_98;
            r13 = (rcx + 1);
            *(u8*)(rcx) = dl;
            ecx = var_f0;
            rdx = var_a0;
            ecx += 1;
            var_f0 = ecx;
            rcx = var_100;
            var_68 = r13;
            var_98 = r13;
            r13 = var_e8;
        } while (ecx < r10d /* signed */);
        rdx = var_68;
        r14 = 0xADDR;
        var_f0 = r8d;
        r15 = 0xADDR;
        r8 = var_128;
        ecx = 0;
        var_78 = eax;
        al = var_60;
        var_80 = rdi;
        var_f8 = r12;
        var_120 = rsi;
        var_c0 = rbx;
        goto loc_ADDR;
        loc_ADDR:
        ecx = 0;
        r11d = 0xADDR;
        var_78 = ecx;
        r9d = 0xADDR;
        var_f0 = ecx;
        loc_ADDR:
        if (r10d > 4) {
            ecx = 8;
            ecx -= r10d;
            ecx <<= 3;
            r9d >>= cl;
            r9d <<= cl;
            var_d8 = r9d;
            goto loc_ADDR;
        }
        var_d8 = ecx;
        __asm("if (jae short 0xADDR /* unmodelled flags */) goto 0xADDR");
        ecx = 4;
        ecx -= r10d;
        ecx <<= 3;
        r11d >>= cl;
        r11d <<= cl;
        goto loc_ADDR;
        loc_ADDR:
        r10d = 0;
        ecx = -1;
        var_78 = r10d;
        var_f0 = ecx;
        r11d = r10d;
        var_d8 = r10d;
        loc_ADDR:
        r10 = var_110;
        rcx = (r10 >> 3);
        if (rcx == 0) goto loc_ADDR;
        esi = var_88c;
        rdx += 7;
        edi = (uint32)(var_88a);
        ebx = esi;
        r14d = var_d8;
        ebx ^= r15d;
        r13d = var_f0;
        r12 = rcx;
        var_68 = rdx;
        rdx = var_100;
        eax = (uint32)(r15w);
        rdx += 2;
        r8d = (uint32)(si);
        var_118 = eax;
        eax = var_78;
        var_a8 = r8d;
        do {
            ecx = (uint32)(*(u8*)((rdx - 1)));
            r10d = (uint32)(*(u8*)((rdx - 2)));
            r9d = (uint32)(*(u8*)((rdx + 2)));
            r10d <<= 8;
            r10d |= ecx;
            r9d <<= 8;
            ecx = (uint32)(*(u8*)(rdx));
            r10d <<= 8;
            r10d |= ecx;
            ecx = (uint32)(*(u8*)((rdx + 1)));
            r10d <<= 8;
            r10d |= ecx;
            ecx = (uint32)(*(u8*)((rdx + 3)));
            r9d |= ecx;
            r11d ^= r10d;
            ecx = (uint32)(*(u8*)((rdx + 4)));
            r9d <<= 8;
            r9d |= ecx;
            ecx = (uint32)(*(u8*)((rdx + 5)));
            rdx += 8;
            r9d <<= 8;
            var_100 = rdx;
            r9d |= ecx;
            r14d ^= r9d;
            r14d ^= r15d;
            r14d ^= r11d;
            r14d ^= esi;
            edx = r14d;
            ecx = r14d;
            edx -= r15d;
            ecx = ror(ecx, 0xADDR);
            edx = ror(edx, 0xADDR);
            edx *= r8d;
            edx += ecx;
            r11d ^= edx;
            ecx = r11d;
            ecx = ror(ecx, 0xADDR);
            edx = (r11 + rsi);
            edx = ror(edx, 9);
            edx *= edi;
            edx -= ecx;
            r14d ^= edx;
            edx = r14d;
            ecx = r14d;
            edx -= r8d;
            ecx >>= 0xADDR;
            edx *= var_118;
            edx -= ecx;
            ecx = (uint32)(var_88e);
            r11d ^= edx;
            edx = edi;
            edx ^= r11d;
            edx = ror(edx, 0xADDR);
            edx *= ecx;
            ecx = r11d;
            ecx = ror(ecx, 0xADDR);
            edx -= ecx;
            ecx = esi;
            r14d ^= edx;
            edx = r15d;
            edx ^= r14d;
            ecx -= edx;
            r11d ^= ecx;
            ecx = r8d;
            ecx ^= r11d;
            edx = r11d;
            edx = ror(edx, 6);
            ecx *= edi;
            edx ^= ecx;
            r14d ^= edx;
            ecx = r14d;
            ecx = ror(ecx, 0xADDR);
            edx = (rsi + r14);
            edx = ror(edx, 0xADDR);
            edx *= var_118;
            edx += ecx;
            ecx = (uint32)(var_88e);
            r11d ^= edx;
            edx = r11d;
            edx -= r15d;
            edx = ror(edx, 0xADDR);
            edx *= ecx;
            ecx = r11d;
            ecx = ror(ecx, 0xADDR);
            edx -= ecx;
            r14d ^= edx;
            edx = r15d;
            edx ^= r14d;
            ecx = r14d;
            edx = ror(edx, 0xADDR);
            edx *= r8d;
            ecx = ror(ecx, 0xADDR);
            edx ^= ecx;
            r11d ^= edx;
            edx = (uint32)(var_88e);
            edx ^= r11d;
            ecx = r11d;
            edx *= edi;
            ecx >>= 0xADDR;
            edx ^= ecx;
            r14d ^= edx;
            edx = (uint32)(var_88e);
            ecx = r14d;
            ecx = ~(ecx);
            ecx = ror(ecx, 5);
            ecx += var_118;
            ecx *= edx;
            r11d ^= ecx;
            ecx = r11d;
            ecx -= edx;
            ecx ^= r15d;
            r14d ^= ecx;
            ecx = (uint32)(var_88e);
            edx = r14d;
            edx ^= ecx;
            ecx = r14d;
            edx = ror(edx, 0xADDR);
            edx *= r8d;
            ecx >>= 2;
            edx += ecx;
            r11d ^= edx;
            edx = r11d;
            ecx = r11d;
            edx -= esi;
            ecx = ror(ecx, 0xADDR);
            edx = ror(edx, 6);
            edx *= edi;
            edx += ecx;
            r14d ^= edx;
            ecx = r14d;
            edx = r14d;
            edx = ror(edx, 9);
            ecx ^= r8d;
            ecx *= var_118;
            edx += ecx;
            ecx = (uint32)(var_88e);
            r11d ^= edx;
            edx = r11d;
            edx ^= edi;
            edx = ror(edx, 0xADDR);
            edx *= ecx;
            ecx = r11d;
            ecx = ror(ecx, 0xADDR);
            edx += ecx;
            r14d ^= edx;
            edi = r14d;
            edi ^= r11d;
            r11d = (uint32)(var_88a);
            edi ^= ebx;
            ecx = edi;
            ecx = ror(ecx, 3);
            ecx -= r11d;
            ecx *= r8d;
            r14d ^= ecx;
            edx = r14d;
            ecx = r14d;
            edx -= esi;
            ecx = ror(ecx, 6);
            edx = ror(edx, 1);
            edx *= var_118;
            edx -= ecx;
            edi ^= edx;
            ecx = (uint32)(var_88e);
            edx = (r15 + rdi);
            edx = ror(edx, 0xADDR);
            edx *= ecx;
            ecx = edi;
            ecx = ror(ecx, 0xADDR);
            edx += ecx;
            r14d ^= edx;
            ecx = r14d;
            ecx = ror(ecx, 0xADDR);
            edx = (r15 + r14);
            edx = ror(edx, 0xADDR);
            edx *= r8d;
            edx -= ecx;
            edi ^= edx;
            edx = edi;
            ecx = edi;
            edx ^= var_118;
            edx *= r11d;
            r11d = esi;
            ecx >>= 3;
            edx ^= ecx;
            r14d ^= edx;
            r11d ^= r14d;
            ecx = r14d;
            ecx = ror(ecx, 0xADDR);
            r14d ^= r13d;
            r11d = ror(r11d, 0xADDR);
            r13d = r9d;
            r11d *= var_118;
            r9d = 1;
            r11d ^= ecx;
            r11d ^= edi;
            edi = r14d;
            r11d ^= eax;
            edi <<= 0xADDR;
            rax = var_68;
            edx = r11d;
            var_68 += 8;
            ecx = (r11d >> 8);
            edx <<= 0xADDR;
            edx |= ecx;
            *(u8*)((rax - 4)) = r11b;
            *(u8*)(rax) = r14b;
            r8d = edx;
            *(u8*)((rax - 5)) = dl;
            ecx = (r14d >> 8);
            edi |= ecx;
            r8d <<= 0xADDR;
            *(u8*)((rax - 1)) = dil;
            ecx = edi;
            edx >>= 8;
            r8d |= edx;
            ecx <<= 0xADDR;
            rdx = var_100;
            *(u8*)((rax - 6)) = r8b;
            edi >>= 8;
            ecx |= edi;
            r8d = ror(r8d, 8);
            edi = (uint32)(var_88a);
            *(u8*)((rax - 2)) = cl;
            *(u8*)((rax - 7)) = r8b;
            r8d = var_a8;
            ecx = ror(ecx, 8);
            *(u8*)((rax - 3)) = cl;
            eax = r10d;
            r12 -= r9;
        } while (r12 != 0);
        al = var_60;
        r14 = 0xADDR;
        rbx = var_c0;
        rsi = var_120;
        r12 = var_f8;
        rdi = var_80;
        r13 = var_e8;
        r8 = var_128;
        r10 = var_110;
        goto loc_ADDR;
        loc_ADDR:
        r9d = 1;
        loc_ADDR:
        edx = 0;
        ecx = edx;
        if (r10 != 0) {
            do {
                al ^= *(u8*)((rcx + r8));
                rcx += r9;
            } while (rcx < r10);
        }
        rcx = var_a0;
        rdx = r13;
        r9 = rcx;
        var_a0 = rcx;
        ecx = (uint32)(al);
        rax = var_d0;
        var_e8 = r13;
        if (rcx != *(u64*)((r10 + rax))) {
            sub_140024e2c(r8);
            loc_ADDR:
            var_128 = r13;
            r13d = 0xADDR;
            goto loc_ADDR;
        }
        var_e8 = rdx;
        rcx = r9;
        var_a0 = rcx;
        if (r10d >= 4) goto loc_ADDR;
        do {
            r13d = 0xADDR;
            goto loc_ADDR;
            loc_ADDR:
            r11d = *(u32*)(r8);
            var_f0 = r11d;
            r11 = (r8 + 4);
            var_a0 = rcx;
            var_e8 = rdx;
            var_128 = r8;
            if (r11 < r8) goto loc_ADDR;
            eax = (r10 - 4);
            if (eax < 4) {
                r13d = 0xADDR;
                var_128 = r8;
                var_e8 = rdx;
                var_a0 = r9;
                goto loc_ADDR;
            }
            r13d = *(u32*)(r11);
            r9 = (r11 + 4);
            var_78 = r13d;
            if (r9 < r11) goto loc_ADDR;
            eax = (r10 - 8);
            var_d0 = rcx;
        } while (eax < r13d);
        eax = (r13 + 8);
        if (eax < 8) goto loc_ADDR;
        ecx = r10d;
        r10 = (r9 + r13);
        rax = (r8 + rcx);
        if (rax < r10) goto loc_ADDR;
        rax = r8;
        rax -= r13;
        rax -= r9;
        rax += rcx;
        if (rax >= 8) goto loc_ADDR;
        eax = 0;
        var_c8 = r9;
        var_d8 = eax;
        r13d = eax;
        rax = var_d0;
        var_a0 = rax;
        rcx = rax;
        var_128 = r8;
        var_e8 = rdx;
        var_80 = rdi;
        if (r9 == 0) goto loc_ADDR;
        var_128 = r8;
        var_e8 = rdx;
        var_a0 = rax;
        if (r10 < r9) goto loc_ADDR;
        ecx = 0;
        var_c8 = r9;
        var_80 = rdi;
        rcx = rax;
        if (r9 < r10) {
            rax = r9;
            var_128 = r8;
            var_e8 = rdx;
            var_a0 = rcx;
            do {
                var_c8 = rax;
                rax = (r9 + 4);
                if (rax < r9) goto loc_ADDR;
                r13 = (r9 + 4);
                var_a0 = rcx;
                var_e8 = rdx;
                rax = rbx;
                var_128 = r8;
                if (r13 > r10) goto loc_ADDR;
                eax = (*(u32*)(r9) + 4);
                if (eax < 4) goto loc_ADDR;
                r11d = eax;
                r11 += r9;
                if (r11 < r9) goto loc_ADDR;
                r9 = r11;
                r11d = 0;
                r13d = r11d;
                if (r9 > r10) goto loc_ADDR;
                eax = (r11 + 1);
                var_d8 += eax;
                rax = var_c8;
            } while (r9 < r10);
            var_128 = r8;
            var_e8 = rdx;
            var_80 = rdi;
            var_a0 = rcx;
        }
        __asm("if (je short 0xADDR /* unmodelled flags */) goto 0xADDR");
        r13d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        r13d = 0xADDR;
        while (1) {
            var_e8 = rdx;
            var_128 = r8;
            var_a0 = rcx;
            goto loc_ADDR;
            loc_ADDR:
            var_128 = r8;
            var_e8 = rdx;
            var_a0 = rcx;
            loc_ADDR:
            r13d = 0xADDR;
            goto loc_ADDR;
            loc_ADDR:
            r13d = 0xADDR;
            goto loc_ADDR;
            loc_ADDR:
            var_d0 = rcx;
            eax = 0;
            ecx = var_78;
            var_68 = rdi;
            var_90 = rdx;
            var_c0 = r8;
            if (ecx != 0) {
                edi = ecx;
                GetProcessHeap();
                HeapAlloc(rax, 8, edi);
                r13d = 0;
                rdi = rax;
                if (rax != 0) {
                    ecx = var_78;
                    goto loc_ADDR;
                }
                rdi = var_80;
                goto loc_ADDR;
            }
            rdi = rax;
            loc_ADDR:
            rax = var_c8;
            if (rax != 0) {
                r8d = ecx;
                rdx = rax;
                rcx = rdi;
                sub_1400274ac(rdi, rax, ecx);
            }
            ecx = var_d8;
            var_300 = ecx;
            var_308 = rdi;
            rax = var_c0;
            r10 = rsi;
            r11 = var_68;
            r9 = rbx;
            rdx = var_d0;
            rdi = r11;
            var_128 = rax;
            r8 = r12;
            rax = var_90;
            var_e8 = rax;
            var_a0 = rdx;
            if (r13d < 0 /* signed */) goto loc_ADDR;
            var_128 = var_c0;
            rax = var_90;
            var_e8 = rax;
            var_a0 = rdx;
            if (var_f0 == ecx) goto loc_ADDR;
            rax = var_c0;
            r13d = 0xADDR;
            var_128 = rax;
            rax = var_90;
            var_e8 = rax;
            var_a0 = rdx;
            goto loc_ADDR;
            loc_ADDR:
            rcx = var_d0;
            r13d = 0xADDR;
        }
        loc_ADDR:
        r13d = 0xADDR;
        loc_ADDR:
        var_a0 = rcx;
        var_e8 = rdx;
        var_128 = r8;
        goto loc_ADDR;
        loc_ADDR:
        r13d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        r13d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        r13d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        r13d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        r13d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        r13d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        r13d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        r13d = 0;
        var_a0 = r13;
        loc_ADDR:
        r13d = 0xADDR;
        loc_ADDR:
        r13d |= 0xADDR;
    }
    loc_ADDR:
    r13d = 0xADDR;
    var_d8 = r13d;
    goto loc_ADDR;
    r13d = 0;
    if (var_308 == r13) goto loc_ADDR;
    rax = var_308;
    r9d = r13d;
    do {
        rdi = (rax + 4);
        rbx = r8;
        rsi = rdx;
        if (rdi < rax) goto loc_ADDR;
        eax = *(u32*)(rax);
        rax += rdi;
        if (rax < rdi) goto loc_ADDR;
        r9d += r11d;
    } while (r9d < r11d);
    rdi = (rax + 4);
    if (rdi < rax) goto loc_ADDR;
    rdx = r13;
    rdx = ((*(u32*)(rax) != r13d) ? rdi : rdx);
    if (*(u32*)(rax) != 8) goto loc_ADDR;
    rdx = *(u64*)(rdx);
    if (var_308 == r13) goto loc_ADDR;
    rax = var_308;
    r10d = r13d;
    rdi = rsi;
    do {
        r9 = (rax + 4);
        rbx = r8;
        rsi = rdi;
        if (r9 < rax) goto loc_ADDR;
        eax = *(u32*)(rax);
        rax += r9;
        if (rax < r9) goto loc_ADDR;
        r10d += r11d;
    } while (r10d < 2);
    rdi = (rax + 4);
    if (rdi < rax) goto loc_ADDR;
    r10 = r13;
    r10 = ((*(u32*)(rax) != r13d) ? rdi : r10);
    if (*(u32*)(rax) != 4) goto loc_ADDR;
    r10d = *(u32*)(r10);
    if (var_308 == r13) goto loc_ADDR;
    rax = var_308;
    r11d = r13d;
    r8 = rsi;
    r9 = rbx;
    do {
        rdi = (rax + 4);
        rbx = r9;
        rsi = r8;
        if (rdi < rax) goto loc_ADDR;
        eax = *(u32*)(rax);
        rax += rdi;
        if (rax < rdi) goto loc_ADDR;
        r11d += 1;
    } while (r11d < 3);
    r8d = *(u32*)(rax);
    rdi = (rax + 4);
    if (rdi < rax) goto loc_ADDR;
    r9 = r13;
    r9 = ((r8d != 0) ? rdi : r9);
    if (var_308 == r13) goto loc_ADDR;
    rax = var_308;
    rdi = rsi;
    var_f0 = r13d;
    r11 = rbx;
    do {
        r13 = (rax + 4);
        rbx = r11;
        rsi = rdi;
        if (r13 < rax) goto loc_ADDR;
        eax = *(u32*)(rax);
        rax += r13;
        if (rax < r13) goto loc_ADDR;
        r13d = (var_f0 + 1);
        var_f0 = r13d;
    } while (r13d < 4);
    r11d = *(u32*)(rax);
    rdi = (rax + 4);
    if (rdi < rax) goto loc_ADDR;
    r13d = 0;
    eax = r13d;
    rax = ((r11d != 0) ? rdi : rax);
    if (r11d != 4) goto loc_ADDR;
    r13d = *(u32*)(rax);
    edi = 0;
    if (var_308 == rdi) goto loc_ADDR;
    rax = var_308;
    r11 = rbx;
    var_f0 = edi;
    rdi = rsi;
    do {
        var_78 = r13d;
        rbx = r11;
        r13 = (rax + 4);
        rsi = rdi;
        if (r13 < rax) goto loc_ADDR;
        eax = *(u32*)(rax);
        rax += r13;
        if (rax < r13) goto loc_ADDR;
        r13d = (var_f0 + 1);
        var_f0 = r13d;
        r13d = var_78;
    } while (r13d < 5);
    r11 = (rax + 4);
    if (r11 < rax) goto loc_ADDR;
    r13d = 0;
    edi = r13d;
    rdi = ((*(u32*)(rax) != r13d) ? r11 : rdi);
    if (*(u32*)(rax) != 4) goto loc_ADDR;
    edi = *(u32*)(rdi);
    if (var_b8 != rdx) {
        loc_ADDR:
        r13d = 0xADDR;
        goto loc_ADDR;
    }
    var_188 = r10d;
    if (var_78 > 4) goto loc_ADDR;
    if (r8d > 4) {
        loc_ADDR:
        r13d = 0xADDR;
        goto loc_ADDR;
    }
    rdx = r9;
    sub_1400274ac(var_148);
    goto loc_ADDR;
    do {
        rdi = off_1400360c0;
        do {
            rax += 1;
        } while (*(u16*)((rbx + (rax * 2))) != r8w);
        rbx += (rax * 2);
        eax = var_a8;
        rcx = (rax + (rax * 2));
        rdi += (rcx * 8);
        GetModuleHandleExW(0, rdx, rdi);
        r8d = 0;
        if (eax == 0) goto loc_ADDR;
        rax = *(u64*)(rdi);
        ecx = 0xADDR;
        r12d = r8d;
        if (*(u16*)(rax) == cx) goto loc_ADDR;
        loc_ADDR:
        do {
            r12d = 0xADDR;
            goto loc_ADDR;
            loc_ADDR:
            rcx = (int64)(*(u32*)((rax + 0xADDR)));
            if (ecx >= 0xADDR) goto loc_ADDR;
            rcx += *(u64*)(rdi);
            if (rcx < *(u64*)(rdi)) goto loc_ADDR;
        } while (*(u32*)(rcx) != 0xADDR);
        eax = (uint32)(*(u16*)((rcx + 0xADDR)));
        ax -= dx;
        if ((0xADDR & ax) != 0) {
            r12d = 0xADDR;
            goto loc_ADDR;
        }
        rax = *(u64*)((rcx + 0xADDR));
        *(u64*)((rdi + 0xADDR)) = rax;
        eax = *(u32*)((rcx + 0xADDR));
        *(u32*)((rdi + 8)) = eax;
        loc_ADDR:
        eax = *(u32*)((rbx + 2));
        edi = r8d;
        rbx += 6;
        var_108 = eax;
        if (eax != 0) {
            eax = var_a8;
            rcx = (rax + (rax * 2));
            var_68 = rcx;
            do {
                rdx = rbx;
                rax = -1;
                do {
                    rax += 1;
                } while (*(u8*)((rbx + rax)) != r8b);
                rbx += 1;
                rbx += rax;
                if (r12d >= 0 /* signed */) {
                    rax = off_1400360c0;
                    rcx = *(u64*)((rax + (rcx * 8)));
                    GetProcAddress(*(u64*)((rax + (rcx * 8))), rdx);
                    r8d = 0;
                    if (rax == 0) goto loc_ADDR;
                    ecx = var_78;
                    rdx = off_140034000;
                    *(u64*)((rdx + (rcx * 8))) = rax;
                    rcx = var_68;
                }
                var_78 += 1;
                edi += 1;
            } while (edi < var_108);
            goto loc_ADDR;
        }
        loc_ADDR:
        var_a8 += 1;
    } while (*(u8*)(rbx) != r8b);
    goto loc_ADDR;
    loc_ADDR:
    r12d = 0xADDR;
    loc_ADDR:
    rdi = var_230;
    if (rdi == 0) goto loc_ADDR;
    goto loc_ADDR;
    do {
        rdi = off_1400360c0;
        do {
            rax += 1;
        } while (*(u16*)((rbx + (rax * 2))) != r12w);
        rbx += (rax * 2);
        eax = var_148;
        rcx = (rax + (rax * 2));
        r12 = (rdi + (rcx * 8));
        GetModuleHandleExW(0, rdx, r12);
        r8d = 0;
        if (eax == 0) goto loc_ADDR;
        rax = *(u64*)(r12);
        ecx = 0xADDR;
        edi = r8d;
        if (*(u16*)(rax) == cx) goto loc_ADDR;
        loc_ADDR:
        do {
            edi = 0xADDR;
            goto loc_ADDR;
            loc_ADDR:
            rcx = (int64)(*(u32*)((rax + 0xADDR)));
            if (ecx >= 0xADDR) goto loc_ADDR;
            rcx += *(u64*)(r12);
            if (rcx < *(u64*)(r12)) goto loc_ADDR;
        } while (*(u32*)(rcx) != 0xADDR);
        eax = (uint32)(*(u16*)((rcx + 0xADDR)));
        ax -= dx;
        if ((0xADDR & ax) != 0) {
            edi = 0xADDR;
            goto loc_ADDR;
        }
        rax = *(u64*)((rcx + 0xADDR));
        *(u64*)((r12 + 0xADDR)) = rax;
        eax = *(u32*)((rcx + 0xADDR));
        *(u32*)((r12 + 8)) = eax;
        loc_ADDR:
        eax = *(u32*)((rbx + 2));
        r12d = r8d;
        rbx += 6;
        var_120 = eax;
        if (eax != 0) {
            eax = var_148;
            rcx = (rax + (rax * 2));
            var_80 = rcx;
            do {
                rdx = rbx;
                rax = -1;
                do {
                    rax += 1;
                } while (*(u8*)((rbx + rax)) != r8b);
                rbx += 1;
                rbx += rax;
                if (edi >= 0 /* signed */) {
                    rax = off_1400360c0;
                    rcx = *(u64*)((rax + (rcx * 8)));
                    GetProcAddress(*(u64*)((rax + (rcx * 8))), rdx);
                    r8d = 0;
                    if (rax == 0) goto loc_ADDR;
                    ecx = var_f8;
                    rdx = off_140034000;
                    *(u64*)((rdx + (rcx * 8))) = rax;
                    rcx = var_80;
                }
                var_f8 += 1;
                r12d += 1;
            } while (r12d < var_120);
            goto loc_ADDR;
        }
        eax = 1;
        loc_ADDR:
        var_148 += 1;
        r12d = 0;
    } while (*(u8*)(rbx) != r12b);
    goto loc_ADDR;
    loc_ADDR:
    edi = 0xADDR;
    loc_ADDR:
    r12d = 0;
    loc_ADDR:
    if (rbx == 0) goto loc_ADDR;
    goto loc_ADDR;
    r12d = ecx;
    var_c0 = ecx;
    GetLastError();
    r12d = eax;
    if (eax > 0 /* signed */) {
        r12d = (uint32)(ax);
        r12d |= 0xADDR;
    }
    if (r12d < 0 /* signed */) goto loc_ADDR;
    r12d = 0xADDR;
    goto loc_ADDR;
    GetLastError();
    r12d = eax;
    if (eax > 0 /* signed */) {
        r12d = (uint32)(ax);
        r12d |= 0xADDR;
    }
    eax = 0xADDR;
    r12d = ((r12d >= 0 /* signed */) ? eax : r12d);
    rax = var_238;
    var_68 = rdi;
    edi = 0;
    var_88 = rbx;
    var_90 = rcx;
    do {
        r8d = 0;
        do {
            ecx = (uint32)(*(u8*)((r9 - 1)));
            edx = (rax + (rcx * 2));
            eax = (uint32)(*(u8*)(r9));
            edx += eax;
            edx >>= 2;
            dl = ~(dl);
            if (dl != 0) {
                edi = (uint32)(*(u8*)((r9 + r10)));
                eax = r12d;
                r8d = (uint32)(dl);
                ecx = (uint32)(r15b);
                ecx -= edi;
                ecx *= r8d;
                __asm("imul ecx");
                edx += ecx;
                ecx = (uint32)(var_f9);
                edx >>= 7;
                eax = (edx >> 0xADDR);
                edx += eax;
                eax = r12d;
                dl += dil;
                *(u8*)((r9 + r10)) = dl;
                edi = (uint32)(*(u8*)(((r10 + r9) - 1)));
                ecx -= edi;
                ecx *= r8d;
                __asm("imul ecx");
                edx += ecx;
                ecx = (uint32)(var_fa);
                edx >>= 7;
                eax = (edx >> 0xADDR);
                edx += eax;
                eax = r12d;
                dl += dil;
                *(u8*)(((r10 + r9) - 1)) = dl;
                edi = (uint32)(*(u8*)(((r10 + r9) - 2)));
                ecx -= edi;
                ecx *= r8d;
                __asm("imul ecx");
                edx += ecx;
                ecx = 0xADDR;
                edx >>= 7;
                eax = (edx >> 0xADDR);
                edx += eax;
                eax = r12d;
                dl += dil;
                *(u8*)(((r10 + r9) - 2)) = dl;
                edi = (uint32)(*(u8*)(((r10 + r9) + 1)));
                ecx -= edi;
                ecx *= r8d;
                r8d = var_108;
                __asm("imul ecx");
                edx += ecx;
                edx >>= 7;
                eax = (edx >> 0xADDR);
                edx += eax;
                dl += dil;
                *(u8*)(((r10 + r9) + 1)) = dl;
            }
            r8d += r13d;
            r9 += 4;
            var_108 = r8d;
            edi = esi;
        } while (r8d < r14d /* signed */);
        rdx = var_90;
        rax = var_88;
        ecx = var_78;
        rdx += var_a0;
        ecx += 1;
        rax += var_e0;
        var_90 = rdx;
        var_88 = rax;
        var_78 = ecx;
    } while (ecx < r11d /* signed */);
    esi = var_f0;
    r14 = 0xADDR;
    r13d = var_d8;
    rbx = var_80;
    r12d = 0xADDR;
    edi = 1;
    rax += 8;
    var_90 = rax;
    sub_140024bf8(rax);
    r10d = 0;
    var_138 = rax;
    r8 = rax;
    if (rax == 0) goto loc_ADDR;
    eax = var_80;
    edx = r10d;
    var_60 = r10b;
    ecx = eax;
    if (eax != 0) {
        al = r10b;
        r9d = (r10 + 1);
        do {
            al ^= *(u8*)((rdx + r12));
            rdx += r9;
        } while (rdx < rcx);
        var_60 = al;
    }
    eax = 0;
    var_e8 = r8;
    r9 = r8;
    var_880 = r14;
    r8 = rcx;
    var_a0 = eax;
    r11d = -1;
    var_e0 = eax;
    rdx = r12;
    r8d &= 7;
    if (r8d == 0) goto loc_ADDR;
    ecx = r10d;
    r9d = r10d;
    var_a0 = ecx;
    if (r8d == 0) goto loc_ADDR;
    r11d = (rax + 0xADDR);
    do {
        r10d = (uint32)(*(u8*)(rdx));
        if (ecx < 4) {
            ecx = (r11 - 0xADDR);
            r10d <<= cl;
            r9d |= r10d;
            goto loc_ADDR;
        }
        ecx = r11d;
        r10d <<= cl;
        eax |= r10d;
        loc_ADDR:
        ecx = var_a0;
        rdx += 1;
        ecx += 1;
        r11d -= 8;
        var_a0 = ecx;
    } while (ecx < r8d /* signed */);
    rdi = var_e8;
    r10d = (r9d ^ 0xADDR);
    r11d = (eax ^ 0xADDR);
    r15d = r10d;
    ecx = 0;
    r14d = r11d;
    var_e0 = ecx;
    r13d = (rcx + 1);
    do {
        var_a0 = eax;
        if (ecx < 4 /* signed */) {
            ecx = r15d;
            eax = r15d;
            ecx >>= 0xADDR;
            eax <<= 8;
            r15d = ecx;
            r15d |= eax;
            eax = r15d;
            goto loc_ADDR;
        }
        ecx = r14d;
        eax = r14d;
        ecx >>= 0xADDR;
        eax <<= 8;
        r14d = ecx;
        r14d |= eax;
        eax = r14d;
        loc_ADDR:
        rcx = (rdi + 1);
        *(u8*)(rdi) = al;
        eax = var_a0;
        var_e8 = rcx;
        ecx = var_e0;
        rdi = var_e8;
        ecx += r13d;
        var_e0 = ecx;
    } while (ecx < r8d /* signed */);
    r13d = var_d8;
    r14 = 0xADDR;
    var_e0 = r9d;
    eax = 0;
    r9 = rdi;
    var_88 = r12;
    rdi = var_b0;
    var_68 = rsi;
    var_98 = rbx;
    goto loc_ADDR;
    loc_ADDR:
    r9 = var_e8;
    r10d = 0xADDR;
    r11d = 0xADDR;
    var_e0 = eax;
    var_a0 = eax;
    loc_ADDR:
    if (r8d > 4) {
        ecx = 8;
        ecx -= r8d;
        ecx <<= 3;
        r11d >>= cl;
        r11d <<= cl;
        goto loc_ADDR;
    }
    r11d = eax;
    __asm("if (jae short 0xADDR /* unmodelled flags */) goto 0xADDR");
    ecx = 4;
    ecx -= r8d;
    ecx <<= 3;
    r10d >>= cl;
    r10d <<= cl;
    loc_ADDR:
    eax = var_80;
    rax >>= 3;
    var_120 = rax;
    if (rax == 0) goto loc_ADDR;
    eax = (uint32)(var_882);
    rcx = (rdx + 2);
    edi = var_884;
    ebx = r14d;
    r15d = var_e0;
    ebx ^= edi;
    r12d = var_a0;
    r9 += 7;
    r13 = var_120;
    var_e8 = eax;
    var_128 = (uint32)(di);
    eax = (uint32)(var_886);
    var_c8 = eax;
    esi = (uint32)(r14w);
    var_f8 = r9;
    do {
        eax = (uint32)(*(u8*)((rcx - 1)));
        r9d = (uint32)(*(u8*)((rcx - 2)));
        r8d = (uint32)(*(u8*)((rcx + 2)));
        edx = var_128;
        r9d <<= 8;
        r9d |= eax;
        r8d <<= 8;
        eax = (uint32)(*(u8*)(rcx));
        r9d <<= 8;
        r9d |= eax;
        eax = (uint32)(*(u8*)((rcx + 1)));
        r9d <<= 8;
        r9d |= eax;
        eax = (uint32)(*(u8*)((rcx + 3)));
        r8d |= eax;
        eax = (uint32)(*(u8*)((rcx + 4)));
        r8d <<= 8;
        r8d |= eax;
        eax = (uint32)(*(u8*)((rcx + 5)));
        rcx += 8;
        r8d <<= 8;
        r8d |= eax;
        var_a0 = rcx;
        r11d ^= r8d;
        eax = r11d;
        eax -= esi;
        eax ^= edi;
        eax ^= r9d;
        r10d ^= eax;
        eax = (uint32)(var_882);
        ecx = r10d;
        ecx ^= edi;
        ecx = ror(ecx, 0xADDR);
        ecx *= eax;
        eax = r10d;
        eax = ror(eax, 7);
        ecx += eax;
        r11d ^= ecx;
        eax = r11d;
        eax = ror(eax, 0xADDR);
        ecx = (r14 + r11);
        ecx = ror(ecx, 9);
        ecx *= edx;
        ecx -= eax;
        eax = (uint32)(var_886);
        r10d ^= ecx;
        ecx = r10d;
        ecx ^= edx;
        ecx = ror(ecx, 0xADDR);
        ecx *= eax;
        eax = r10d;
        eax = ror(eax, 0xADDR);
        ecx += eax;
        eax = edi;
        r11d ^= ecx;
        ecx = r14d;
        ecx ^= r11d;
        eax -= ecx;
        r10d ^= eax;
        eax = (uint32)(var_882);
        ecx = r10d;
        ecx -= esi;
        ecx *= eax;
        eax = (r10d >> 6);
        ecx -= eax;
        r11d ^= ecx;
        ecx = (uint32)(var_886);
        eax = r11d;
        eax = ror(eax, 0xADDR);
        eax ^= edx;
        eax *= esi;
        r10d ^= eax;
        eax = r10d;
        eax = ~(eax);
        eax = ror(eax, 3);
        eax += ecx;
        eax *= edx;
        r11d ^= eax;
        eax = r11d;
        eax -= esi;
        eax -= edi;
        r10d ^= eax;
        eax = r10d;
        ecx = r10d;
        eax ^= var_c8;
        eax *= var_e8;
        ecx = ror(ecx, 0xADDR);
        ecx ^= eax;
        r11d ^= ecx;
        ecx = esi;
        ecx ^= r11d;
        eax = r11d;
        ecx = ror(ecx, 0xADDR);
        ecx *= edx;
        eax = ror(eax, 3);
        ecx ^= eax;
        r10d ^= ecx;
        ecx = (uint32)(var_886);
        eax = r10d;
        eax = ror(eax, 0xADDR);
        eax -= ecx;
        eax *= esi;
        r11d ^= eax;
        eax = (uint32)(var_886);
        ecx = r11d;
        ecx ^= eax;
        eax = r11d;
        ecx *= esi;
        eax >>= 1;
        ecx ^= eax;
        eax = r11d;
        r10d ^= ecx;
        eax -= edx;
        ecx = eax;
        eax <<= 3;
        ecx >>= 0xADDR;
        ecx |= eax;
        eax = r11d;
        ecx *= esi;
        eax >>= 0xADDR;
        ecx ^= eax;
        eax = (uint32)(var_882);
        r10d ^= ecx;
        ecx = r10d;
        ecx -= edx;
        ecx *= eax;
        eax = (r10d >> 0xADDR);
        ecx -= eax;
        r11d ^= ecx;
        ecx = r14d;
        ecx -= r11d;
        eax = r11d;
        eax = ror(eax, 0xADDR);
        ecx = ror(ecx, 9);
        ecx *= edx;
        ecx ^= eax;
        r10d ^= ecx;
        ecx = (uint32)(var_886);
        eax = r10d;
        eax -= r14d;
        eax -= ecx;
        r11d ^= eax;
        eax = r11d;
        ecx = r11d;
        eax ^= var_e8;
        ecx = ror(ecx, 7);
        eax *= esi;
        eax -= ecx;
        r10d ^= eax;
        eax = (uint32)(var_886);
        ecx = r10d;
        ecx ^= eax;
        eax = (uint32)(var_882);
        ecx = ror(ecx, 0xADDR);
        ecx *= eax;
        eax = r10d;
        eax = ror(eax, 0xADDR);
        ecx -= eax;
        r11d ^= ecx;
        ecx = r14d;
        ecx -= r11d;
        eax = r11d;
        ecx = ror(ecx, 0xADDR);
        ecx *= edx;
        eax = ror(eax, 4);
        ecx += eax;
        eax = (uint32)(var_886);
        r10d ^= ecx;
        ecx = r10d;
        ecx -= r14d;
        ecx = ror(ecx, 4);
        ecx *= eax;
        eax = r10d;
        eax = ror(eax, 9);
        ecx ^= eax;
        r11d ^= ecx;
        ecx = edi;
        ecx ^= r11d;
        eax = r11d;
        ecx = ror(ecx, 0xADDR);
        ecx *= esi;
        eax = ror(eax, 0xADDR);
        ecx -= eax;
        r10d ^= ecx;
        ecx = edi;
        edi = (uint32)(var_882);
        ecx -= r10d;
        ecx = ror(ecx, 0xADDR);
        eax = r10d;
        ecx *= edi;
        eax = ror(eax, 0xADDR);
        ecx -= eax;
        r11d ^= ecx;
        ecx = r11d;
        eax = r11d;
        ecx ^= edi;
        eax >>= 8;
        ecx *= edx;
        ecx ^= eax;
        eax = ebx;
        r10d ^= ecx;
        eax ^= r10d;
        r10d ^= r15d;
        r15 = var_f8;
        r11d ^= eax;
        var_f8 += 8;
        r11d ^= r12d;
        ecx = r10d;
        eax = (r10d >> 8);
        edx = r11d;
        *(u8*)((r15 - 4)) = r10b;
        r12d = r8d;
        *(u8*)(r15) = r11b;
        ecx <<= 0xADDR;
        ecx |= eax;
        edx <<= 0xADDR;
        *(u8*)((r15 - 5)) = cl;
        edi = (ecx << 0xADDR);
        eax = (r11d >> 8);
        edx |= eax;
        ecx >>= 8;
        *(u8*)((r15 - 1)) = dl;
        edi |= ecx;
        rcx = var_a0;
        eax = edx;
        *(u8*)((r15 - 6)) = dil;
        eax <<= 0xADDR;
        edi = ror(edi, 8);
        edx >>= 8;
        eax |= edx;
        *(u8*)((r15 - 2)) = al;
        *(u8*)((r15 - 7)) = dil;
        edi = var_884;
        eax = ror(eax, 8);
        *(u8*)((r15 - 3)) = al;
        r15d = r9d;
        r13 -= 1;
    } while (r13 != 0);
    rbx = var_98;
    rsi = var_68;
    r13d = var_d8;
    r12 = var_88;
    rdi = var_b0;
    loc_ADDR:
    r8d = var_80;
    rdx = var_138;
    eax = (uint32)(var_60);
    *(u64*)((rdx + r8)) = rax;
    GetProcessHeap();
    edx = 8;
    HeapAlloc(rax, 8, (rdx + 0xADDR));
    var_b0 = rax;
    r14 = rax;
    if (rax == 0) goto loc_ADDR;
    rax = var_90;
    *(u32*)(r14) = eax;
    GetProcessHeap();
    r8 = var_90;
    HeapAlloc(rax, 8, var_90);
    if (rax == 0) goto loc_ADDR;
    *(u64*)((r14 + 8)) = rax;
    r8 = var_90;
    sub_1400274ac(rax, var_138);
    var_b0 = r14;
    var_68 = rbx;
    *(u32*)((r14 + 0xADDR)) = 0xADDR;
    GetProcessHeap();
    HeapAlloc(rax, 8, 0xADDR);
    if (rax == 0) goto loc_ADDR;
    *(u64*)((r14 + 0xADDR)) = rax;
    rcx = rax;
    edx = 0xADDR;
    rax = (0xADDR + 0xADDR);
    *(__m128*)(rcx) = *(__m128*)(rax);
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)((rcx + 0xADDR)) = *(__m128*)((rax + 0xADDR));
    *(__m128*)(((rcx + rdx) - 0xADDR)) = *(__m128*)((rax + 0xADDR));
    xmm0 = *(__m128*)((rax + rdx));
    *(__m128*)((rcx + rdx)) = xmm0;
    xmm1 = *(__m128*)(((rax + rdx) + 0xADDR));
    *(__m128*)(((rcx + rdx) + 0xADDR)) = xmm1;
    *(u32*)((r14 + 0xADDR)) = 8;
    GetProcessHeap();
    rcx = rax;
    eax = 8;
    HeapAlloc(rax, eax, eax);
    ecx = 0;
    if (rax == 0) {
        var_b0 = r14;
        rbx = var_68;
        loc_ADDR:
        rax = var_b0;
        r14d = 0xADDR;
        rcx = *(u64*)((rax + 8));
        var_98 = rcx;
        if (rcx != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_98);
            rax = var_b0;
            ecx = 0;
            *(u64*)((rax + 8)) = rcx;
        }
        rcx = *(u64*)((rax + 0xADDR));
        var_98 = rcx;
        if (rcx != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_98);
            rax = var_b0;
            ecx = 0;
            *(u64*)((rax + 0xADDR)) = rcx;
        }
        rax = *(u64*)((rax + 0xADDR));
        var_98 = rax;
        if (rax != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_98);
            rax = var_b0;
            ecx = 0;
            *(u64*)((rax + 0xADDR)) = rcx;
        }
        GetProcessHeap();
        HeapFree(rax, 0, var_b0);
        goto loc_ADDR;
    }
    *(u64*)((r14 + 0xADDR)) = rax;
    rdx = var_b0;
    r14d = ecx;
    rax = qword_ADDR;
    rcx = *(u64*)((rdx + 0xADDR));
    *(u64*)(rcx) = rax;
    rax = rdx;
    goto loc_ADDR;
    loc_ADDR:
    r14d = 0xADDR;
    loc_ADDR:
    if (r14d >= 0 /* signed */) {
        rax = var_70;
        loc_ADDR:
        rdi = rax;
        eax = 0;
        var_70 = rax;
    }
    GetProcessHeap();
    HeapFree(rax, 0, var_138);
    rax = var_70;
    if (rax != 0) {
        rcx = *(u64*)((rax + 8));
        var_b0 = rcx;
        if (rcx != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_b0);
            rax = var_70;
            edx = 0;
            *(u64*)((rax + 8)) = rdx;
        }
        rcx = *(u64*)((rax + 0xADDR));
        var_b0 = rcx;
        if (rcx != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_b0);
            rax = var_70;
            edx = 0;
            *(u64*)((rax + 0xADDR)) = rdx;
        }
        rax = *(u64*)((rax + 0xADDR));
        var_b0 = rax;
        if (rax != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, var_b0);
            rax = var_70;
            ecx = 0;
            *(u64*)((rax + 0xADDR)) = rcx;
        }
        GetProcessHeap();
        HeapFree(rax, 0, var_70);
    }
    r14d |= 0xADDR;
    if (r14d < 0 /* signed */) goto loc_ADDR;
    eax = *(u32*)((rdi + 0xADDR));
    r11d = *(u32*)((rdi + 0xADDR));
    var_80 = eax;
    eax = (*(u32*)(rdi) + 4);
    if (eax < 4) goto loc_ADDR;
    edx = (rax + 4);
    if (edx < eax) goto loc_ADDR;
    ecx = (r11 + rdx);
    if (ecx < edx) goto loc_ADDR;
    eax = (rcx + 4);
    if (eax < ecx) goto loc_ADDR;
    ecx = var_80;
    ecx += eax;
    if (ecx < eax) goto loc_ADDR;
    eax = ecx;
    var_70 = rax;
    GetProcessHeap();
    HeapAlloc(rax, 8, var_70);
    var_b0 = rax;
    rcx = rax;
    if (rax == 0) goto loc_ADDR;
    eax = *(u32*)(rdi);
    *(u32*)(rcx) = eax;
    rax = (rcx + 4);
    var_98 = rax;
    if (rax < rcx) goto loc_ADDR;
    sub_1400274ac(rax, *(u64*)((rdi + 8)), *(u32*)(rdi));
    edx = *(u32*)(rdi);
    rdx += var_98;
    if (rdx < var_98) goto loc_ADDR;
    eax = *(u32*)((rdi + 0xADDR));
    *(u32*)(rdx) = eax;
    rax = (rdx + 4);
    var_98 = rax;
    if (rax < rdx) goto loc_ADDR;
    sub_1400274ac(rax, *(u64*)((rdi + 0xADDR)), *(u32*)((rdi + 0xADDR)));
    ecx = *(u32*)((rdi + 0xADDR));
    rcx += var_98;
    if (rcx < var_98) goto loc_ADDR;
    eax = *(u32*)((rdi + 0xADDR));
    *(u32*)(rcx) = eax;
    rax = (rcx + 4);
    var_98 = rax;
    if (rax < rcx) goto loc_ADDR;
    sub_1400274ac(rax, *(u64*)((rdi + 0xADDR)), *(u32*)((rdi + 0xADDR)));
    eax = *(u32*)((rdi + 0xADDR));
    rax += var_98;
    if (rax >= var_98) {
        rcx = var_b0;
        eax = var_70;
        var_140 = rcx;
        goto loc_ADDR;
        loc_ADDR:
        rax = var_b0;
        var_b0 = rax;
        goto loc_ADDR;
        loc_ADDR:
        var_b0 = rcx;
    }
    loc_ADDR:
    rax = var_140;
    r14d = 0xADDR;
    var_140 = rax;
    GetProcessHeap();
    HeapFree(rax, 0, var_b0);
    r14d |= 0xADDR;
    rcx = var_140;
    if (r14d < 0 /* signed */) goto loc_ADDR;
    eax = ecx;
    loc_ADDR:
    r14d = 0;
    if (var_348 != 0) {
        edx = (r14 + 1);
        if (var_340 > edx) goto loc_ADDR;
    }
    do {
        r14d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        rdx = var_348;
        r9d = r14d;
        do {
            r8 = (rdx + 4);
            var_b8 = eax;
            var_140 = rcx;
            if (r8 < rdx) goto loc_ADDR;
            edx = *(u32*)(rdx);
            rdx += r8;
            if (rdx < r8) goto loc_ADDR;
            r9d += 1;
            eax = var_b8;
        } while (r9d < eax);
        r14d = *(u32*)(rdx);
        rax = (rdx + 4);
        if (rax < rdx) goto loc_ADDR;
        r11d = 0;
        var_140 = rcx;
        rax = rcx;
    } while (var_348 == r11);
    if (var_340 <= 2) {
        r14d = 0xADDR;
        loc_ADDR:
        var_140 = rax;
        goto loc_ADDR;
    }
    rdx = var_348;
    r8 = rdi;
    var_80 = r11d;
    r9 = rsi;
    r11 = r12;
    r10 = rbx;
    do {
        ecx = var_b8;
        r12 = r11;
        var_a0 = r14d;
        rbx = r10;
        r14 = (rdx + 4);
        var_b8 = ecx;
        var_140 = rax;
        rcx = rax;
        rsi = r9;
        rdi = r8;
        if (r14 < rdx) goto loc_ADDR;
        edx = *(u32*)(rdx);
        rdx += r14;
        if (rdx < r14) goto loc_ADDR;
        r14d = (var_80 + 1);
        var_80 = r14d;
        r14d = var_a0;
    } while (r14d < 2);
    r8d = *(u32*)(rdx);
    rax = (rdx + 4);
    var_80 = r8d;
    if (rax < rdx) goto loc_ADDR;
    eax = (r14 + 0xADDR);
    if (eax < 0xADDR) goto loc_ADDR;
    edx = (rax + 4);
    if (edx < eax) goto loc_ADDR;
    eax = (rdx + r8);
    if (eax < edx) goto loc_ADDR;
    if (eax > 0xADDR) {
        r14d = 0xADDR;
        goto loc_ADDR;
    }
    r14d = eax;
    var_70 = rdi;
    var_98 = rbx;
    var_90 = rcx;
    GetProcessHeap();
    HeapAlloc(rax, 8, r14d);
    r10 = var_d0;
    r9 = rsi;
    rcx = rax;
    r8 = rbx;
    rdx = r12;
    r10 = ((rax != 0) ? rax : r10);
    rcx = var_90;
    r14d = ((rcx != 0) ? -1 : 0);
    var_d0 = r10;
    r14d &= 0xADDR;
    var_140 = rcx;
    r14d += 0xADDR;
    r11d = 0;
    if (rax == 0) goto loc_ADDR;
    ebx = 0;
    xmm0 = 0;
    var_2e8 = rbx;
    var_360 = xmm0;
    var_370 = xmm0;
    if (rcx == 0) goto loc_ADDR;
    if (r10 == 0) {
        loc_ADDR:
        r14d = 0xADDR;
        rbx = r8;
        goto loc_ADDR;
    }
    eax = var_b8;
    edi = var_a0;
    var_360 = rcx;
    edi += 0xADDR;
    edi += var_80;
    var_374 = edi;
    var_370 = eax;
    var_368 = r10;
    var_378 = ebx;
    GetModuleHandleExW(1, "ntdll.dll", &var_2e8, rsi);
    if (eax != 0) goto loc_ADDR;
    loc_ADDR:
    GetLastError();
    r14d = eax;
    if (eax > 0 /* signed */) {
        r14d = (uint32)(ax);
        r14d |= 0xADDR;
    }
    edx = edi;
    if (r14d >= 0 /* signed */) {
        r14d = 0xADDR;
    }
    loc_ADDR:
    if (r14d != 0xADDR) goto loc_ADDR;
    rdi = var_70;
    r14d = 0xADDR;
    rbx = var_98;
    rcx = var_90;
    loc_ADDR:
    while (1) {
        var_140 = rcx;
        goto loc_ADDR;
        loc_ADDR:
        GetProcAddress(var_2e8, "NtQuerySystemInformation");
        if (rax == 0) goto loc_ADDR;
        r9d = 0;
        r8d = (r9 + 0xADDR);
        sub_140028010((r8 + 0xADDR), &var_360, (r9 + 0xADDR));
        r14d = eax;
        edx = edi;
        r14d |= 0xADDR;
        if (r14d < 0 /* signed */) goto loc_ADDR;
        edx = var_374;
        r9 = r12;
        r8 = var_90;
        r11 = rsi;
        r10 = var_98;
        rax = var_70;
        goto loc_ADDR;
        loc_ADDR:
        rax = var_70;
        r11 = rsi;
        r10 = var_98;
        rdi = rax;
        r8 = var_90;
        rbx = r10;
        var_140 = r8;
        r9 = r12;
        if (r14d < 0 /* signed */) goto loc_ADDR;
        ebx = 0;
        loc_ADDR:
        var_90 = rbx;
        rbx = r10;
        rsi = r11;
        r12 = r9;
        if (edx >= 4) goto loc_ADDR;
        rdi = rax;
        loc_ADDR:
        r14d = 0xADDR;
        loc_ADDR:
        while (1) {
            var_140 = r8;
            goto loc_ADDR;
            loc_ADDR:
            rdi = var_d0;
            r14d = *(u32*)(rdi);
            rcx = (rdi + 4);
            var_80 = r14d;
            rdi = rax;
            if (rcx < rdi) goto loc_ADDR;
            eax = (rdx - 4);
            rcx = r8;
            var_140 = rcx;
            if (eax >= r14d) goto loc_ADDR;
            r14d = 0xADDR;
            while (1) {
                rdi = var_70;
                goto loc_ADDR;
                loc_ADDR:
                r15 = var_d0;
                rax = (r15 + 4);
                rax += r14;
                var_138 = rax;
                rax = (r15 + 4);
                if (var_138 < rax) goto loc_ADDR;
                r14d += 4;
                if (r14d < 4) goto loc_ADDR;
                eax = edx;
                eax -= r14d;
                if (eax >= 4) goto loc_ADDR;
                loc_ADDR:
                do {
                    r14d = 0xADDR;
                    goto loc_ADDR;
                    loc_ADDR:
                    rax = var_138;
                    var_140 = rcx;
                    r8d = *(u32*)(rax);
                    r9 = (rax + 4);
                    var_b8 = r8d;
                    if (r9 < rax) goto loc_ADDR;
                    r11d = (r14 + 4);
                    rax = rbx;
                    if (r11d < r14d) goto loc_ADDR;
                    eax = edx;
                    eax -= r11d;
                    if (eax < r8d) goto loc_ADDR;
                    r10 = (r9 + r8);
                    var_140 = rcx;
                    if (r10 < r9) goto loc_ADDR;
                    r8d += r11d;
                    rax = r12;
                    var_a0 = r8d;
                    r9 = rbx;
                    if (r8d < r11d) goto loc_ADDR;
                    eax = edx;
                    eax -= r8d;
                    if (eax < 4) goto loc_ADDR;
                    eax = *(u32*)(r10);
                    r9 = (r10 + 4);
                    var_e0 = eax;
                    var_128 = r9;
                    var_140 = rcx;
                    if (r9 < r10) goto loc_ADDR;
                    eax = (r8 + 4);
                    r9 = rbx;
                    r10 = rsi;
                    if (eax < r8d) goto loc_ADDR;
                    r15d = (r8 + 4);
                    var_70 = rcx;
                    eax = edx;
                    r8 = rcx;
                    eax -= r15d;
                    r15 = 0xADDR;
                } while (eax < var_e0);
                r14d = var_a0;
                eax = (var_e0 + 4);
                r14d += eax;
                eax = (var_a0 + 4);
                var_e8 = r14d;
                r14 = rdi;
                if (r14d < eax) goto loc_ADDR;
                var_140 = rcx;
                var_88 = r12;
                var_98 = rbx;
                var_68 = rsi;
                var_b0 = rdi;
                if (edx != var_e8) goto loc_ADDR;
                ecx = var_b8;
                ecx += var_80;
                ecx += var_e0;
                rcx += 0xADDR;
                eax = edx;
                if (rcx != rax) goto loc_ADDR;
                GetProcessHeap();
                edx = 8;
                HeapAlloc(rax, 8, (rdx + 0xADDR));
                r9d = 0;
                var_110 = rax;
                rcx = rax;
                if (rax == 0) goto loc_ADDR;
                rax = var_d0;
                r14d = r9d;
                rax += 4;
                if (rax != 0) {
                    eax = var_80;
                    *(u32*)(rcx) = eax;
                    r14d = eax;
                    var_80 = rax;
                    GetProcessHeap();
                    HeapAlloc(rax, 8, r14d);
                    edx = 0;
                    rcx = rax;
                    if (rax == 0) {
                        rax = var_70;
                        var_140 = rax;
                        var_b0 = rdi;
                        var_68 = rsi;
                        var_98 = rbx;
                        var_88 = r12;
                        goto loc_ADDR;
                    }
                    rax = var_110;
                    *(u64*)((rax + 8)) = rcx;
                    r14d = edx;
                    rdx = (var_d0 + 4);
                    sub_1400274ac(rcx, rdx, var_80);
                    rcx = var_110;
                    r9d = 0;
                    rdx = var_128;
                    goto loc_ADDR;
                }
                rdx = var_128;
                rax = var_70;
                *(u32*)(rcx) = r9d;
                var_128 = rdx;
                var_140 = rax;
                *(u64*)((rcx + 8)) = r9;
                var_b0 = rdi;
                var_68 = rsi;
                var_98 = rbx;
                var_88 = r12;
                loc_ADDR:
                rax = var_138;
                var_110 = rcx;
                rax += 4;
                if (rax != 0) {
                    eax = var_b8;
                    *(u32*)((rcx + 0xADDR)) = eax;
                    r14d = eax;
                    var_70 = rax;
                    GetProcessHeap();
                    HeapAlloc(rax, 8, r14d);
                    edx = 0;
                    rcx = rax;
                    if (rax == 0) goto loc_ADDR;
                    rax = var_110;
                    *(u64*)((rax + 0xADDR)) = rcx;
                    r14d = edx;
                    rdx = (var_138 + 4);
                    sub_1400274ac(rcx, rdx, var_70);
                    rcx = var_110;
                    r9d = 0;
                    rdx = var_128;
                    rax = var_140;
                    var_110 = rcx;
                    var_128 = rdx;
                    var_140 = rax;
                    var_b0 = rdi;
                    var_68 = rsi;
                    var_98 = rbx;
                    var_88 = r12;
                    goto loc_ADDR;
                }
                *(u32*)((rcx + 0xADDR)) = r9d;
                *(u64*)((rcx + 0xADDR)) = r9;
                loc_ADDR:
                if (rdx != 0) {
                    eax = var_e0;
                    *(u32*)((rcx + 0xADDR)) = eax;
                    r14d = eax;
                    var_70 = rax;
                    GetProcessHeap();
                    HeapAlloc(rax, 8, r14d);
                    edx = 0;
                    rcx = rax;
                    if (rax == 0) {
                        loc_ADDR:
                        rax = var_110;
                        r14d = 0xADDR;
                        rcx = *(u64*)((rax + 8));
                        var_70 = rcx;
                        if (rcx != 0) {
                            GetProcessHeap();
                            HeapFree(rax, 0, var_70);
                            rax = var_110;
                            edx = 0;
                            *(u64*)((rax + 8)) = rdx;
                        }
                        rcx = *(u64*)((rax + 0xADDR));
                        var_70 = rcx;
                        if (rcx != 0) {
                            GetProcessHeap();
                            HeapFree(rax, 0, var_70);
                            rax = var_110;
                            edx = 0;
                            *(u64*)((rax + 0xADDR)) = rdx;
                        }
                        rax = *(u64*)((rax + 0xADDR));
                        var_70 = rax;
                        if (rax != 0) {
                            GetProcessHeap();
                            HeapFree(rax, 0, var_70);
                            rax = var_110;
                            ecx = 0;
                            *(u64*)((rax + 0xADDR)) = rcx;
                        }
                        GetProcessHeap();
                        HeapFree(rax, 0, var_110);
                        goto loc_ADDR;
                    }
                    rax = var_110;
                    *(u64*)((rax + 0xADDR)) = rcx;
                    r14d = edx;
                    sub_1400274ac(rcx, var_128, var_70);
                    rax = var_140;
                    rcx = var_110;
                    var_140 = rax;
                    var_b0 = rdi;
                    var_68 = rsi;
                    var_98 = rbx;
                    var_88 = r12;
                    goto loc_ADDR;
                }
                *(u32*)((rcx + 0xADDR)) = r9d;
                *(u64*)((rcx + 0xADDR)) = r9;
                loc_ADDR:
                var_90 = rcx;
                goto loc_ADDR;
                loc_ADDR:
                rax = var_70;
                r14d = 0xADDR;
                var_140 = rax;
                var_b0 = rdi;
                var_68 = rsi;
                var_98 = rbx;
                var_88 = r12;
                loc_ADDR:
                rcx = var_90;
                loc_ADDR:
                eax = 0;
                if (r14d >= 0 /* signed */) goto loc_ADDR;
                var_110 = rax;
                if (rcx != 0) {
                    rax = *(u64*)((rcx + 8));
                    var_70 = rax;
                    if (rax != 0) {
                        GetProcessHeap();
                        HeapFree(rax, 0, var_70);
                        rcx = var_90;
                        eax = 0;
                        *(u64*)((rcx + 8)) = rax;
                    }
                    rax = *(u64*)((rcx + 0xADDR));
                    var_70 = rax;
                    if (rax != 0) {
                        GetProcessHeap();
                        HeapFree(rax, 0, var_70);
                        rcx = var_90;
                        eax = 0;
                        *(u64*)((rcx + 0xADDR)) = rax;
                    }
                    rax = *(u64*)((rcx + 0xADDR));
                    var_70 = rax;
                    if (rax != 0) {
                        GetProcessHeap();
                        HeapFree(rax, 0, var_70);
                        rax = var_90;
                        ecx = 0;
                        *(u64*)((rax + 0xADDR)) = rcx;
                    }
                    GetProcessHeap();
                    HeapFree(rax, 0, var_90);
                    ecx = 0;
                    loc_ADDR:
                    var_110 = rcx;
                }
                r14d |= 0xADDR;
                if (r14d < 0 /* signed */) goto loc_ADDR;
                r14 = var_110;
                if (r14 != 0) goto loc_ADDR;
                loc_ADDR:
                do {
                    r14d = 0xADDR;
                    goto loc_ADDR;
                    loc_ADDR:
                    rax = *(u64*)((r14 + 8));
                    r11d = 0;
                    var_70 = rax;
                    if (rax == 0) goto loc_ADDR;
                } while (*(u32*)(r14) == r11d);
                ecx = *(u32*)(r14);
                rcx -= 0xADDR;
                var_90 = rcx;
                sub_140024bf8(rcx);
                r11d = 0;
                var_100 = rax;
                r8 = rax;
                if (rax == 0) goto loc_ADDR;
                r10 = var_90;
                al = r11b;
                rcx = var_70;
                rdx = r10;
                edx &= 7;
                var_60 = al;
                var_a0 = rdx;
                var_898 = r15;
                var_138 = rcx;
                var_e8 = r8;
                if (edx == 0) goto loc_ADDR;
                var_154 = r11d;
                r9d = r11d;
                if (edx == 0) goto loc_ADDR;
                rdx = var_140;
                r13d = r11d;
                do {
                    r11d = (uint32)(*(u8*)(rcx));
                    ecx = var_150;
                    if (r13d < 4) {
                        ecx -= 0xADDR;
                        r11d <<= cl;
                        r9d |= r11d;
                        goto loc_ADDR;
                    }
                    r11d <<= cl;
                    var_154 |= r11d;
                    loc_ADDR:
                    rcx = var_138;
                    r13d += 1;
                    var_150 -= 8;
                    rcx += 1;
                    var_138 = rcx;
                } while (r13d < var_a0 /* signed */);
                r13d = var_d8;
                r11d = r9d;
                var_80 = r9d;
                r11d ^= 0xADDR;
                r9d = var_154;
                eax = r11d;
                r15d = var_80;
                r9d ^= 0xADDR;
                r10d = 0;
                r8d = r9d;
                var_b8 = r10d;
                r10 = var_e8;
                do {
                    var_138 = rcx;
                    ecx = var_154;
                    var_154 = ecx;
                    var_140 = rdx;
                    if (var_b8 < 4 /* signed */) {
                        edx = eax;
                        ecx = eax;
                        edx >>= 0xADDR;
                        ecx <<= 8;
                        eax = edx;
                        eax |= ecx;
                        ecx = eax;
                        goto loc_ADDR;
                    }
                    edx = r8d;
                    ecx = r8d;
                    edx >>= 0xADDR;
                    ecx <<= 8;
                    r8d = edx;
                    r8d |= ecx;
                    ecx = r8d;
                    loc_ADDR:
                    *(u8*)(r10) = cl;
                    rdx = (r10 + 1);
                    ecx = var_b8;
                    r10 = rdx;
                    rdx = var_140;
                    ecx += 1;
                    var_b8 = ecx;
                    rcx = var_138;
                } while (ecx < var_a0 /* signed */);
                al = var_60;
                ecx = 0;
                r8 = var_100;
                rdx = var_a0;
                var_80 = r15d;
                r15 = 0xADDR;
                var_e8 = r10;
                r10 = var_90;
                var_110 = r14;
                var_b0 = rdi;
                var_88 = r12;
                var_68 = rsi;
                var_98 = rbx;
                goto loc_ADDR;
                loc_ADDR:
                ecx = 0;
                r11d = 0xADDR;
                var_80 = ecx;
                r9d = 0xADDR;
                var_154 = ecx;
                loc_ADDR:
                if (edx > 4) {
                    ecx = 8;
                    ecx -= edx;
                    ecx <<= 3;
                    r9d >>= cl;
                    r9d <<= cl;
                    var_b8 = r9d;
                    goto loc_ADDR;
                }
                var_b8 = ecx;
                __asm("if (jae short 0xADDR /* unmodelled flags */) goto 0xADDR");
                ecx = 4;
                ecx -= edx;
                ecx <<= 3;
                r11d >>= cl;
                r11d <<= cl;
                goto loc_ADDR;
                loc_ADDR:
                ecx = 0;
                var_80 = ecx;
                var_b8 = ecx;
                loc_ADDR:
                rcx = r10;
                edx = 0;
                rcx >>= 3;
                if (rcx == 0) goto loc_ADDR;
                rsi = var_e8;
                r13 = rcx;
                r12d = var_89c;
                rsi += 7;
                rbx = var_138;
                edi = (uint32)(var_89a);
                rbx += 2;
                r14d = var_80;
                eax = (uint32)(r15w);
                r8d = (uint32)(r12w);
                var_e0 = eax;
                eax = var_b8;
                var_a0 = r8d;
                do {
                    ecx = (uint32)(*(u8*)((rbx - 1)));
                    r10d = (uint32)(*(u8*)((rbx - 2)));
                    r9d = (uint32)(*(u8*)((rbx + 2)));
                    r10d <<= 8;
                    r10d |= ecx;
                    r9d <<= 8;
                    ecx = (uint32)(*(u8*)(rbx));
                    rbx += 8;
                    r10d <<= 8;
                    r10d |= ecx;
                    ecx = (uint32)(*(u8*)((rbx - 7)));
                    r10d <<= 8;
                    r10d |= ecx;
                    ecx = (uint32)(*(u8*)((rbx - 5)));
                    r9d |= ecx;
                    r11d ^= r10d;
                    ecx = (uint32)(*(u8*)((rbx - 4)));
                    r9d <<= 8;
                    r9d |= ecx;
                    ecx = (uint32)(*(u8*)((rbx - 3)));
                    r9d <<= 8;
                    r9d |= ecx;
                    ecx = r15d;
                    ecx ^= r11d;
                    eax ^= r9d;
                    eax ^= ecx;
                    eax ^= r12d;
                    edx = eax;
                    ecx = eax;
                    edx -= r15d;
                    ecx = ror(ecx, 0xADDR);
                    edx = ror(edx, 0xADDR);
                    edx *= r8d;
                    edx += ecx;
                    r11d ^= edx;
                    ecx = r11d;
                    ecx = ror(ecx, 0xADDR);
                    edx = (r11 + r12);
                    edx = ror(edx, 9);
                    edx *= edi;
                    edx -= ecx;
                    eax ^= edx;
                    edx = eax;
                    ecx = eax;
                    edx -= r8d;
                    ecx >>= 0xADDR;
                    edx *= var_e0;
                    edx -= ecx;
                    ecx = (uint32)(var_89e);
                    r11d ^= edx;
                    edx = r11d;
                    edx ^= edi;
                    edx = ror(edx, 0xADDR);
                    edx *= ecx;
                    ecx = r11d;
                    ecx = ror(ecx, 0xADDR);
                    edx -= ecx;
                    ecx = r12d;
                    eax ^= edx;
                    edx = eax;
                    edx ^= r15d;
                    ecx -= edx;
                    r11d ^= ecx;
                    ecx = r11d;
                    edx = r11d;
                    edx = ror(edx, 6);
                    ecx ^= r8d;
                    ecx *= edi;
                    edx ^= ecx;
                    eax ^= edx;
                    ecx = eax;
                    ecx = ror(ecx, 0xADDR);
                    edx = (rax + r12);
                    edx = ror(edx, 0xADDR);
                    edx *= var_e0;
                    edx += ecx;
                    ecx = (uint32)(var_89e);
                    r11d ^= edx;
                    edx = r11d;
                    edx -= r15d;
                    edx = ror(edx, 0xADDR);
                    edx *= ecx;
                    ecx = r11d;
                    ecx = ror(ecx, 0xADDR);
                    edx -= ecx;
                    eax ^= edx;
                    edx = r15d;
                    edx ^= eax;
                    ecx = eax;
                    edx = ror(edx, 0xADDR);
                    edx *= r8d;
                    ecx = ror(ecx, 0xADDR);
                    edx ^= ecx;
                    r11d ^= edx;
                    edx = (uint32)(var_89e);
                    edx ^= r11d;
                    ecx = r11d;
                    edx *= edi;
                    ecx >>= 0xADDR;
                    edx ^= ecx;
                    eax ^= edx;
                    edx = (uint32)(var_89e);
                    ecx = eax;
                    ecx = ~(ecx);
                    ecx = ror(ecx, 5);
                    ecx += var_e0;
                    ecx *= edx;
                    r11d ^= ecx;
                    ecx = r11d;
                    ecx -= edx;
                    ecx ^= r15d;
                    eax ^= ecx;
                    ecx = (uint32)(var_89e);
                    edx = eax;
                    edx ^= ecx;
                    ecx = eax;
                    edx = ror(edx, 0xADDR);
                    edx *= r8d;
                    ecx >>= 2;
                    edx += ecx;
                    r11d ^= edx;
                    edx = r11d;
                    ecx = r11d;
                    edx -= r12d;
                    ecx = ror(ecx, 0xADDR);
                    edx = ror(edx, 6);
                    edx *= edi;
                    edx += ecx;
                    eax ^= edx;
                    ecx = eax;
                    edx = eax;
                    ecx ^= r8d;
                    edx = ror(edx, 9);
                    ecx *= var_e0;
                    edx += ecx;
                    ecx = (uint32)(var_89e);
                    r11d ^= edx;
                    edx = r11d;
                    edx ^= edi;
                    edi = r15d;
                    edx = ror(edx, 0xADDR);
                    edi ^= r11d;
                    edx *= ecx;
                    ecx = r11d;
                    r11d = (uint32)(var_89a);
                    ecx = ror(ecx, 0xADDR);
                    edx += ecx;
                    eax ^= edx;
                    edi ^= eax;
                    edi ^= r12d;
                    ecx = edi;
                    ecx = ror(ecx, 3);
                    ecx -= r11d;
                    ecx *= r8d;
                    eax ^= ecx;
                    edx = eax;
                    ecx = eax;
                    edx -= r12d;
                    ecx = ror(ecx, 6);
                    edx = ror(edx, 1);
                    edx *= var_e0;
                    edx -= ecx;
                    ecx = (uint32)(var_89e);
                    edi ^= edx;
                    edx = (r15 + rdi);
                    edx = ror(edx, 0xADDR);
                    edx *= ecx;
                    ecx = edi;
                    ecx = ror(ecx, 0xADDR);
                    edx += ecx;
                    eax ^= edx;
                    ecx = eax;
                    ecx = ror(ecx, 0xADDR);
                    edx = (r15 + rax);
                    edx = ror(edx, 0xADDR);
                    edx *= r8d;
                    edx -= ecx;
                    edi ^= edx;
                    edx = edi;
                    ecx = edi;
                    edx ^= var_e0;
                    edx *= r11d;
                    ecx >>= 3;
                    edx ^= ecx;
                    eax ^= edx;
                    r11d = eax;
                    ecx = eax;
                    eax ^= var_154;
                    r11d ^= r12d;
                    ecx = ror(ecx, 0xADDR);
                    r11d = ror(r11d, 0xADDR);
                    r11d *= var_e0;
                    var_154 = r9d;
                    r9d = 1;
                    r11d ^= ecx;
                    r11d ^= edi;
                    edi = eax;
                    r11d ^= r14d;
                    edi <<= 0xADDR;
                    *(u8*)((rsi - 4)) = r11b;
                    edx = r11d;
                    *(u8*)(rsi) = al;
                    ecx = (r11d >> 8);
                    rsi += 8;
                    edx <<= 0xADDR;
                    r14d = r10d;
                    edx |= ecx;
                    ecx = eax;
                    *(u8*)((rsi - 0xADDR)) = dl;
                    r8d = edx;
                    ecx >>= 8;
                    edi |= ecx;
                    r8d <<= 0xADDR;
                    *(u8*)((rsi - 9)) = dil;
                    ecx = (edi << 0xADDR);
                    edi >>= 8;
                    ecx |= edi;
                    edx >>= 8;
                    edi = (uint32)(var_89a);
                    r8d |= edx;
                    *(u8*)((rsi - 0xADDR)) = r8b;
                    *(u8*)((rsi - 0xADDR)) = cl;
                    r8d = ror(r8d, 8);
                    *(u8*)((rsi - 0xADDR)) = r8b;
                    r8d = var_a0;
                    ecx = ror(ecx, 8);
                    *(u8*)((rsi - 0xADDR)) = cl;
                    r13 -= r9;
                } while (r13 != 0);
                al = var_60;
                edx = 0;
                rbx = var_98;
                rsi = var_68;
                r13d = var_d8;
                r12 = var_88;
                rdi = var_b0;
                r14 = var_110;
                r8 = var_100;
                r10 = var_90;
                goto loc_ADDR;
                loc_ADDR:
                r9d = 1;
                loc_ADDR:
                rcx = rdx;
                if (r10 != 0) {
                    do {
                        al ^= *(u8*)((rcx + r8));
                        rcx += r9;
                    } while (rcx < r10);
                }
                r15 = var_140;
                rdx = r14;
                ecx = (uint32)(al);
                rax = var_70;
                var_140 = r15;
                var_110 = r14;
                if (rcx != *(u64*)((r10 + rax))) {
                    sub_140024e2c(r8);
                    r11d = 0;
                    loc_ADDR:
                    r14d = 0xADDR;
                    var_100 = r11;
                    goto loc_ADDR;
                }
                var_110 = rdx;
                rcx = r15;
                var_140 = rcx;
                if (r10d < 4) {
                    loc_ADDR:
                    r14d = 0xADDR;
                    goto loc_ADDR;
                }
                r11d = *(u32*)(r8);
                var_80 = r11d;
                r11 = (r8 + 4);
                var_140 = rcx;
                var_110 = rdx;
                var_100 = r8;
                if (r11 < r8) goto loc_ADDR;
                eax = (r10 - 4);
                if (eax >= 4) goto loc_ADDR;
                loc_ADDR:
                do {
                    r14d = 0xADDR;
                    var_100 = r8;
                    var_110 = rdx;
                    var_140 = r15;
                    goto loc_ADDR;
                    loc_ADDR:
                    r14d = *(u32*)(r11);
                    r9 = (r11 + 4);
                    var_b8 = r14d;
                    if (r9 < r11) goto loc_ADDR;
                    eax = (r10 - 8);
                    var_70 = r15;
                    if (eax < r14d) goto loc_ADDR;
                    eax = (r14 + 8);
                    if (eax < 8) goto loc_ADDR;
                    ecx = r10d;
                    r10 = (r14 + r9);
                    rax = (r8 + rcx);
                    if (rax < r10) goto loc_ADDR;
                    rax = r8;
                    rax -= r14;
                    rax -= r9;
                    rax += rcx;
                } while (rax >= 8);
                eax = 0;
                var_e0 = r9;
                var_100 = r8;
                r14d = eax;
                var_110 = rdx;
                r15d = eax;
                rax = var_70;
                var_140 = rax;
                rcx = rax;
                var_b0 = rdi;
                if (r9 != 0) {
                    var_100 = r8;
                    var_110 = rdx;
                    var_140 = rax;
                    if (r10 >= r9) {
                        ecx = 0;
                        var_e0 = r9;
                        var_b0 = rdi;
                        rcx = rax;
                        if (r9 < r10) {
                            rax = r9;
                            var_100 = r8;
                            var_110 = rdx;
                            var_140 = rcx;
                            do {
                                var_e0 = rax;
                                rax = (r9 + 4);
                                if (rax < r9) goto loc_ADDR;
                                r14 = (r9 + 4);
                                var_140 = rcx;
                                var_110 = rdx;
                                rax = rbx;
                                var_100 = r8;
                                if (r14 > r10) goto loc_ADDR;
                                eax = (*(u32*)(r9) + 4);
                                if (eax < 4) goto loc_ADDR;
                                r11d = eax;
                                r11 += r9;
                                if (r11 < r9) goto loc_ADDR;
                                r9 = r11;
                                r11d = 0;
                                r14d = r11d;
                                if (r9 > r10) goto loc_ADDR;
                                rax = var_e0;
                                r15d += 1;
                            } while (r9 < r10);
                            var_100 = r8;
                            var_110 = rdx;
                            var_b0 = rdi;
                            var_140 = rcx;
                        }
                        __asm("if (je short 0xADDR /* unmodelled flags */) goto 0xADDR");
                        r14d = 0xADDR;
                        goto loc_ADDR;
                        loc_ADDR:
                        r14d = 0xADDR;
                        var_100 = r8;
                        var_110 = rdx;
                        goto loc_ADDR;
                        loc_ADDR:
                        var_100 = r8;
                        var_110 = rdx;
                        var_140 = rcx;
                    }
                    loc_ADDR:
                    r14d = 0xADDR;
                    goto loc_ADDR;
                    loc_ADDR:
                    r14d = 0xADDR;
                    var_140 = rcx;
                    goto loc_ADDR;
                }
                var_70 = rcx;
                eax = 0;
                ecx = var_b8;
                var_88 = rdi;
                var_138 = rdx;
                var_e8 = r8;
                if (ecx != 0) {
                    edi = ecx;
                    GetProcessHeap();
                    HeapAlloc(rax, 8, edi);
                    r14d = 0;
                    rdi = rax;
                    if (rax != 0) {
                        ecx = var_b8;
                        goto loc_ADDR;
                    }
                    rdi = var_b0;
                    goto loc_ADDR;
                }
                rdi = rax;
                loc_ADDR:
                rax = var_e0;
                if (rax != 0) {
                    r8d = ecx;
                    rdx = rax;
                    rcx = rdi;
                    sub_1400274ac(rdi, rax, ecx);
                }
                var_358 = rdi;
                var_350 = r15d;
                rax = var_e8;
                r9 = rsi;
                r11 = var_138;
                r8 = rbx;
                r10 = var_88;
                rdx = r12;
                rcx = var_70;
                rdi = r10;
                var_100 = rax;
                var_110 = r11;
                var_140 = rcx;
                if (r14d < 0 /* signed */) goto loc_ADDR;
                var_100 = rax;
                var_110 = r11;
                var_140 = rcx;
                if (var_80 == r15d) goto loc_ADDR;
                r14d = 0xADDR;
                var_100 = rax;
                var_110 = r11;
                loc_ADDR:
                var_140 = rcx;
                goto loc_ADDR;
                loc_ADDR:
                r14d = 0xADDR;
                var_140 = r15;
                loc_ADDR:
                var_110 = rdx;
                var_100 = r8;
                goto loc_ADDR;
                loc_ADDR:
                r14d = 0xADDR;
                goto loc_ADDR;
                loc_ADDR:
                r14d = 0xADDR;
                goto loc_ADDR;
                loc_ADDR:
                r14d = 0xADDR;
            }
            loc_ADDR:
            r14d = 0xADDR;
        }
        loc_ADDR:
        r14d = 0xADDR;
        goto loc_ADDR;
        loc_ADDR:
        r14d = 0xADDR;
    }
    loc_ADDR:
    r14d = 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    r14d = 0;
    var_140 = r14;
    loc_ADDR:
    r14d = 0xADDR;
    loc_ADDR:
    r14d |= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    r14d = 0xADDR;
    goto loc_ADDR;
    GetProcessHeap();
    HeapFree(rax, 0, rdi);
    goto loc_ADDR;
    rax = (rcx + 4);
    goto loc_ADDR;
}
