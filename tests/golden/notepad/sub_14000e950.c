// L1 structure recovery (111 blocks)
void sub_14000e950_l1(void) {
    r14 = r9;
    rdi = r8;
    ebp = edx;
    r15 = rcx;
    if (edx > 0xADDR) {
        if (ebp > 0xADDR) {
            eax = (ebp - 0xADDR);
            if (eax == 0) {
                ecx = *(u32*)((r14 + 0xADDR));
                r9d = *(u32*)((r9 + 4));
                ecx -= r9d;
                eax = *(u32*)((r14 + 8));
                eax -= *(u32*)(r14);
                var_30 = 0xADDR;
                var_28 = ecx;
                var_20 = eax;
                SetWindowPos(qword_ADDR, 0, *(u32*)(r14), *(u32*)((r9 + 4)), (eax - *(u32*)(r14)), (ecx - r9d), 0xADDR);
                r12d = 5;
                SendMessageW(qword_ADDR, 5, rdi, r14);
                GetDpiForWindow(qword_ADDR);
                MulDiv(dword_ADDR, eax, 0xADDR);
                eax = -(eax);
                dword_ADDR = eax;
                CreateFontIndirectW(off_140035650);
                rdi = rax;
                if (rax != 0) {
                    DeleteObject(qword_ADDR);
                    rcx = qword_ADDR;
                    qword_ADDR = rdi;
                    SendMessageW(qword_ADDR, (r12 + 0xADDR), rdi, (r12 - 4));
                }
                RedrawWindow(qword_ADDR, 0, 0, r12d);
                loc_ADDR:
                eax = 0;
            } else {
                eax -= 0xADDR;
                if (eax == 0) {
                    rax = (r14 >> 0xADDR);
                    r12d = 5;
                    ax &= 0xADDR;
                    if (ax != 0xADDR) {
                        if (r14 != qword_ADDR) {
                            loc_ADDR:
                            r8 = r14;
                            sub_14000da7c(r15, rdi);
                            if (eax != 0) {
                                eax = 0;
                                return;
                            }
                            r9 = r14;
                            r8 = rdi;
                            edx = ebp;
                            rcx = r15;
                            DefWindowProcW(rcx, rdx, r8, r9);
                        } else {
                            rax = (rdi >> 0xADDR);
                            if (ax != 0xADDR) {
                                ecx = 0xADDR;
                                ebx = 1;
                                ax -= 0xADDR;
                                if (ax <= 0xADDR) {
                                    if (dword_ADDR != ebx) {
                                        rdx = qword_ADDR;
                                        r9d = 0xADDR;
                                        MessageBoxW(qword_ADDR, rdx, qword_ADDR);
                                        eax = 0;
                                        return;
                                    }
                                    dword_ADDR = 2;
                                    eax = 0;
                                    return;
                                }
                            } else {
                                sub_14000d990();
                                if (al == byte_ADDR) goto loc_ADDR;
                            }
                        }
                    } else {
                        r8d = 0;
                        sub_14000da7c(r15, (r12 + 0xADDR));
                        eax = 0;
                        return;
                    }
                } else {
                    eax -= 0xADDR;
                    if (eax == 0) {
                        GetKeyboardLayout(0);
                        ax &= cx;
                        edx = 0xADDR;
                        ecx = (rsi + 0xADDR);
                        r8d = (rcx - 0xADDR);
                        sil = (uint8)((ax == cx));
                        r9 = 0;
                        rcx = qword_ADDR;
                        SendMessageW(rcx, rdx, r8, r9);
                        eax = 0;
                        return;
                    } else if (eax == 1) {
                        sub_14000a234((uint8)((r14 != 0)));
                        eax = 0;
                        return;
                    }
                    ebx = 1;
                    if (ebp != dword_ADDR) {
                        rcx = r15;
                        DefWindowProcW(rcx, rdx, r8, r9);
                        return;
                    }
                    ecx = *(u32*)((r9 + 0xADDR));
                    esi = 0;
                    al = cl;
                    al = ~(al);
                    al &= bl;
                    byte_ADDR = al;
                    eax = (ecx >> 2);
                    al &= bl;
                    byte_ADDR = al;
                    eax = (ecx >> 0xADDR);
                    al &= bl;
                    byte_ADDR = al;
                    if ((cl & 8) == 0) {
                        if ((cl & 0xADDR) == 0) {
                            if ((cl & 0xADDR) == 0) {
                                r8 = rsi;
                                if ((cl & 0xADDR) != 0) {
                                    sub_14000fcd0(off_14002b210, rdx);
                                    r8 = rsi;
                                    qword_ADDR = rsi;
                                }
                            } else {
                                sub_140021b08();
                                eax = -(eax);
                                r8 = ((eax != 0) ? -1 : 0);
                                r8d &= 0xADDR;
                            }
                        } else {
                            SetCursor(qword_ADDR);
                            sub_140021cd0(ebx);
                            edi = eax;
                            sub_14001fe40(rcx, byte_ADDR);
                            ebx = (uint32)(al);
                            SetCursor(qword_ADDR);
                            r8d = ebx;
                            r8 <<= 3;
                            edi = -(edi);
                            rax = ((edi != 0) ? -1 : 0);
                            eax &= 0xADDR;
                            r8 |= rax;
                        }
                    } else {
                        SetCursor(qword_ADDR);
                        sub_14001fe40(rcx, byte_ADDR);
                        ebx = (uint32)(al);
                        SetCursor(qword_ADDR);
                        r8d = ebx;
                        r8 <<= 3;
                    }
                    rcx = qword_ADDR;
                    if (rcx == 0) {
                        eax = 0;
                        return;
                    }
                    r9 = r14;
                    edx = 0xADDR;
                    SendMessageW(rcx, rdx, r8, r9);
                    eax = 0;
                    return;
                }
            }
        } else if (ebp == eax) {
            sub_14001008c(rdi, r15);
            eax = 0;
            return;
        }
        eax = (ebp - 0xADDR);
        if (eax == 0) {
            if (dword_ADDR == 0) {
                DefWindowProcW(r15, rdx, r8, r9);
                eax = 0;
                return;
            }
            rcx = rdi;
            ecx &= 0xADDR;
            rax = (rcx - 0xADDR);
            if ((rax & -0xADDR) != 0) {
                DefWindowProcW(r15, rdx, r8, r9);
                eax = 0;
                return;
            }
            if (rcx != 0xADDR) {
                eax = 0;
                return;
            }
            DefWindowProcW(r15, rdx, r8, r9);
            eax = 0;
            return;
        }
        switch (eax) {
            case 0xADDR: {
                eax -= 0xADDR;
                sub_14000f2a0(rcx);
                eax = 0;
                return;
            }
            case 0xADDR: {
                eax -= 0xADDR;
                esi = 0;
                if (dword_ADDR == 0) {
                    eax = 0;
                    return;
                }
                r14 >>= 0xADDR;
                if (r14w == 0) {
                    eax = 0;
                    return;
                }
                EnableMenuItem(qword_ADDR, 0xADDR, (rsi + 3));
                eax = 0;
                return;
            }
            case 0xADDR: {
                eax -= 0xADDR;
                DefWindowProcW(rcx, rdx, r8, r9);
                rbx = rax;
                sub_14000fd5c();
                rax = rbx;
                break;
            }
            case 0xADDR: {
                eax -= 0xADDR;
                DefWindowProcW(rcx, rdx, r8, r9);
                rbx = rax;
                sub_14000fd5c();
                rax = rbx;
                return;
            }
            default: {
                eax -= 0xADDR;
                break;
            }
        }
    } else if (edx != eax) {
        if (edx > 0xADDR) {
            ebx = 1;
            ecx = (rbx + 0xADDR);
            if (ebp == ecx) {
                if (dword_ADDR == 0) {
                    sub_14000d990();
                    if (al == 0) {
                        rax = 1;
                        return;
                    }
                    rcx = qword_ADDR;
                    if (rcx == 0) {
                        sub_14000c520();
                        rax = (int64)(eax);
                    } else if ((bl & r14b) == 0) {
                        sub_14000c520();
                        rax = (int64)(eax);
                        return;
                    }
                    sub_140022440(rcx);
                    if (al != 0) {
                        rax = rbx;
                        return;
                    }
                    sub_14000c520();
                    rax = (int64)(eax);
                    return;
                }
                MessageBeep(0);
                MessageBeep(0);
                rdx = qword_ADDR;
                r9d = 0xADDR;
                MessageBoxW(qword_ADDR, rdx, qword_ADDR);
                eax = 0;
                return;
            } else if (ebp == 0xADDR) {
                if (rdi == 0) {
                    eax = 0;
                    return;
                }
                rax = qword_ADDR;
                if ((rax == 0) || ((bl & r14b) == 0)) {
                    eax = 0;
                    return;
                }
                *(u8*)((rax + 0xADDR)) = bl;
                sub_1400225a4(qword_ADDR);
                return;
            } else if (ebp == 0xADDR) {
                GetDpiForWindow(qword_ADDR);
                MulDiv(0xADDR, eax, 0xADDR);
                *(u32*)((r14 + 0xADDR)) = 0;
                eax = 0;
                return;
            } else if (ebp == 0xADDR) {
                rcx = qword_ADDR;
                esi = 0;
                if (rcx == 0) {
                    eax = 0;
                    return;
                }
                GetDlgCtrlID(qword_ADDR);
                if (*(u32*)((r14 + 8)) != eax) {
                    eax = 0;
                    return;
                } else if (*(u32*)((r14 + 0xADDR)) != 0xADDR) {
                    if (*(u32*)((r14 + 0xADDR)) != 0xADDR) {
                        eax = 0;
                        return;
                    }
                    edx = *(u32*)((r14 + 0xADDR));
                    sub_140023890(*(u32*)((r14 + 0xADDR)));
                    eax = 0;
                    return;
                }
                if (dword_ADDR != esi) {
                    rax = rbx;
                    return;
                }
                edx = 0;
                sub_1400029dc(&var_50, 0, 0xADDR);
                sub_140025fb8(&var_50);
                if ((eax < 0 /* signed */) || (var_50 == si)) {
                    rax = rbx;
                    return;
                }
                sub_14000f610(&var_50);
                word_ADDR += bx;
            }
        } else if (edx == 0xADDR) {
            sub_14001eee0(rcx);
            sub_14000d990();
            esi = 0;
            if (al == 0) {
                sub_140023d18(qword_ADDR);
                DestroyWindow(qword_ADDR);
                DestroyWindow(qword_ADDR);
                DeleteObject(qword_ADDR);
                eax = 0;
                return;
            }
            rcx = qword_ADDR;
            if (rcx == 0) goto loc_ADDR;
        } else {
            eax = (edx - 2);
            if (eax == 0) {
                PostQuitMessage(0);
                eax = 0;
                return;
            }
            eax -= 3;
            if (eax == 0) {
                if (rdi == 0) {
                    sub_14000e848((int32)(r14w), (int32)(var_42));
                    eax = 0;
                    return;
                }
                rdi -= 1;
                if (rdi == 0) {
                    r8d = 1;
                } else if (rdi != 1) {
                    eax = 0;
                    return;
                }
            } else {
                switch (eax) {
                    case 0xADDR: {
                        eax -= 0xADDR;
                        di -= 0xADDR;
                        if (di > 0xADDR) {
                            eax = 0;
                            return;
                        }
                        IsIconic(qword_ADDR);
                        if (eax != 0) {
                            eax = 0;
                            return;
                        }
                        GetForegroundWindow();
                        if (rax != qword_ADDR) {
                            eax = 0;
                            return;
                        }
                        GetForegroundWindow();
                        rcx = rax;
                        SetFocus(rcx);
                        eax = 0;
                        return;
                    }
                    case 0xADDR: {
                        eax -= 0xADDR;
                        IsIconic(qword_ADDR);
                        if (eax != 0) {
                            eax = 0;
                            return;
                        }
                        rcx = qword_ADDR;
                        break;
                    }
                    case 0xADDR: {
                        eax -= 0xADDR;
                        rcx = qword_ADDR;
                        SendMessageW(rcx, rdx, r8, r9);
                        eax = 0;
                        return;
                    }
                    default: {
                        eax -= 0xADDR;
                        break;
                    }
                }
            }
        }
    }
    return;
    loc_ADDR:
    sub_14000c520();
    if (eax == 0) goto loc_ADDR;
    sub_140023d18(qword_ADDR);
    DestroyWindow(qword_ADDR);
    DestroyWindow(qword_ADDR);
    DeleteObject(qword_ADDR);
    goto loc_ADDR;
    sub_14000fe3c(rcx);
    rdx = qword_ADDR;
    r9d = 0xADDR;
}
