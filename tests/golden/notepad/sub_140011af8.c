// L1 structure recovery (90 blocks)
void sub_140011af8_l1(void) {
    rax = qword_ADDR;
    r12d = 0;
    var_58 = rax;
    r15d = 0;
    var_54 = 0;
    r13 = r9;
    var_44 = 0;
    dil = r8b;
    var_40 = r8b;
    rsi = rdx;
    sub_1400235c8(rcx);
    ebx = (r12 + 1);
    if (dil != 0) {
        ebx = (r12 + 3);
        GetFileAttributesW(qword_ADDR);
        if (eax != -1 && ((eax >> 0xADDR) & 1) != 0) {
            var_20 = r12;
            DuplicateEncryptionInfoFile(qword_ADDR, *(u64*)(rsi), (r12 + 2), (rbx + 0xADDR), r12);
        }
    }
    PathFileExistsW(*(u64*)(rsi));
    var_30 = r12;
    var_28 = 0xADDR;
    r14d = eax;
    var_48 = eax;
    var_20 = 4;
    CreateFileW(*(u64*)(rsi), 0xADDR, ebx, 0, 4, 0xADDR, r12);
    qword_ADDR = rax;
    if (rax == -1) {
        al = 0;
    } else {
        GetLastError();
        ebx = eax;
        var_50 = eax;
        if (dword_ADDR != r12d) {
            if (dword_ADDR == r12d) {
                SendMessageW(qword_ADDR, 0xADDR, off_140036158, off_14003615c);
                SendMessageW(qword_ADDR, 0xADDR, 0, 0);
            } else {
                sub_14000cbc8(1);
            }
        }
        r9d = 0;
        SendMessageW(qword_ADDR, (r9 + 0xADDR), 0, 0);
        rdi = rax;
        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
        r12 = rax;
        if (rax == 0) {
            SetCursor(qword_ADDR);
            CloseHandle(qword_ADDR);
            qword_ADDR = (qword_ADDR | -1);
            if (r12 != 0) {
                LocalUnlock(r12);
            }
            if (ebx != 0xADDR) {
                DeleteFileW(*(u64*)(rsi));
            }
            if (dword_ADDR == 0) {
                al = 0;
                r11 = &var_490;
                return;
            }
            sub_140011a64();
        } else {
            LocalLock(rax);
            r8d = 0;
            rbx = rax;
            if (rax == 0) {
                ebx = var_50;
            } else {
                edx = dword_ADDR;
                ecx = (edx - 2);
                if (ecx == 0) {
                    var_20 = r8;
                    WriteFile(qword_ADDR, off_14003465c, 2, &var_44, r8);
                    var_20 &= r15;
                    WriteFile(qword_ADDR, rbx, (rdi + rdi), &var_44, (var_20 & r15));
                    loc_ADDR:
                    r8d = eax;
                    loc_ADDR:
                    if (r8d != 0) {
                        SetEndOfFile(qword_ADDR);
                        LocalUnlock(r12);
                        eax = dword_ADDR;
                        rcx = qword_ADDR;
                        dword_ADDR = eax;
                        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
                        r12d = 0;
                        dword_ADDR = 0;
                        if (dword_ADDR == 1 && r15d == 0) {
                            r8d = 0;
                            sub_140010f48(rsi, (r12 + 1), 0);
                        }
                        CloseHandle(qword_ADDR);
                        qword_ADDR = (qword_ADDR | -1);
                        edx = 0;
                        sub_1400029dc(&var_60, 0, 0xADDR);
                        r15d = (uint32)(var_40);
                        if (r15b != 0) {
                            ord_170(*(u64*)(rsi));
                            var_64 = eax;
                            PathFindExtensionW(*(u64*)(rsi));
                            var_68 = rax;
                            sub_140010eec(*(u64*)(rsi));
                            var_60 = eax;
                            sub_140010d60(*(u64*)(rsi), &var_70);
                            var_278 = dword_ADDR;
                        }
                        if (dword_ADDR != r12d) {
                            sub_140011a64();
                        }
                        edi = r12d;
                        if (word_ADDR == r12w) {
                            r14d = 1;
                        } else {
                            var_48 = r12;
                            if (r14d != 0) {
                                sub_140010a74(*(u64*)(rsi), &var_48);
                                rbx = var_48;
                                if (eax < 0 /* signed */) {
                                    r14d = 1;
                                } else {
                                    edx = 0;
                                    var_48 = r12d;
                                    sub_1400029dc(&var_280, 0, 0xADDR);
                                    sub_140026268(rbx, &var_280, r8, &var_48);
                                    if (eax >= 0 /* signed */ && var_48 != 2) {
                                        edi = 1;
                                    }
                                }
                                if (rbx != 0) {
                                    CoTaskMemFree(rbx);
                                }
                            } else {
                                r14d = 1;
                                edi = 1;
                            }
                        }
                        if (r13 != 0) {
                            var_48 = r12;
                            sub_140010a74(*(u64*)(rsi), &var_48);
                            rbx = var_48;
                            if (eax < 0 /* signed */) {
                                loc_ADDR:
                                if (rbx != 0) {
                                    CoTaskMemFree(rbx);
                                }
                                SetCursor(qword_ADDR);
                                sub_14000f9f0();
                                edx = r15d;
                                sub_14002346c(&var_60);
                                al = r14b;
                            } else {
                                edx = 0;
                                sub_1400029dc(&var_280, 0, 0xADDR);
                                rax = off_140036200;
                                edx = 0xADDR;
                                rcx = &var_280;
                                r13 = ((r13 == 0) ? rax : r13);
                                rax = &var_280;
                                r13 -= rax;
                                do {
                                    rax = (rdx + 0xADDR);
                                    if (rax == 0) break;
                                    eax = (uint32)(*(u16*)((rcx + r13)));
                                    if (ax == 0) break;
                                    *(u16*)(rcx) = ax;
                                    rcx += 2;
                                    rdx -= r14;
                                } while (rdx != 0);
                                rax = (rcx - 2);
                                rax = ((rdx != 0) ? rcx : rax);
                                *(u16*)(rax) = r12w;
                                if (rdx != 0) {
                                    GetFileAttributesW(rbx);
                                    if (eax == -1) goto loc_ADDR;
                                } else {
                                    if (rbx == 0) {
                                        al = 0;
                                        r11 = &var_490;
                                        return;
                                    }
                                    CoTaskMemFree(rbx);
                                    al = 0;
                                    r11 = &var_490;
                                    return;
                                }
                            }
                        } else if (edi == 0) {
                            SetCursor(qword_ADDR);
                            sub_14000f9f0();
                            edx = r15d;
                            sub_14002346c(&var_60);
                            al = r14b;
                            r11 = &var_490;
                            return;
                        }
                    }
                } else {
                    ecx -= 1;
                    if (ecx == 0) {
                        var_20 = r8;
                        WriteFile(qword_ADDR, off_140034658, 2, &var_44, r8);
                        r10d = 0;
                        r8 = rbx;
                        if (edi != 0) {
                            r9d = edi;
                            r14d = 0xADDR;
                            do {
                                ecx = (uint32)(*(u16*)(r8));
                                edx = (uint32)(*(u8*)((r8 + 1)));
                                ecx *= r14d;
                                dx += cx;
                                *(u16*)(r8) = dx;
                                r8 += 2;
                                r9 -= 1;
                            } while (r9 != 0);
                            r14d = var_48;
                        }
                        var_20 = r10;
                        WriteFile(qword_ADDR, rbx, (rdi + rdi), &var_44, r10);
                        r8d = eax;
                        if (edi == 0) goto loc_ADDR;
                    } else {
                        if (ecx == 1) {
                            var_20 = r8;
                            r8d = (rcx + 2);
                            WriteFile(qword_ADDR, off_140034660, (rcx + 2), &var_44, r8);
                            edx = dword_ADDR;
                            r8d = 0;
                        }
                        if (edx != 6) {
                            eax = (rdx - 4);
                            if (eax <= 1) {
                                r14d = 0xADDR;
                                r15d = r8d;
                                rax = r8;
                            } else {
                                GetACP();
                                r15d = 0xADDR;
                                r8d = 0;
                                r14d = eax;
                                rax = &var_54;
                            }
                        } else {
                            r14d = 0xADDR;
                            r15d = r8d;
                            rax = r8;
                        }
                        var_38 = rax;
                        var_30 = r8;
                        var_28 = r8d;
                        var_20 = r8;
                        WideCharToMultiByte(r14d, r15d, rbx, edi, r8, r8d, r8, rax);
                        ecx = 0;
                        if (var_54 != 0) {
                            var_20 = 0xADDR;
                            sub_14000c3a4(var_58, qword_ADDR, qword_ADDR, *(u64*)(rsi), 0xADDR);
                            if (eax != 2) {
                                r15d = 0;
                                var_20 = edi;
                                sub_140010c0c(rcx, r14d, r15d, rbx, edi);
                                r14d = var_48;
                                goto loc_ADDR;
                            }
                        }
                    }
                }
            }
        }
    }
    r11 = &var_490;
    return;
    do {
        ecx = (uint32)(*(u16*)(rbx));
        edx = (uint32)(*(u8*)((rbx + 1)));
        ecx *= r10d;
        dx += cx;
        *(u16*)(rbx) = dx;
        rbx += 2;
        r9 -= 1;
    } while (r9 != 0);
    goto loc_ADDR;
    if (((eax >> 0xADDR) & 1) == 0) goto loc_ADDR;
    DecryptFileW(rbx, 0);
    if (eax != 0) goto loc_ADDR;
    sub_140007cd0(var_4b8, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
    if (eax < 0 /* signed */) goto loc_ADDR;
    loc_ADDR:
    do {
        if (var_280 == r12w) {
            sub_14000c980(r14d);
            r11 = &var_490;
            return;
        }
        sub_14000f610(&var_280);
        if (eax == 0) {
            r8d = 0;
            sub_140010f48(rsi, dword_ADDR, 0);
        }
        word_ADDR = (word_ADDR + r14w);
        goto loc_ADDR;
        loc_ADDR:
        if (var_280 == r12w) {
            sub_14000c980(r14d);
            r11 = &var_490;
            return;
        }
        sub_140026438(rbx, &var_280);
    } while (eax >= 0 /* signed */);
    r8 = "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp";
    r9d = eax;
    edx = 0xADDR;
    sub_140007cf0(var_4b8);
    goto loc_ADDR;
    sub_14000c980(r14d);
}
