// L1 structure recovery (24 blocks)
void sub_140010d60_l1(void) {
    esi = 0;
    rbx = rdx;
    var_60 = rsi;
    rdi = rcx;
    PathFindExtensionW(rcx);
    if (rax == 0) {
        rcx = "Unknown";
        edx = 0xADDR;
        rcx -= rbx;
        do {
            rax = (rdx + 0xADDR);
            if (rax == 0) break;
            eax = (uint32)(*(u16*)((rcx + rbx)));
            if (ax == 0) break;
            *(u16*)(rbx) = ax;
            rbx += 2;
            rdx -= 1;
        } while (rdx != 0);
        rax = (rbx - 2);
        rax = ((rdx != 0) ? rbx : rax);
        *(u16*)(rax) = si;
    } else {
        _o__wcsicmp(rax, ".log");
        if (eax == 0) {
            rcx = "test/log";
            edx = 0xADDR;
            rcx -= rbx;
            while (1) {
                rax = (rdx + 0xADDR);
                if (rax == 0) break;
                eax = (uint32)(*(u16*)((rcx + rbx)));
                if (ax == 0) break;
                *(u16*)(rbx) = ax;
                rbx += 2;
                rdx -= 1;
                if (rdx != 0) continue;
            }
        } else {
            var_38 = esi;
            rax = &var_60;
            var_30 = rax;
            var_28 = esi;
            var_20 = rsi;
            FindMimeFromData(0, rdi, 0, 0, rsi, esi, &var_60, esi);
            edx = 0xADDR;
            if (eax < 0 /* signed */) {
                rcx = "FAIL/Error";
                rcx -= rbx;
                while (1) {
                    rax = (rdx + 0xADDR);
                    if (rax == 0) break;
                    eax = (uint32)(*(u16*)((rcx + rbx)));
                    if (ax == 0) break;
                    *(u16*)(rbx) = ax;
                    rbx += 2;
                    rdx -= 1;
                    if (rdx != 0) continue;
                }
            } else {
                rcx = var_60;
                r8 = rcx;
                r8 -= rbx;
                do {
                    rax = (rdx + 0xADDR);
                    if (rax == 0) break;
                    eax = (uint32)(*(u16*)((r8 + rbx)));
                    if (ax == 0) break;
                    *(u16*)(rbx) = ax;
                    rbx += 2;
                    rdx -= 1;
                } while (rdx != 0);
                rax = (rbx - 2);
                rax = ((rdx != 0) ? rbx : rax);
                *(u16*)(rax) = si;
                CoTaskMemFree(rcx);
            }
        }
    }
    return;
}
