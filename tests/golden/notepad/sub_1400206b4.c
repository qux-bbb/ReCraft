// L1 structure recovery (59 blocks)
void sub_1400206b4_l1(void) {
    xmm0 = 0;
    eax = 0;
    rdi = rcx;
    ecx = 0;
    var_d0 = 0;
    ebx = 0;
    var_38 = 0;
    r14d = 0;
    dword_ADDR = ecx;
    r12d = ecx;
    qword_ADDR = rcx;
    r15d = ecx;
    var_68 = rax;
    var_98 = xmm0;
    var_a8 = xmm0;
    var_b8 = xmm0;
    var_48 = xmm0;
    var_58 = xmm0;
    SetCursor(qword_ADDR);
    GetDeviceCaps(rdi, (r15 + 8));
    dword_ADDR = eax;
    GetDeviceCaps(rdi, (r15 + 0xADDR));
    dword_ADDR = eax;
    GetDeviceCaps(rdi, (r15 + 0xADDR));
    dword_ADDR = eax;
    GetDeviceCaps(rdi, (r15 + 0xADDR));
    dword_ADDR = eax;
    GetDeviceCaps(rdi, (r15 + 0xADDR));
    dword_ADDR = eax;
    GetDeviceCaps(rdi, (r15 + 0xADDR));
    dword_ADDR = eax;
    GetDeviceCaps(rdi, (r15 + 0xADDR));
    dword_ADDR = eax;
    GetLocalTime(off_140035a80);
    esi = 0xADDR;
    var_28 = 0xADDR;
    var_20 = off_140034870;
    GetDateFormatW(esi, (r15 + 2), off_140035a80, 0, off_140034870, 0xADDR);
    rax = off_140034670;
    var_28 = 0xADDR;
    var_20 = off_140034670;
    GetTimeFormatW(esi, 0, off_140035a80, 0, off_140034670, 0xADDR);
    xmm0 = xmmword_ADDR;
    xmm1 = xmmword_ADDR;
    xmm0 = xmmword_ADDR;
    var_f0 = xmm1;
    xmm1 = xmmword_ADDR;
    var_100 = xmm0;
    xmm0 = xmmword_ADDR;
    var_110 = xmm1;
    xmm1 = dbl_ADDR;
    var_120 = xmm0;
    var_130 = xmm1;
    ecx = dword_ADDR;
    esi = 0;
    ecx *= dword_ADDR;
    var_138 = dword_ADDR;
    eax = 0xADDR;
    var_e4 = 0;
    edx = (uint32)((((int64)((int32)(eax)) * (int64)((int32)(ecx))) >> 0xADDR));
    edx -= ecx;
    edx >>= 9;
    eax = (edx >> 0xADDR);
    edx += eax;
    var_e0 = edx;
    SetMapMode(rdi, (rsi + 8));
    SetViewportExtEx(rdi, dword_ADDR, dword_ADDR, 0);
    SetWindowExtEx(rdi, dword_ADDR, dword_ADDR, 0);
    xmm0 = xmmword_ADDR;
    xmmword_ADDR = xmm0;
    LPtoDP(rdi, off_140036090, (r15 + 2));
    SetMapMode(rdi, (rsi + 1));
    CreateFontIndirectW(&var_e0);
    r13 = rax;
    if (rax == 0) {
        GetLastError();
        esi = eax;
        r12d = 0;
    } else {
        SelectObject(rdi, rax);
        r14 = rax;
        if (rax != 0) {
            SetBkMode(rdi, (rsi + 1));
            GetTextMetricsW(rdi, &var_98);
            if (eax != 0) {
                if ((var_cf & 6) != 0) {
                    ecx = dword_ADDR;
                    eax = var_ac;
                    edx = dword_ADDR;
                    r9d = var_a8;
                    r9d += var_98;
                    r8d = dword_ADDR;
                    eax <<= 3;
                    dword_ADDR = eax;
                    eax = dword_ADDR;
                    eax -= ecx;
                    dword_ADDR = r9d;
                    eax = ((eax <= 0 /* signed */) ? esi : eax);
                    dword_ADDR = eax;
                    eax = dword_ADDR;
                    eax -= dword_ADDR;
                    eax += dword_ADDR;
                    ecx += eax;
                    eax = edx;
                    ecx = ((ecx <= 0 /* signed */) ? esi : ecx);
                    dword_ADDR = ecx;
                    ecx = dword_ADDR;
                    r8d -= ecx;
                    r8d = ((r8d <= 0 /* signed */) ? esi : r8d);
                    eax -= dword_ADDR;
                    eax += dword_ADDR;
                    ecx += eax;
                    dword_ADDR = r8d;
                    ecx = ((ecx <= 0 /* signed */) ? esi : ecx);
                    edx -= ecx;
                    dword_ADDR = ecx;
                    edx -= r8d;
                    eax = edx;
                    edx = ((eax < 0 /* signed */) ? -1 : 0);
                    __asm("idiv r9d");
                    if (word_ADDR != si) {
                        eax -= 1;
                    }
                    if (word_ADDR != si) {
                        eax -= 1;
                    }
                    if (eax <= 0 /* signed */) {
                        MessageBoxW(qword_ADDR, qword_ADDR, qword_ADDR, 0xADDR);
                        SetLastError(0);
                        GetLastError();
                        esi = eax;
                        r12d = 0;
                    }
                    SetAbortProc(rdi, sub_140020260);
                    esi = eax;
                    if (eax >= 0 /* signed */) {
                        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
                        esi = 0;
                        var_30 = rax;
                        if (rax == 0) {
                            rbx = rax;
                        } else {
                            LocalLock(rax);
                            r15 = rax;
                            if (rax == 0) {
                                loc_ADDR:
                                rbx = var_30;
                                GetLastError();
                                esi = eax;
                                r12d = 0;
                            }
                            GetWindowTextLengthW(qword_ADDR);
                            edx = 0;
                            ebx = (rax + 1);
                            sub_140009e44(&var_40, 0, (int64)(ebx));
                            r8d = ebx;
                            rbx = var_40;
                            GetWindowTextW(qword_ADDR, rbx, ebx);
                            if (eax == 0) {
                                *(u16*)(rbx) = si;
                            }
                            EnableWindow(qword_ADDR, 0);
                            var_20 = rsi;
                            CreateDialogParamW(qword_ADDR, 0xADDR, qword_ADDR, sub_140020110, rsi);
                            qword_ADDR = rax;
                            if (rax != 0) {
                                xmm0 = 0;
                                var_48 = 0xADDR;
                                var_50 = rbx;
                                var_58 = xmm0;
                                var_68 = esi;
                                SetLastError(0);
                                StartDocW(rdi, &var_48);
                                if (eax > 0 /* signed */) {
                                    edx = dword_ADDR;
                                    var_80 = 0;
                                    eax = dword_ADDR;
                                    xmm0 = 0;
                                    var_88 = eax;
                                    eax = dword_ADDR;
                                    eax -= dword_ADDR;
                                    var_38 = 1;
                                    dword_ADDR = 1;
                                    ecx = dword_ADDR;
                                    ecx -= dword_ADDR;
                                    var_90 = eax;
                                    eax = dword_ADDR;
                                    var_8c = eax;
                                    var_70 = xmm0;
                                    var_94 = ecx;
                                    if (word_ADDR != si) {
                                        eax += edx;
                                        var_8c = eax;
                                    }
                                    if (word_ADDR != si) {
                                        ecx -= edx;
                                        var_94 = ecx;
                                    }
                                    wcsnlen(r15, 0xADDR);
                                    rsi = rax;
                                    GetWindowLongW(qword_ADDR, 0xADDR);
                                    eax &= 0xADDR;
                                    var_40 = eax;
                                    if (dword_ADDR != r12d) {
                                        esi = r12d;
                                        if (rbx != 0) {
                                            CoTaskMemFree(rbx);
                                        }
                                        rbx = var_30;
                                    } else {
                                        ecx = dword_ADDR;
                                        while (esi > 0 /* signed */) {
                                            r9d = ecx;
                                            sub_1400083f8(&var_140, 0xADDR, qword_ADDR, ecx);
                                            SetDlgItemTextW(qword_ADDR, 0xADDR, &var_140);
                                            edx = 0;
                                            sub_140021140(rdi, 0);
                                            eax = dword_ADDR;
                                            var_74 = eax;
                                            var_78 = r12;
                                            var_80 = r12d;
                                            var_70 = 0xADDR;
                                            StartPage(rdi);
                                            if (eax > 0 /* signed */) {
                                                eax = var_40;
                                                eax = -(eax);
                                                rax = &var_70;
                                                ecx = ((eax != 0) ? -1 : 0);
                                                var_28 = rax;
                                                ecx &= 0xADDR;
                                                r12d = 1;
                                                ecx += 0xADDR;
                                                var_20 = ecx;
                                                DrawTextExW(rdi, r15, esi, &var_88, (ecx + 0xADDR), &var_70);
                                                sub_140021140(rdi, r12d);
                                                EndPage(rdi);
                                                if (eax > 0 /* signed */) {
                                                    ecx = dword_ADDR;
                                                    r12d = 0;
                                                    edx = var_80;
                                                    ecx += 1;
                                                    dword_ADDR = ecx;
                                                    if (edx != 0) {
                                                        esi -= edx;
                                                        r15 += (rdx * 2);
                                                        if (dword_ADDR != 0) break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    GetLastError();
                                    esi = eax;
                                }
                            } else {
                                if (rbx == 0) goto loc_ADDR;
                            }
                        }
                    }
                } else {
                    SelectObject(rdi, r14);
                    DeleteObject(rax);
                    edx = 0;
                    sub_1400029dc(&var_fc, 0, (r15 + 0xADDR));
                    CreateFontIndirectW(&var_e0);
                    r13 = rax;
                    if (rax != 0) {
                        SelectObject(rdi, rax);
                        r14 = rax;
                        if (rax != 0) {
                            GetTextMetricsW(rdi, &var_98);
                        }
                    }
                }
            }
        }
    }
    if (r14 != 0) {
        SelectObject(rdi, r14);
        DeleteObject(r13);
    }
    r14d = 0;
    if (r15 != 0) {
        LocalUnlock(rbx);
    }
    if (r12d != 0) {
        EndPage(rdi);
        if (eax <= 0 /* signed */ && esi == 0) {
            GetLastError();
            esi = eax;
        }
    }
    if (var_38 != 0) {
        rcx = rdi;
        if (dword_ADDR == r14d) {
            EndDoc(rdi);
            if (eax <= 0 /* signed */ && esi == 0) {
                GetLastError();
                esi = eax;
            }
        } else {
            AbortDoc(rdi);
        }
    }
    DeleteDC(rdi);
    EnableWindow(qword_ADDR, 1);
    DestroyWindow(qword_ADDR);
    rcx = qword_ADDR;
    qword_ADDR = r14;
    SetCursor(qword_ADDR);
    eax = 0xADDR;
    eax = ((dword_ADDR == r14d) ? esi : eax);
    r11 = &var_220;
    return;
}
