// L1 structure recovery (23 blocks)
void sub_140012f3c_l1(void) {
    r13 = rcx;
    r14d = 0xADDR;
    LocalAlloc(0xADDR, 0xADDR);
    rbx = rax;
    if (rax == 0) {
        loc_ADDR:
        eax = 0;
    } else {
        rdi = off_1400295b0;
        do {
            ebp = 0;
            r12d = (r14 - 1);
            r15 = rdi;
            esi = 0;
            rdi = off_140029790;
            do {
                rdx = *(u64*)(r15);
                edx = *(u32*)(rdx);
                LoadStringW(r13, *(u32*)(rdx), rbx, r14d);
                if (eax >= r12d /* signed */) break;
                esi += 1;
                ebp += 1;
                esi += eax;
                r15 += 8;
            } while (r15 < rdi /* signed */);
            LocalFree(rbx);
            rdi = off_1400295b0;
            r14d += r14d;
            rdx = (int64)(r14d);
            rdx += rdx;
            LocalAlloc(0xADDR, (rdx + rdx));
            rbx = rax;
        } while (rax != 0);
    }
    return;
    loc_ADDR:
    rdx = *(u64*)(rdi);
    edx = *(u32*)(rdx);
    LoadStringW(r13, *(u32*)(rdx), rsi, ebx);
    rcx = *(u64*)(rdi);
    eax += 1;
    *(u64*)(rcx) = rsi;
    rcx = (int64)(eax);
    rsi += (rcx * 2);
    if (eax > ebx /* signed */) goto loc_ADDR;
    ebx -= eax;
    rdi += 8;
    if (rdi < off_140029790 /* signed */) {
        rdx = *(u64*)(rdi);
        edx = *(u32*)(rdx);
        LoadStringW(r13, *(u32*)(rdx), rsi, ebx);
        rcx = *(u64*)(rdi);
        eax += 1;
        *(u64*)(rcx) = rsi;
        rcx = (int64)(eax);
        rsi += (rcx * 2);
        if (eax > ebx /* signed */) goto loc_ADDR;
        ebx -= eax;
        rdi += 8;
        rax = off_140029790;
        if (rdi < rax /* signed */) goto loc_ADDR;
        r9 = qword_ADDR;
        rcx = off_1400356e0;
        r8d = 0xADDR;
        r9 -= off_1400356e0;
        edx = r8d;
        rax = (rdx + 0xADDR);
        if (rax == 0) goto loc_ADDR;
        eax = (uint32)(*(u16*)((r9 + rcx)));
        if (ax == 0) goto loc_ADDR;
        *(u16*)(rcx) = ax;
        rcx += 2;
        rdx -= 1;
        if (rdx != 0) goto loc_ADDR;
        rax = (rcx - 2);
        rdx = off_140035730;
        rax = ((rdx != 0) ? rcx : rax);
        rcx = qword_ADDR;
        rcx -= rdx;
        *(u16*)(rax) = bp;
        rax = (r8 + 0xADDR);
        if (rax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        eax = (uint32)(*(u16*)((rcx + rdx)));
        if (ax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        *(u16*)(rdx) = ax;
        rdx += 2;
        r8 -= 1;
        if (r8 != 0) goto loc_ADDR;
        rcx = (rdx - 2);
        eax = 1;
        rcx = ((r8 != 0) ? rdx : rcx);
        *(u16*)(rcx) = bp;
        rcx = qword_ADDR;
        edx = (uint32)(*(u16*)(rcx));
        word_ADDR = dx;
        return;
    }
    r9 = qword_ADDR;
    rcx = off_1400356e0;
    r8d = 0xADDR;
    r9 -= off_1400356e0;
    edx = r8d;
    loc_ADDR:
    rax = (rdx + 0xADDR);
    if (rax == 0) {
        rax = (rcx - 2);
        rdx = off_140035730;
        rax = ((rdx != 0) ? rcx : rax);
        rcx = qword_ADDR;
        rcx -= rdx;
        *(u16*)(rax) = bp;
        rax = (r8 + 0xADDR);
        if (rax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        eax = (uint32)(*(u16*)((rcx + rdx)));
        if (ax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        *(u16*)(rdx) = ax;
        rdx += 2;
        r8 -= 1;
        if (r8 != 0) goto loc_ADDR;
        rcx = (rdx - 2);
        eax = 1;
        rcx = ((r8 != 0) ? rdx : rcx);
        *(u16*)(rcx) = bp;
        rcx = qword_ADDR;
        edx = (uint32)(*(u16*)(rcx));
        word_ADDR = dx;
        return;
    }
    eax = (uint32)(*(u16*)((r9 + rcx)));
    if (ax == 0) {
        rax = (rcx - 2);
        rdx = off_140035730;
        rax = ((rdx != 0) ? rcx : rax);
        rcx = qword_ADDR;
        rcx -= rdx;
        *(u16*)(rax) = bp;
        rax = (r8 + 0xADDR);
        if (rax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        eax = (uint32)(*(u16*)((rcx + rdx)));
        if (ax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        *(u16*)(rdx) = ax;
        rdx += 2;
        r8 -= 1;
        if (r8 != 0) goto loc_ADDR;
        rcx = (rdx - 2);
        eax = 1;
        rcx = ((r8 != 0) ? rdx : rcx);
        *(u16*)(rcx) = bp;
        rcx = qword_ADDR;
        edx = (uint32)(*(u16*)(rcx));
        word_ADDR = dx;
        return;
    }
    *(u16*)(rcx) = ax;
    rcx += 2;
    rdx -= 1;
    if (rdx != 0) {
        rax = (rdx + 0xADDR);
        if (rax == 0) goto loc_ADDR;
        eax = (uint32)(*(u16*)((r9 + rcx)));
        if (ax == 0) goto loc_ADDR;
        *(u16*)(rcx) = ax;
        rcx += 2;
        rdx -= 1;
        if (rdx != 0) goto loc_ADDR;
        rax = (rcx - 2);
        rdx = off_140035730;
        rax = ((rdx != 0) ? rcx : rax);
        rcx = qword_ADDR;
        rcx -= rdx;
        *(u16*)(rax) = bp;
        rax = (r8 + 0xADDR);
        if (rax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        eax = (uint32)(*(u16*)((rcx + rdx)));
        if (ax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        *(u16*)(rdx) = ax;
        rdx += 2;
        r8 -= 1;
        if (r8 != 0) goto loc_ADDR;
        rcx = (rdx - 2);
        eax = 1;
        rcx = ((r8 != 0) ? rdx : rcx);
        *(u16*)(rcx) = bp;
        rcx = qword_ADDR;
        edx = (uint32)(*(u16*)(rcx));
        word_ADDR = dx;
        return;
    }
    loc_ADDR:
    rax = (rcx - 2);
    rdx = off_140035730;
    rax = ((rdx != 0) ? rcx : rax);
    rcx = qword_ADDR;
    rcx -= rdx;
    *(u16*)(rax) = bp;
    loc_ADDR:
    rax = (r8 + 0xADDR);
    if (rax == 0) {
        rcx = (rdx - 2);
        eax = 1;
        rcx = ((r8 != 0) ? rdx : rcx);
        *(u16*)(rcx) = bp;
        rcx = qword_ADDR;
        edx = (uint32)(*(u16*)(rcx));
        word_ADDR = dx;
        return;
    }
    eax = (uint32)(*(u16*)((rcx + rdx)));
    if (ax == 0) {
        rcx = (rdx - 2);
        eax = 1;
        rcx = ((r8 != 0) ? rdx : rcx);
        *(u16*)(rcx) = bp;
        rcx = qword_ADDR;
        edx = (uint32)(*(u16*)(rcx));
        word_ADDR = dx;
        return;
    }
    *(u16*)(rdx) = ax;
    rdx += 2;
    r8 -= 1;
    if (r8 != 0) {
        rax = (r8 + 0xADDR);
        if (rax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        eax = (uint32)(*(u16*)((rcx + rdx)));
        if (ax == 0) {
            rcx = (rdx - 2);
            eax = 1;
            rcx = ((r8 != 0) ? rdx : rcx);
            *(u16*)(rcx) = bp;
            rcx = qword_ADDR;
            edx = (uint32)(*(u16*)(rcx));
            word_ADDR = dx;
            return;
        }
        *(u16*)(rdx) = ax;
        rdx += 2;
        r8 -= 1;
        if (r8 != 0) goto loc_ADDR;
        rcx = (rdx - 2);
        eax = 1;
        rcx = ((r8 != 0) ? rdx : rcx);
        *(u16*)(rcx) = bp;
        rcx = qword_ADDR;
        edx = (uint32)(*(u16*)(rcx));
        word_ADDR = dx;
        return;
    }
    rcx = (rdx - 2);
    eax = 1;
    rcx = ((r8 != 0) ? rdx : rcx);
    *(u16*)(rcx) = bp;
    rcx = qword_ADDR;
    edx = (uint32)(*(u16*)(rcx));
    word_ADDR = dx;
    return;
}
