// L1 structure recovery (47 blocks)
void sub_140023f90_l1(void) {
    ebx = edx;
    rsi = r9;
    r14 = r8;
    rdi = rcx;
    ebp = 0;
    GetPropW();
    r15 = rax;
    if (rax == 0) {
        rax = qword_ADDR;
        if (rax == 0) {
            switch (ebx) {
                case 0xADDR: {
                    ebx -= 0xADDR;
                    eax = 0;
                    break;
                }
                case 0xADDR: {
                    ebx -= 0xADDR;
                    sub_140024aec(rdi, rbp, 8, *(u32*)(r15));
                    r9d = 0;
                    SendMessageW(rdi, 0xADDR, (r9 + 2), 0);
                    eax = 1;
                    break;
                }
                case 0xADDR: {
                    ebx -= 0xADDR;
                    SetPropW(rdi, 0xADDR, rsi);
                    rbx = *(u64*)((rsi + 8));
                    r8d = *(u32*)(rsi);
                    qword_ADDR = 0;
                    sub_140024500(rdi, rbx, *(u32*)(rsi));
                    esi = 1;
                    if ((*(u32*)((rbx + 0xADDR)) & 0xADDR) == 0) {
                        rbx = rsi;
                        ShowWindow(rdi, esi);
                        UpdateWindow(rdi);
                    } else {
                        rax = *(u64*)((rbx + 0xADDR));
                        r9 = rbx;
                        r8 = r14;
                        sub_140028010(rdi, 0xADDR);
                        rbx = rax;
                        if (rax != 0) {
                            ShowWindow(rdi, esi);
                            UpdateWindow(rdi);
                            rax = rbx;
                            return;
                        }
                    }
                    rax = rbx;
                    break;
                }
                case 0xADDR: {
                    ebx -= 0xADDR;
                    if (r15 == 0) {
                        eax = 0;
                        return;
                    } else if (rbp == 0) {
                        eax = 0;
                        return;
                    }
                    ecx = (uint32)(r14w);
                    esi = 1;
                    ecx -= 1;
                    if (ecx == 0) {
                        sub_140024aec(rdi, rbp, 8, *(u32*)(r15));
                        edx = dword_ADDR;
                        r8d = 0;
                        rcx = *(u64*)((rbp + 8));
                        SendMessageW(*(u64*)((rbp + 8)), rdx, r8, rbp);
                        rax = rsi;
                    } else {
                        ecx -= esi;
                        if (ecx == 0) {
                            eax = ((*(u32*)((rbp + 0xADDR)) & -0xADDR) | 0xADDR);
                            *(u32*)((rbp + 0xADDR)) = eax;
                            SendMessageW(*(u64*)((rbp + 8)), dword_ADDR, 0, rbp);
                            RemovePropW(rdi, 0xADDR);
                            DestroyWindow(rdi);
                            LocalFree(r15);
                            rax = rsi;
                            return;
                        }
                        ecx -= esi;
                        if (ecx == 0) {
                            eax = ((*(u32*)((rbp + 0xADDR)) & -0xADDR) | 0xADDR);
                            *(u32*)((rbp + 0xADDR)) = eax;
                            SendMessageW(*(u64*)((rbp + 8)), dword_ADDR, 0, rbp);
                            RemovePropW(rdi, 0xADDR);
                            DestroyWindow(rdi);
                            LocalFree(r15);
                            rax = rsi;
                            return;
                        }
                        ecx -= 0xADDR;
                        if (ecx == 0) {
                            r8d = (rax - 0xADDR);
                            r8d = ((r14 != 0xADDR) ? eax : r8d);
                            sub_140024aec(rdi, rbp, r8, *(u32*)(r15));
                            SendMessageW(*(u64*)((rbp + 8)), dword_ADDR, 0, rbp);
                            if (0xADDR != rsi) {
                                rax = rsi;
                                return;
                            }
                            GetDlgItem(rdi, 2);
                            SetWindowTextW(rax, off_140036190);
                            rax = rsi;
                            return;
                        }
                        ecx -= esi;
                        if (ecx != 0) {
                            ecx -= 0xADDR;
                            if (ecx == 0) {
                                edx = dword_ADDR;
                                if (edx == 0) {
                                    rax = rsi;
                                    return;
                                }
                                rcx = *(u64*)((rbp + 8));
                                if (rcx == 0) {
                                    rax = rsi;
                                    return;
                                }
                                r8 = rdi;
                                SendMessageW(*(u64*)((rbp + 8)), rdx, r8, rbp);
                                rax = rsi;
                                return;
                            } else if (ecx != 0xADDR) {
                                eax = 0;
                                return;
                            }
                            r14 >>= 0xADDR;
                            if (r14w != 0xADDR) {
                                rax = rsi;
                                return;
                            }
                            var_20 = 0;
                            ebp = 0xADDR;
                            SendDlgItemMessageW(rdi, ebp, (rsi + 0xADDR), 0, 0);
                            rbx = rax;
                            GetDlgItem(rdi, esi);
                            EnableWindow(rax, ebx);
                            if (*(u32*)(r15) == esi) {
                                GetDlgItem(rdi, &var_n80);
                                EnableWindow(rax, ebx);
                                GetDlgItem(rdi, &var_n7f);
                                EnableWindow(rax, ebx);
                            }
                            var_20 = 0;
                            r9d = 0;
                            SendDlgItemMessageW(rdi, ebp, (r9 + 0xADDR), 0, 0);
                            rbx = rax;
                            GetDlgItem(rdi, esi);
                            EnableWindow(rax, ebx);
                            rax = rsi;
                            return;
                        }
                    }
                }
                case 0xADDR: {
                    ebx -= 0xADDR;
                    sub_1400248d4(rdi, r14d, rsi);
                    eax = 1;
                    return;
                }
                case 0xADDR: {
                    ebx -= 0xADDR;
                rdx = rsi;
                sub_1400249b0(rdi);
                eax = 1;
                return;
                }
                default: {
                    ebx -= 0xADDR;
                eax = 0;
                return;
                }
            }
        } else if (ebx != 0xADDR) {
            r9 = rsi;
            r8 = r14;
            edx = ebx;
            rcx = rdi;
            sub_140028010(rdi, ebx, r14, rsi);
        }
    } else {
        rbp = *(u64*)((rax + 8));
        if (rbp != 0 && (*(u32*)((rbp + 0xADDR)) & 0xADDR) != 0) {
            rax = *(u64*)((rbp + 0xADDR));
        }
    }
    return;
}
