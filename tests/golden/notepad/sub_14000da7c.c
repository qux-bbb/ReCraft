// L1 structure recovery (161 blocks)
void sub_14000da7c_l1(void) {
    rbp = &var_100;
    esi = 0;
    edi = (uint32)(dx);
    r14 = rdx;
    xmm0 = 0;
    edx = 0xADDR;
    ebx = 0;
    var_40 = 0;
    r15 = rcx;
    r12d = (rsi + 1);
    rect = xmm0;
    r8d = (rdx + 1);
    if (edi > 0xADDR) {
        ecx = edi;
        ecx -= r8d;
        if (ecx == 0) {
            if (di != dx) {
                if (di != r8w) {
                    word_ADDR = (word_ADDR + r12w);
                } else {
                    word_ADDR = (word_ADDR + r12w);
                }
            } else {
                word_ADDR = (word_ADDR + r12w);
            }
            var_48 = esi;
            var_40 = esi;
            SendMessageW(qword_ADDR, 0xADDR, &var_48, &var_40);
            eax = var_40;
            if (var_48 != eax) {
                ecx = 0xADDR;
                rax = ((di != cx) ? r14 : rax);
                r14 = rax;
                eax = 0xADDR;
                if (r14w == 0xADDR) {
                    word_ADDR = (word_ADDR + r12w);
                    if (dword_ADDR == esi) {
                        edx = 0;
                        sub_1400029dc(rbp, 0, 0xADDR);
                        sub_140025fb8(rbp);
                        if (eax >= 0 /* signed */ && var_100 != si) {
                            sub_14000f610(rbp);
                            word_ADDR = (word_ADDR + r12w);
                        }
                    }
                }
                GetFocus();
                rcx = qword_ADDR;
                if (rax == rcx) {
                    PostMessageW(qword_ADDR, (uint32)(r14w), 0, 0);
                    loc_ADDR:
                    if (rbx != 0) {
                        rcx = rbx;
                        CoTaskMemFree(rcx);
                    }
                }
            }
            eax = r12d;
        } else {
            ecx -= r12d;
            if (ecx != 0 && ecx != r12d) {
                eax = 0;
            }
        }
    } else if (edi != edx) {
        if (edi > 0xADDR) {
            if (edi > 0xADDR) {
                edi -= 0xADDR;
                if (edi == 0) {
                    var_48 = esi;
                    var_40 = esi;
                    SendMessageW(qword_ADDR, 0xADDR, &var_48, &var_40);
                    eax = var_48;
                    if (eax == 0) {
                        ecx = r12d;
                        eax = r12d;
                        var_40 = ecx;
                        var_48 = eax;
                    } else {
                        ecx = var_40;
                    }
                    eax *= 0xADDR;
                    edx = 0;
                    __asm("div ecx");
                    ecx = 0xADDR;
                    eax += 0xADDR;
                    eax = ((eax >= ecx) ? ecx : eax);
                    r8d = eax;
                    r9d = 0xADDR;
                    SendMessageW(qword_ADDR, 0xADDR, r8, r9);
                    sub_14000fd5c();
                    eax = r12d;
                    r11 = &var_310;
                    return;
                }
                edi -= r12d;
                if (edi == 0) {
                    var_40 = esi;
                    var_48 = esi;
                    SendMessageW(qword_ADDR, 0xADDR, &var_40, &var_48);
                    eax = var_40;
                    if (eax == 0) {
                        ecx = r12d;
                        eax = r12d;
                        var_48 = ecx;
                        var_40 = eax;
                    } else {
                        ecx = var_48;
                    }
                    eax *= 0xADDR;
                    edx = 0;
                    __asm("div ecx");
                    ecx = 0xADDR;
                    eax -= 0xADDR;
                    ecx = ((eax > ecx) ? eax : ecx);
                    r8d = ecx;
                } else {
                    edi -= r12d;
                    if (edi == 0) {
                        r9d = 0xADDR;
                        r8d = 0xADDR;
                    } else {
                        edi -= 0xADDR;
                        if (edi == 0) {
                            sub_14001fd94("https://go.microsoft.com/fwlink/?LinkId=834783");
                            sub_140023a54(rcx);
                            eax = r12d;
                            r11 = &var_310;
                            return;
                        }
                        edi -= r12d;
                        if (edi == 0) {
                            LoadIconW(qword_ADDR, 2);
                            ShellAboutW(qword_ADDR, qword_ADDR, off_14002b210, rax);
                            eax = r12d;
                            r11 = &var_310;
                            return;
                        } else if (edi != r12d) {
                            eax = 0;
                            r11 = &var_310;
                            return;
                        }
                        sub_14001fd94("feedback-hub://?tabid=2&contextid=1010");
                        eax = r12d;
                        r11 = &var_310;
                        return;
                    }
                }
            } else if (edi == 0xADDR) {
                word_ADDR = (word_ADDR + r12w);
                edi = 0xADDR;
                edx = 0;
                r8d = 0xADDR;
                sub_1400029dc(&var_70);
                GetDC(0);
                rbx = rax;
                if (rax == 0) {
                    eax = r12d;
                    r11 = &var_310;
                    return;
                }
                r14 = off_140035650;
                var_70 = edi;
                var_88 = off_140035650;
                var_78 = r15;
                GetDeviceCaps(rax, (rdi - 0xADDR));
                r15d = 0xADDR;
                MulDiv(dword_ADDR, eax, r15d);
                var_94 = 0xADDR;
                eax = -(eax);
                dword_ADDR = eax;
                var_c8 = ax;
                ReleaseDC(0, rbx);
                SetThreadDpiAwarenessContext((rdi - 0xADDR));
                rdi = 0xADDR;
                ChooseFontW(&var_70);
                ebx = 0xADDR;
                SetThreadDpiAwarenessContext(rdi);
                if (ebx == 0) {
                    eax = r12d;
                    r11 = &var_310;
                    return;
                }
                SetCursor(qword_ADDR);
                GetDpiForWindow(qword_ADDR);
                MulDiv(var_90, eax, r15d);
                eax = -(eax);
                dword_ADDR = eax;
                CreateFontIndirectW(r14);
                rbx = rax;
                if (rax != 0) {
                    DeleteObject(qword_ADDR);
                    rcx = qword_ADDR;
                    qword_ADDR = rbx;
                    SendMessageW(qword_ADDR, 0xADDR, rbx, r12);
                    dword_ADDR = var_90;
                }
                SetCursor(qword_ADDR);
                dword_ADDR = r12d;
                eax = r12d;
                r11 = &var_310;
                return;
            }
            ecx = (edi - 0xADDR);
            if (ecx == 0) {
                r9 = -1;
                SendMessageW(qword_ADDR, 0xADDR, 0, (r9 | -1));
                edx = 0xADDR;
                SendMessageW(qword_ADDR, rdx, 0, 0);
                eax = r12d;
                r11 = &var_310;
                return;
            }
            ecx -= r12d;
            if (ecx == 0) {
                ecx = 0;
                sub_140010580(0);
                eax = r12d;
                r11 = &var_310;
                return;
            }
            ecx -= r12d;
            if (ecx == 0) {
                GetClientRect(qword_ADDR, &rect);
                if (dword_ADDR == esi) {
                    edx = rect.bottom;
                    ecx = rect.right;
                    edx -= rect.top;
                    ecx -= rect.left;
                    dword_ADDR = r12d;
                    sub_14000e848(rcx, rdx);
                    sub_14000f9f0();
                    ShowWindow(qword_ADDR, 5);
                } else {
                    rcx = qword_ADDR;
                    dword_ADDR = esi;
                    ShowWindow(qword_ADDR, 0);
                    edx = rect.bottom;
                    ecx = rect.right;
                    edx -= rect.top;
                    ecx -= rect.left;
                    sub_14000e848(rcx, rdx);
                }
                dword_ADDR = r12d;
                eax = r12d;
                r11 = &var_310;
                return;
            }
            ecx -= r12d;
            if (ecx != 0) {
                ecx -= r12d;
                if (ecx == 0) {
                    if (dword_ADDR == esi) {
                        dword_ADDR = r12d;
                        if (edi == 0xADDR) {
                            word_ADDR = (word_ADDR + r12w);
                        }
                        rcx = qword_ADDR;
                        if (rcx != 0) {
                            r9d = 0;
                            SendMessageW(qword_ADDR, (r9 + 0xADDR), 0, 0);
                        }
                        al = byte_ADDR;
                        al = byte_ADDR;
                        edx = (((al != 0) ? -1 : 0) + 0xADDR);
                        al = byte_ADDR;
                        ecx = ((al != 0) ? 4 : 0);
                        edx |= ecx;
                        al = -(al);
                        ecx = ((al != 0) ? 0xADDR : 0);
                        edx |= ecx;
                        dword_ADDR = edx;
                        sub_14001f948(rcx, rdx);
                        rcx = off_140036540;
                        qword_ADDR = rsi;
                        qword_ADDR = off_140036540;
                        dword_ADDR = 0xADDR;
                        FindTextW(off_1400355d0);
                        if (rax == 0) {
                            edx = 0;
                            sub_140023e28(off_1400355d0, rdx);
                        }
                        qword_ADDR = rax;
                        eax = r12d;
                        r11 = &var_310;
                        return;
                    }
                } else if (ecx != 3) {
                    eax = 0;
                    r11 = &var_310;
                    return;
                }
                eax = dword_ADDR;
                eax = -(eax);
                ecx = (((eax != 0) ? 0xADDR : 0) + 0xADDR);
                sub_14001fa28(rcx);
                if (eax == 0) {
                    MessageBoxW(qword_ADDR, qword_ADDR, qword_ADDR, 0xADDR);
                    eax = r12d;
                    r11 = &var_310;
                    return;
                }
                sil = (uint8)((dword_ADDR == esi));
                dword_ADDR = esi;
            }
        } else if (edi == 0xADDR) {
            r8 = qword_ADDR;
            rcx = qword_ADDR;
            word_ADDR = (word_ADDR + r12w);
            var_20 = rsi;
            DialogBoxParamW(qword_ADDR, 0xADDR, qword_ADDR, sub_14000cc40, rsi);
            if (rax != 0) {
                eax = r12d;
                r11 = &var_310;
                return;
            }
            sub_14000cbc8(dword_ADDR);
            eax = r12d;
            r11 = &var_310;
            return;
        } else if (edi > 7) {
            ecx = (edi - 8);
            if (ecx == 0) {
                rax = &var_48;
                var_50 = rsi;
                var_60 = rax;
                rax = &var_50;
                var_48 = rsi;
                var_68 = rax;
                xmm0 = var_60;
                var_78 = off_1400291b8;
                rax = &var_78;
                var_e0 = rax;
                var_80 = xmm0;
                sub_14000bb98(&var_40, &var_70);
                ebx = eax;
                if (eax >= 0 /* signed */) {
                    rbx = var_40;
                    var_28 = 5;
                    var_20 = rsi;
                    ShellExecuteW(0, 0, rbx, off_14002b210, rsi, 5);
                    goto loc_ADDR;
                }
                r8 = "shell\\osshell\\accesory\\notepad\\notepad.cpp";
                r9d = eax;
                edx = 0xADDR;
                sub_140007cf0(var_338);
                rcx = var_40;
                if (rcx != 0) {
                    CoTaskMemFree(var_40);
                }
                eax = ebx;
            } else {
                ecx -= 7;
                if (ecx == 0) {
                    eax = r12d;
                    r11 = &var_310;
                    return;
                }
                ecx -= r12d;
                if (ecx == 0) {
                    word_ADDR = (word_ADDR + r12w);
                    edx = 0xADDR;
                } else {
                    ecx -= 5;
                    if (ecx == 0) {
                        dword_ADDR = r12d;
                        if (edi == 0xADDR) {
                            word_ADDR = (word_ADDR + r12w);
                        }
                        rcx = qword_ADDR;
                        if (rcx != 0) {
                            r9d = 0;
                            SendMessageW(qword_ADDR, (r9 + 0xADDR), 0, 0);
                        }
                        al = byte_ADDR;
                        al = byte_ADDR;
                        edx = (((al != 0) ? -1 : 0) + 0xADDR);
                        al = byte_ADDR;
                        ecx = ((al != 0) ? 4 : 0);
                        edx |= ecx;
                        al = -(al);
                        ecx = ((al != 0) ? 0xADDR : 0);
                        edx |= ecx;
                        dword_ADDR = edx;
                        sub_14001f948(rcx, rdx);
                        qword_ADDR = rsi;
                        qword_ADDR = off_140036540;
                        dword_ADDR = 0xADDR;
                        FindTextW(off_1400355d0);
                        if (rax == 0) {
                            edx = 0;
                            sub_140023e28(off_1400355d0, rdx);
                        }
                        qword_ADDR = rax;
                        eax = r12d;
                        r11 = &var_310;
                        return;
                    }
                    ecx -= r12d;
                    if (ecx != 0) {
                        if (ecx != r12d) {
                            eax = 0;
                            r11 = &var_310;
                            return;
                        }
                        word_ADDR = (word_ADDR + r12w);
                        rcx = qword_ADDR;
                        dword_ADDR = r12d;
                        if (rcx != 0) {
                            r9d = 0;
                            SendMessageW(qword_ADDR, (r9 + 0xADDR), 0, 0);
                        }
                        al = byte_ADDR;
                        al = byte_ADDR;
                        edx = (((al != 0) ? 4 : 0) + 0xADDR);
                        al = -(al);
                        ecx = ((al != 0) ? 0xADDR : 0);
                        edx |= ecx;
                        dword_ADDR = edx;
                        sub_14001f948(rcx, rdx);
                        rcx = off_140036540;
                        dword_ADDR = 0xADDR;
                        qword_ADDR = off_140036540;
                        rax = 0xADDR;
                        qword_ADDR = 0xADDR;
                        ReplaceTextW(off_1400355d0);
                        if (rax != 0) {
                            qword_ADDR = rax;
                            eax = r12d;
                            r11 = &var_310;
                            return;
                        }
                        edx = r12d;
                    }
                }
            }
        } else if (edi == 7) {
            r9d = 0;
            PostMessageW(rcx, (r9 + 0xADDR), 0, 0);
            eax = r12d;
            r11 = &var_310;
            return;
        }
        ecx = edi;
        ecx -= r12d;
        if (ecx == 0) {
            word_ADDR = (word_ADDR + r12w);
            sub_1400118a0(r12d);
            eax = r12d;
            r11 = &var_310;
            return;
        }
        ecx -= r12d;
        if (ecx == 0) {
            sub_14000c520();
            if (eax == 0) {
                eax = r12d;
                r11 = &var_310;
                return;
            }
            r14d = dword_ADDR;
            var_40 = rsi;
            sub_14000d54c(r15, &var_40);
            if (eax >= 0 /* signed */) {
                r9d = 0;
                r15 = qword_ADDR;
                var_30 = rsi;
                var_28 = 0xADDR;
                var_20 = 3;
                CreateFileW(var_40, 0xADDR, (r9 + 3), 0, 3, 0xADDR, rsi);
                rcx = qword_ADDR;
                ebx = dword_ADDR;
                edi = dword_ADDR;
                qword_ADDR = rax;
                sub_140010eec(qword_ADDR);
                sub_140022fe8(0xADDR, eax, edi, ebx);
                sub_14000d9f0();
                sub_140023648(eax);
                r8d = 0;
                sub_140010f48(&var_40, dword_ADDR, 0);
                if (al == 0) {
                    sub_140010b30(var_40);
                    qword_ADDR = r15;
                    dword_ADDR = r14d;
                }
            } else if (eax == 0xADDR) {
                dword_ADDR = r14d;
                if (eax == 0xADDR) {
                    MessageBoxW(qword_ADDR, qword_ADDR, qword_ADDR, 0xADDR);
                }
            } else {
                sub_14000d078(&var_40);
            }
            rcx = var_40;
            if (rcx == 0) {
                eax = r12d;
                r11 = &var_310;
                return;
            }
            CoTaskMemFree(rcx);
            eax = r12d;
            r11 = &var_310;
            return;
        }
        ecx -= r12d;
        if (ecx == 0) {
            word_ADDR = (word_ADDR + r12w);
            eax = dword_ADDR;
            dword_ADDR = eax;
            if (dword_ADDR != esi) {
                eax = 4;
                if (di == 0xADDR) {
                    word_ADDR = (word_ADDR + r12w);
                }
                rbx = "*.txt";
                rdx = rbx;
                var_48 = rsi;
                rdx = ((dword_ADDR == esi) ? qword_ADDR : rdx);
                var_40 = rsi;
                dil = sil;
                sub_14000d6a8(r15, rdx, &var_40, &var_48);
                if (eax >= 0 /* signed */) {
                    edx = 0;
                    sub_1400029dc(rbp, 0, 0xADDR);
                    rcx = var_48;
                    if (rcx == 0) {
                        r9 = rsi;
                    } else {
                        sub_140026734(var_48, rbp, r8);
                        if (eax >= 0 /* signed */ && var_100 != si) {
                            r9 = rbp;
                        }
                    }
                    sub_140011af8(rcx, &var_40, r12b, r9);
                    if (al == 0) {
                        sub_140010b30(var_40);
                    } else {
                        sub_14000a0c0(off_140036148, &var_40);
                        sub_14000fe3c(rcx);
                    }
                } else {
                    if (eax != 0xADDR) {
                        rbx = ((dword_ADDR == esi) ? qword_ADDR : rbx);
                        sub_14000d2e8(rbx, &var_40);
                        dil = r12b;
                    }
                    if (eax < 0 /* signed */ && eax == 0xADDR) {
                        MessageBoxW(qword_ADDR, qword_ADDR, qword_ADDR, 0xADDR);
                    }
                }
                r11 = &var_310;
                return;
            }
        } else {
            ecx -= r12d;
            if (ecx == 0) {
                eax = 4;
                if (di == 0xADDR) {
                    word_ADDR = (word_ADDR + r12w);
                }
                rbx = "*.txt";
                rdx = rbx;
                var_48 = rsi;
                rdx = ((dword_ADDR == esi) ? qword_ADDR : rdx);
                var_40 = rsi;
                dil = sil;
                sub_14000d6a8(r15, rdx, &var_40, &var_48);
                if (eax >= 0 /* signed */) {
                    edx = 0;
                    sub_1400029dc(rbp, 0, 0xADDR);
                    rcx = var_48;
                    if (rcx == 0) {
                        r9 = rsi;
                    } else {
                        sub_140026734(var_48, rbp, r8);
                        if (eax >= 0 /* signed */ && var_100 != si) {
                            r9 = rbp;
                        }
                    }
                    sub_140011af8(rcx, &var_40, r12b, r9);
                    if (al == 0) {
                        sub_140010b30(var_40);
                    } else {
                        sub_14000a0c0(off_140036148, &var_40);
                        sub_14000fe3c(rcx);
                    }
                } else {
                    if (eax != 0xADDR) {
                        rbx = ((dword_ADDR == esi) ? qword_ADDR : rbx);
                        sub_14000d2e8(rbx, &var_40);
                        dil = r12b;
                    }
                    if (eax < 0 /* signed */ && eax == 0xADDR) {
                        MessageBoxW(qword_ADDR, qword_ADDR, qword_ADDR, 0xADDR);
                    }
                }
                rcx = var_40;
                if (rcx != 0) {
                    CoTaskMemFree(var_40);
                }
                rcx = var_48;
                if (rcx == 0) {
                    eax = r12d;
                    r11 = &var_310;
                    return;
                }
                var_48 = rsi;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010();
                eax = r12d;
                r11 = &var_310;
                return;
            }
            ecx -= r12d;
            if (ecx == 0) {
                while (1) {
                    sub_14000f5b8(0xADDR);
                    if (eax != 0) break;
                    CommDlgExtendedError();
                    ecx = eax;
                    eax += 0xADDR;
                    if (((eax & 0xADDR) != 0) || (ecx == 0xADDR)) {
                        sub_14000f91c();
                        eax = r12d;
                        r11 = &var_310;
                        return;
                    }
                    sub_14000cb7c();
                    qword_ADDR = rsi;
                    qword_ADDR = rsi;
                }
                edx = 0xADDR;
                rcx = 0xADDR;
                r9 = 0xADDR;
                r8d = 0xADDR;
                r9 -= 0xADDR;
                do {
                    rax = (r8 + 0xADDR);
                    if (rax == 0) break;
                    eax = (uint32)(*(u16*)((r9 + rcx)));
                    if (ax == 0) break;
                    *(u16*)(rcx) = ax;
                    rcx += 2;
                    r8 -= r12;
                } while (r8 != 0);
                rax = (rcx - 2);
                r8 = 0xADDR;
                rax = ((r8 != 0) ? rcx : rax);
                rcx = 0xADDR;
                r8 -= 0xADDR;
                *(u16*)(rax) = si;
                do {
                    rax = (rdx + 0xADDR);
                    if (rax == 0) break;
                    eax = (uint32)(*(u16*)((r8 + rcx)));
                    if (ax == 0) break;
                    *(u16*)(rcx) = ax;
                    rcx += 2;
                    rdx -= r12;
                } while (rdx != 0);
                dword_ADDR = r12d;
                rax = (rcx - 2);
                rax = ((rdx != 0) ? rcx : rax);
                *(u16*)(rax) = si;
                if (word_ADDR != si) {
                    esi = r12d;
                } else if (word_ADDR != si) {
                    esi = r12d;
                    sub_140023804(esi);
                    eax = r12d;
                    r11 = &var_310;
                    return;
                }
                sub_140023804(esi);
                eax = r12d;
                r11 = &var_310;
                return;
            } else if (ecx != r12d) {
                eax = 0;
                r11 = &var_310;
                return;
            }
            word_ADDR = (word_ADDR + r12w);
            SetThreadDpiAwarenessContext((rsi - 2));
            ecx = 0;
            rbx = rax;
            sub_140021324(0);
            SetThreadDpiAwarenessContext(rbx);
            eax = r12d;
            r11 = &var_310;
            return;
        }
    }
    r11 = &var_310;
    return;
}
