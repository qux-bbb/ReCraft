// L1 structure recovery (18 blocks)
void sub_140003ba4_l1(void) {
    rbx = rcx;
    *(u32*)(rcx) = 0;
    rcx += 0xADDR;
    edx = 0;
    sub_1400093f0(rcx, 0);
    sub_1400066f4(rbx);
    rcx = *(u64*)((rbx + 0xADDR));
    rdx = qword_ADDR;
    if (rcx == 0) {
        rax = qword_ADDR;
    } else if (rdx == 0) {
        rax = qword_ADDR;
        if (rax != 0) {
            sub_140028010();
            rdx = qword_ADDR;
        }
    } else {
        rax = rdx;
        sub_140028010();
        rdx = qword_ADDR;
    }
    rcx = *(u64*)((rbx + 0xADDR));
    if (rcx != 0) {
        if (rdx == 0) {
            if (rax != 0) {
                sub_140028010();
            }
        } else {
            rax = rdx;
            sub_140028010();
        }
    }
    rdi = *(u64*)((rbx + 0xADDR));
    *(u64*)((rbx + 0xADDR)) = 0;
    if (rdi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rdi);
    }
    rdi = *(u64*)((rbx + 0xADDR));
    *(u64*)((rbx + 0xADDR)) = 0;
    if (rdi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rdi);
    }
    rbx = *(u64*)((rbx + 0xADDR));
    if (rbx != 0) {
        SetThreadpoolTimer(rbx, 0, 0, 0);
        WaitForThreadpoolTimerCallbacks(rbx, 1);
        CloseThreadpoolTimer(rbx);
    }
    return;
}
