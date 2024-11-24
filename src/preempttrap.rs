use std::ptr;
use libc::{c_void, c_ulong, c_long, copy_from_user};

use core::ptr;
use core::mem::size_of;
use core::sync::atomic::{AtomicBool, Ordering};
use linux_kernel_module::{c_types, println, KernelResult};
use linux_kernel_module::task::Task;
use linux_kernel_module::user_ptr::UserPtr;
use linux_kernel_module::prelude::*;
use dune_sys::{DuneConfig, DuneTrapConfig, DuneTrapRegs, dummy_notify_func};

static mut TRAP_CONF: DuneTrapConfig = DuneTrapConfig::default();
static mut TRAP_STATE: TrapState = TrapState::default();

extern "C" fn notifier_sched_in(notifier: *mut c_void, cpu: i32) {
    unsafe {
        if !TRAP_STATE.triggered && Task::current().kstack_top() == TRAP_CONF.trigger_rip {
            TRAP_STATE.triggered = true;
            TRAP_STATE.count = TRAP_CONF.delay;
        }

        if TRAP_STATE.triggered && TRAP_STATE.count == 0 {
            let regs = Task::current().regs();
            let mut trap_regs = DuneTrapRegs::default();

            TRAP_STATE.triggered = false;

            trap_regs.rax = regs.rax;
            trap_regs.rbx = regs.rbx;
            trap_regs.rcx = regs.rcx;
            trap_regs.rdx = regs.rdx;
            trap_regs.rsi = regs.rsi;
            trap_regs.rdi = regs.rdi;
            trap_regs.rsp = regs.rsp;
            trap_regs.rbp = regs.rbp;
            trap_regs.r8 = regs.r8;
            trap_regs.r9 = regs.r9;
            trap_regs.r10 = regs.r10;
            trap_regs.r11 = regs.r11;
            trap_regs.r12 = regs.r12;
            trap_regs.r13 = regs.r13;
            trap_regs.r14 = regs.r14;
            trap_regs.r15 = regs.r15;
            trap_regs.rip = regs.rip;
            trap_regs.rflags = regs.rflags;

            regs.rflags &= !0x100;
            Task::current().clear_single_step();

            if size_of::<DuneTrapRegs>() == TRAP_CONF.regs_size {
                UserPtr::new(TRAP_CONF.regs).write(&trap_regs).unwrap();
                regs.rip = TRAP_CONF.notify_func as u64;
                regs.rdi = TRAP_CONF.regs as u64;
                regs.rsi = TRAP_CONF.priv_data as u64;
                regs.rsp -= 128;
            }
        }
    }
}

extern "C" fn notifier_sched_out(notifier: *mut c_void, next: *mut c_void) {}

static NOTIFIER_OPS: linux_kernel_module::preempt::PreemptOps = linux_kernel_module::preempt::PreemptOps {
    sched_in: notifier_sched_in,
    sched_out: notifier_sched_out,
};

static NOTIFIER: linux_kernel_module::preempt::PreemptNotifier = linux_kernel_module::preempt::PreemptNotifier {
    ops: &NOTIFIER_OPS,
};

fn dune_trap_enable(arg: u64) -> KernelResult<c_long> {
    let mut r: c_long = 0;

    let user_ptr = UserPtr::new(arg as *mut DuneTrapConfig);
    if user_ptr.read(&mut TRAP_CONF).is_err() {
        r = -c_types::EIO;
        return Ok(r);
    }

    linux_kernel_module::preempt::register_notifier(&NOTIFIER);
    TRAP_STATE.enabled.store(true, Ordering::SeqCst);

    Ok(r)
}

fn dune_trap_disable(_arg: u64) -> KernelResult<c_long> {
    if TRAP_STATE.enabled.load(Ordering::SeqCst) {
        linux_kernel_module::preempt::unregister_notifier(&NOTIFIER);
    }
    TRAP_STATE.enabled.store(false, Ordering::SeqCst);

    Ok(0)
}
