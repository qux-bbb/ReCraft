// L1 structure recovery (89 blocks)
void sub_140010f48_l1(void) {
    var_340 = r8b;
    var_58 = edx;
    r14 = rcx;
    var_70 = rcx;
    esi = 0;
    edi = 0;
    var_78 = 0;
    var_54 = 0;
    r12d = esi;
    var_38 = esi;
    xmm0 = 0;
    eax = 0;
    var_2c0 = xmm0;
    var_2d0 = xmm0;
    var_2e0 = xmm0;
    var_2f0 = 0;
    var_80 = rsi;
    r15d = esi;
    sub_1400233ec(rcx);
    edx = 0;
    sub_1400029dc(&var_b0, 0, 0xADDR);
    ord_170(*(u64*)(r14));
    var_a4 = eax;
    PathFindExtensionW(*(u64*)(r14));
    var_a8 = rax;
    rcx = qword_ADDR;
    if (rcx == -1) {
        al = 0;
    } else {
        GetFileInformationByHandle(qword_ADDR, &var_2c0);
        ebx = eax;
        r13d = var_2e4;
        var_a0 = r13d;
        sub_140010d60(*(u64*)(r14), &var_b0);
        if (ebx == 0) {
            CloseHandle(qword_ADDR);
            qword_ADDR = -1;
        } else if (r13d >= 0xADDR) {
            var_20 = 0xADDR;
            sub_14000c3a4(qword_ADDR, qword_ADDR, qword_ADDR, *(u64*)(r14), 0xADDR);
        } else if (var_2e0 != esi) {
            var_20 = 0xADDR;
            sub_14000c3a4(qword_ADDR, qword_ADDR, qword_ADDR, *(u64*)(r14), 0xADDR);
            CloseHandle(qword_ADDR);
            qword_ADDR = -1;
            al = 0;
            r11 = &var_300;
            return;
        }
        SetCursor(qword_ADDR);
        sub_140012308(*(u64*)(r14));
        if (eax == 0) {
            al = 0;
            r11 = &var_300;
            return;
        }
        if (r13d == 0) {
            var_68 = &var_50;
            var_50 = si;
            ebx = 4;
        } else {
            r14d = esi;
            var_68 = rsi;
            var_28 = rsi;
            var_20 = r13d;
            CreateFileMappingW(qword_ADDR, 0, (rsi + 2), 0, r13d, rsi);
            rcx = rax;
            hObject = rax;
            ebx = (rsi + 4);
            if (rax != 0) {
                var_20 = r13;
                MapViewOfFile(rax, ebx, 0, 0, r13);
                r14 = rax;
                var_68 = rax;
                CloseHandle(hObject);
            }
        }
        CloseHandle(qword_ADDR);
        qword_ADDR = -1;
        if (r14 != 0) {
            rax = r14;
            var_40 = rax;
            ecx = var_58;
            if (ecx != 0) {
                if (ecx != 6) {
                    switch (ecx) {
                        case 0xADDR: {
                            ecx -= 0xADDR;
                            loc_ADDR:
                            var_30 = 1;
                            var_48 = esi;
                            r15d = esi;
                            loc_ADDR:
                            rax = var_40;
                            break;
                        }
                        case 0xADDR: {
                            ecx -= 0xADDR;
                            eax = 0xADDR;
                            var_4c = 1;
                            edi = r13d;
                            var_30 = 2;
                            edi >>= 1;
                            var_34 = edi;
                            var_38 = 1;
                            if (*(u16*)(r14) != ax) goto loc_ADDR;
                            break;
                        }
                        case 0xADDR: {
                            ecx -= 0xADDR;
                            ebx = 3;
                            eax = 0xADDR;
                            break;
                        }
                        case 0xADDR: {
                            ecx -= 0xADDR;
                            r15d = 0xADDR;
                            var_48 = 0xADDR;
                            var_30 = ebx;
                            if (r13d > 2) {
                                if (*(u16*)(r14) == 0xADDR && *(u8*)((r14 + 2)) == 0xADDR) {
                                    var_40 = (r14 + 3);
                                    r13d -= 0xADDR;
                                }
                            }
                            break;
                        }
                        default: {
                            ecx -= 0xADDR;
                            if (ecx != 1) goto loc_ADDR;
                            break;
                        }
                    }
                } else {
                    var_48 = 0xADDR;
                    ebx = ecx;
                    var_30 = ecx;
                }
            } else {
                eax = 0xADDR;
                if (*(u16*)(r14) == 0xADDR) {
                    if (r13d <= 2) {
                        var_58 = -1;
                        var_60 = esi;
                        IsTextUnicode(r14, r13d, &var_58);
                        r12d = eax;
                        var_38 = eax;
                        var_60 = eax;
                        if (eax != 0 && var_58 == 2) {
                            r12d = ((r13d < 0xADDR /* signed */) ? esi : r12d);
                            var_38 = r12d;
                            var_60 = r12d;
                        }
                        var_4c = r12d;
                        if (r12d == 0) goto loc_ADDR;
                    } else if (*(u8*)((r14 + 2)) == 0xADDR) {
                        var_48 = 0xADDR;
                        var_30 = ebx;
                        var_40 = (r14 + 3);
                        r13d -= 0xADDR;
                    }
                } else {
                    eax = 0xADDR;
                    if (*(u16*)(r14) == ax) goto loc_ADDR;
                }
            }
            if (r12d == 0) {
                var_28 = esi;
                var_20 = rsi;
                MultiByteToWideChar(r15d, 0, rax, r13d, rsi, esi);
                edi = eax;
                var_34 = eax;
            }
            r9d = 0;
            r12d = (r9 + 0xADDR);
            SendMessageW(qword_ADDR, r12d, 0, 0);
            SendMessageW(qword_ADDR, 0xADDR, 0, 0);
            SendMessageW(qword_ADDR, 0xADDR, 0, 0);
            edx = (rdi + 1);
            rdx += rdx;
            LocalReAlloc(qword_ADDR, (rdx + rdx), (r12 - 9));
            var_80 = rax;
            if (rax != 0) {
                LocalLock(rax);
                r12 = rax;
                var_78 = rax;
                if (var_38 == esi) {
                    var_28 = edi;
                    var_20 = r12;
                    MultiByteToWideChar(r15d, 0, var_40, r13d, r12, edi);
                    edi = eax;
                    var_34 = eax;
                } else {
                    rcx = r12;
                    if (*(u16*)(r14) != 0xADDR) {
                        if (*(u16*)(r14) != 0xADDR) {
                            rdx = r14;
                            r8d = edi;
                            r8 += r8;
                            sub_1400274ac();
                        } else {
                            sub_140011abc(r12, (r14 + 2), edi);
                        }
                    } else {
                        rdx = (r14 + 2);
                        r8d = edi;
                        r8 += r8;
                        sub_1400274ac();
                    }
                }
                dword_ADDR = ebx;
                var_2b8 = ebx;
                rbx = var_70;
                rax = &var_50;
                if (r14 != rax) {
                    UnmapViewOfFile(r14);
                }
                r13b = var_340;
                if (r12 == 0) {
                    r15d = var_54;
                } else {
                    ecx = edi;
                    *(u16*)((r12 + (rcx * 2))) = si;
                    if (r13b == 0) {
                        rax = r12;
                        if (edi != 0) {
                            do {
                                if (*(u16*)(rax) == si) {
                                    *(u16*)(rax) = 0xADDR;
                                }
                                rax += 2;
                                rcx -= 1;
                            } while (rcx != 0);
                        }
                        if (*(u16*)(r12) != 0xADDR) {
                            r15d = esi;
                        } else if (*(u16*)((r12 + 2)) == 0xADDR) {
                            if (*(u16*)((r12 + 4)) == 0xADDR && *(u16*)((r12 + 6)) == 0xADDR) {
                                r15d = 1;
                            }
                        }
                    }
                }
                r14 = var_80;
                LocalUnlock(r14);
                qword_ADDR = r14;
                SendMessageW(qword_ADDR, 0xADDR, 0, 0);
                sub_14000a0c0(off_140036148, rbx);
                sub_14000fe3c(rcx);
                dword_ADDR = esi;
                dword_ADDR = 1;
                SendMessageW(qword_ADDR, 0xADDR, qword_ADDR, 0);
                if (dword_ADDR != 2) {
                    dword_ADDR = esi;
                    PostMessageW(qword_ADDR, 0xADDR, 0, 0);
                    if (r15d != 0) {
                        SendMessageW(qword_ADDR, 0xADDR, edi, edi);
                        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
                        sub_140010580(1);
                    }
                    edx = 0xADDR;
                    SendMessageW(qword_ADDR, 0xADDR, (rdx - 0xADDR), 0);
                    r9d = 1;
                    SetScrollPos(qword_ADDR, r9d, eax, 1);
                    r9d = 0;
                    SendMessageW(qword_ADDR, (r9 + 0xADDR), (r9 + 1), 0);
                    edx = 0;
                    InvalidateRect(qword_ADDR, 0, (rdx + 1));
                    UpdateWindow(qword_ADDR);
                    sub_14000f9f0();
                    SetCursor(qword_ADDR);
                    if (r13b == 0) {
                        SHAddToRecentDocs(3, qword_ADDR);
                    }
                    sub_140023308(&var_a0);
                    al = 1;
                } else {
                    SetCursor(qword_ADDR);
                    dword_ADDR = esi;
                    var_20 = 0xADDR;
                    sub_14000c3a4(qword_ADDR, qword_ADDR, qword_ADDR, *(u64*)(rbx), 0xADDR);
                    ecx = 0;
                    sub_1400118a0(0);
                    r9d = 0;
                    SendMessageW(qword_ADDR, (r9 + 0xADDR), (r9 + 1), 0);
                    al = 0;
                    r11 = &var_300;
                    return;
                }
            } else {
                sub_14000bf34(&var_58, var_70);
                ecx = 0;
                sub_1400118a0(0);
                SetCursor(qword_ADDR);
                var_20 = 0xADDR;
                sub_14000c3a4(qword_ADDR, qword_ADDR, qword_ADDR, var_58, 0xADDR);
                rax = &var_50;
                if (r14 != rax) {
                    UnmapViewOfFile(r14);
                }
                SendMessageW(qword_ADDR, r12d, 0, 0);
                sub_14000a024(&var_58);
                al = 0;
            }
        } else {
            SetCursor(qword_ADDR);
            al = 0;
            r11 = &var_300;
            return;
        }
    }
    r11 = &var_300;
    return;
    loc_ADDR:
    var_4c = 1;
    var_38 = 1;
    var_30 = ebx;
    edi = (r13d >> 1);
    while (1) {
        edi -= 1;
        while (1) {
            var_34 = edi;
            goto loc_ADDR;
            loc_ADDR:
            ebx = 2;
            goto loc_ADDR;
            var_30 = 2;
            edi = (r13d >> 1);
        }
        loc_ADDR:
        sub_140023ae0(r14, r13d);
        if (eax == 0) goto loc_ADDR;
        var_30 = 5;
        var_48 = 0xADDR;
        goto loc_ADDR;
        var_48 = 0xADDR;
        ebx = (rcx + 4);
        var_30 = ebx;
        goto loc_ADDR;
    }
}
