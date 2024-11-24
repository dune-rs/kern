#![no_std]
#![feature(allocator_api, global_asm)]

extern crate alloc;

use alloc::boxed::Box;
use core::ptr;
use kernel::prelude::*;
use kernel::file_operations::{FileOpener, FileOperations, IoctlCommand};
use kernel::file_operations::{IoctlHandler, IoctlHandlerRegistry};
use kernel::file_operations::IoctlCommand::ReadWrite;
use kernel::io_buffer::IoBufferReader;
use kernel::io_buffer::IoBufferWriter;
use kernel::module;
use kernel::sync::Ref;
use kernel::sync::RefBorrow;
use kernel::sync::RefBorrowMut;
use kernel::sync::RefCounted;
use kernel::sync::RefCountedMutex;
use kernel::sync::RefCountedMutexGuard;
use kernel::sync::RefCountedMutexGuardMut;

mod compat;
mod core;
mod dune;
mod ept;
mod preempttrap;
mod vmx;

pub use compat::*;
pub use core::*;
pub use dune::*;
pub use ept::*;
pub use preempttrap::*;
pub use vmx::*;



struct DuneDevice;

impl FileOpener<()> for DuneDevice {
    fn open(ctx: &kernel::file_operations::FileOpenContext, _: &()) -> Result<Self::Wrapper> {
        Ok(Box::try_new(DuneDevice)?)
    }
}

impl FileOperations for DuneDevice {
    type Wrapper = Box<Self>;
    type OpenData = ();

    kernel::declare_file_operations!(open, ioctl, release);
}

impl IoctlHandler for DuneDevice {
    fn ioctl(
        &self,
        cmd: IoctlCommand,
        reader: &mut IoBufferReader,
        writer: &mut IoBufferWriter,
    ) -> Result<i32> {
        match cmd {
            ReadWrite(DUNE_ENTER) => {
                let mut conf: DuneConfig = reader.read()?;
                let ret = dune_enter(&mut conf, &mut conf.ret);
                writer.write(&conf)?;
                Ok(ret)
            }
            ReadWrite(DUNE_GET_SYSCALL) => {
                let syscall = unsafe { libc::syscall(libc::SYS_rdmsr, libc::MSR_LSTAR) as i32 };
                writer.write(&syscall)?;
                Ok(0)
            }
            ReadWrite(DUNE_GET_LAYOUT) => {
                let mut layout: DuneLayout = DuneLayout {
                    phys_limit: (1 << unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) }) as u64,
                    base_map: (unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) } as u64 - 1) & !((1 << 30) - 1),
                    base_stack: (unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) } as u64 - 1) & !((1 << 30) - 1),
                };
                writer.write(&layout)?;
                Ok(0)
            }
            ReadWrite(DUNE_TRAP_ENABLE) => {
                let arg: u64 = reader.read()?;
                let ret = dune_trap_enable(arg);
                Ok(ret as i32)
            }
            ReadWrite(DUNE_TRAP_DISABLE) => {
                let arg: u64 = reader.read()?;
                let ret = dune_trap_disable(arg);
                Ok(ret as i32)
            }
            _ => Err(Error::EINVAL),
        }
    }
}

impl Drop for DuneDevice {
    fn drop(&mut self) {
        unsafe {
            vmx_cleanup();
        }
    }
}

module! {
    type: DuneModule,
    name: b"dune",
    author: b"Adam Belay",
    description: b"A driver for Dune",
    license: b"GPL",
}

struct DuneModule;

impl KernelModule for DuneModule {
    fn init() -> Result<Self> {
        if unsafe { vmx_init() } != 0 {
            return Err(Error::EINVAL);
        }

        let device = kernel::miscdev::Registration::new_pinned::<DuneDevice>(
            cstr!("dune"),
            None,
        )?;
        Ok(DuneModule)
    }
}

impl Drop for DuneModule {
    fn drop(&mut self) {
        unsafe {
            vmx_exit();
        }
    }
}
