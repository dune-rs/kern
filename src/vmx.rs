use std::ptr;
use libc::{c_void, c_ulong, c_long, copy_from_user};
use kernel::prelude::*;
use kernel::sync::Mutex;
use kernel::cpu;
use kernel::cpu::Cpu;