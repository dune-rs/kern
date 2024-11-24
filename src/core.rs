use libc::{c_void, c_ulong, c_long, copy_from_user};
use std::ptr;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering};

/* Callbacks for perf tool.  We intentionally make a wrong assumption that we
 * are always in the kernel mode because perf cannot profile user applications
 * on guest.
 * Callbacks are registered and unregistered along with Dune module.
 */
static LOCAL_VCPU: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

fn dune_is_in_guest() -> bool {
    !LOCAL_VCPU.load(Ordering::SeqCst).is_null()
}

fn dune_is_user_mode() -> bool {
    false
}

fn dune_get_guest_ip() -> u64 {
    if dune_is_in_guest() {
        unsafe { vmcs_readl(GUEST_RIP) }
    } else {
        0
    }
}

struct PerfGuestInfoCallbacks {
    is_in_guest: fn() -> bool,
    is_user_mode: fn() -> bool,
    get_guest_ip: fn() -> u64,
}

static DUNE_GUEST_CBS: PerfGuestInfoCallbacks = PerfGuestInfoCallbacks {
    is_in_guest: dune_is_in_guest,
    is_user_mode: dune_is_user_mode,
    get_guest_ip: dune_get_guest_ip,
};

pub fn dune_enter(conf: &mut DuneConfig, ret: &mut i64) -> i32 {
    unsafe {
        vmx_launch(conf, ret)
    }
}
