// L1 structure recovery (33 blocks)
void sub_14001278c_l1(void) {
    r13d = 0;
    r14 = rcx;
    ecx = (uint32)(*(u16*)(rcx));
    rsi = -1;
    var_20 = 0;
    r12 = rdx;
    if (cx == 0xADDR) {
        r14 += 2;
        edx = 0;
        rdi = r14;
        while (1) {
            eax = (uint32)(*(u16*)(r14));
            if (ax == 0) break;
            if (ax == cx) break;
            edx += 1;
            r14 += 2;
        }
        eax = (rdx + 1);
        edx = 0;
        ebx = eax;
        sub_140009e44(&var_28, 0, eax);
        sub_14000a0c0(&var_20, rax);
        rcx = var_28;
        if (rcx != 0) {
            CoTaskMemFree(var_28);
        }
        r8 = rdi;
    } else if (cx != 0xADDR) {
        rax = rsi;
        do {
            rax += 1;
        } while (*(u16*)((r14 + (rax * 2))) != 0xADDR);
        rbx = (rax + 1);
        edx = 0;
        sub_140009e44(&var_28, 0, rbx);
        sub_14000a0c0(&var_20, rax);
        rcx = var_28;
        if (rcx != 0) {
            CoTaskMemFree(var_28);
        }
        r8 = r14;
    }
    rdx = rbx;
    rbx = var_20;
    rcx = rbx;
    sub_140008384(rbx, rbx);
    var_28 = r13;
    PathIsFileSpecW(r14);
    if (eax == 0) {
        sub_140010a74(rbx, &var_28);
    } else {
        sub_14000a0c0(&var_28, &var_20);
        rbx = var_20;
    }
    edx = 0;
    sub_1400029dc(&var_30, 0, 0xADDR);
    rdi = var_28;
    FindFirstFileW(rdi, &var_30);
    if (rax == rsi) {
        PathFindExtensionW(rdi);
        if (*(u16*)(rax) == r13w) {
            rax = rsi;
            do {
                rax += 1;
            } while (*(u16*)((rdi + (rax * 2))) != r13w);
            r15 = (rax + 5);
            edx = 0;
            sub_140009e44(&var_20, 0, r15);
            rsi = var_20;
            if (rsi != 0) {
                sub_1400082b0(rsi, r15, rdi);
                sub_1400082b0(rsi, r15, ".txt");
            }
            FindFirstFileW(rsi, &var_30);
            if (rax != -1) {
                FindClose(rax);
            }
            sub_14000a0c0(&var_28, &var_20);
            rcx = var_20;
            if (rcx != 0) {
                CoTaskMemFree(var_20);
            }
        }
    } else {
        FindClose(rax);
    }
    sub_14000a0c0(r12, &var_28);
    rcx = var_28;
    if (rcx != 0) {
        CoTaskMemFree(var_28);
    }
    if (rbx != 0) {
        CoTaskMemFree(rbx);
    }
    rax = r14;
    return;
}
