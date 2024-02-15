// SPDX-License-Identifier: GPL-2.0

#![allow(
    unused_imports,
    dead_code,
    unused_variables,
    unused_unsafe,
    non_upper_case_globals,
    missing_docs
)]

//! Rust minimal sample.
use core::borrow::BorrowMut;
use core::cell::UnsafeCell;
use core::convert::AsMut;
use core::ops::{Deref, DerefMut};
use core::ptr::null_mut;
use kernel::bindings::{
    get_net_track, init_net, net, net_device, netdev_get_by_name, netdev_put, netdevice_tracker,
    netns_tracker, put_net_track, GFP_KERNEL,
};
use kernel::c_str;
use kernel::prelude::*;
use kernel::types::Opaque;
use kernel::{fmt, str::CString};

module! {
    type: RustMinimal,
    name: "rust_minimal",
    author: "Rust for Linux Contributors",
    description: "Rust minimal sample",
    license: "GPL",
}

pub struct Container {
    net: *mut net,
    dev: *mut net_device,
    netns_tracker: netns_tracker,
    netdev_tracker: netdevice_tracker,
}

impl Container {
    fn get_ns(&self) -> *mut net {
        self.net
    }

    fn get_dev(&self) -> *mut net_device {
        self.dev
    }

    fn set_ns(&mut self, ns: *mut net) {
        self.net = ns;
    }

    fn set_dev(&mut self, dev: *mut net_device) {
        self.dev = dev;
    }

    pub fn get_netns_tracker(&mut self) -> &mut netns_tracker {
        &mut self.netns_tracker
    }

    pub fn get_netdev_tracker(&mut self) -> &mut netdevice_tracker {
        &mut self.netdev_tracker
    }

    pub fn acquire_dev(&mut self, name: &str) {
        let net = unsafe { get_net_track(&mut init_net, self.get_netns_tracker(), GFP_KERNEL) };
        let dev_name = CString::try_from_fmt(fmt!("{}", name)).unwrap();
        let dev = unsafe {
            netdev_get_by_name(
                net,
                dev_name.as_char_ptr(),
                self.get_netdev_tracker(),
                GFP_KERNEL,
            )
        };

        pr_info!("Acquiring netns_tracker: {:p}\n", unsafe {
            self.get_netns_tracker()
        });
        let c_str = unsafe { CStr::from_char_ptr(&(*dev).name as *const [i8; 16] as *const i8) };
        pr_info!("Got a netdev by name: {}\n", c_str.to_str().unwrap());

        self.set_ns(net);
        self.set_dev(dev);
    }

    pub fn release_dev(&mut self) {
        unsafe { netdev_put(self.get_dev(), self.get_netdev_tracker()) };
        pr_info!("Releasing netdev!\n");
        pr_info!("Releasing netns_tracker: {:p}\n", unsafe {
            self.get_netns_tracker()
        });
        unsafe { put_net_track(self.get_ns(), self.get_netns_tracker()) };
        pr_info!("Releasing net namespace!\n");
    }

    pub fn init() -> Pin<&'static mut Opaque<Container>> {
        let our_cont = unsafe { &mut CONTAINER };
        unsafe {
            (*our_cont.get()).acquire_dev("eth0");
        }

        let cont = Pin::static_mut(our_cont);
        cont
    }

    pub fn deinit(cont: &mut Pin<&'static mut Opaque<Container>>) {
        unsafe {
            (*cont.get()).release_dev();
        }
    }
}

static mut CONTAINER: Opaque<Container> = Opaque::new(Container {
    net: null_mut(),
    dev: null_mut(),
    netns_tracker: netns_tracker {},
    netdev_tracker: netdevice_tracker {},
});

struct RustMinimal {
    cont: Pin<&'static mut Opaque<Container>>,
}

unsafe impl Sync for RustMinimal {}

impl kernel::Module for RustMinimal {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust minimal sample (init)\n");
        pr_info!("Am I built-in? {}\n", !cfg!(MODULE));

        let cont = Container::init();
        Ok(RustMinimal { cont })
    }
}

impl Drop for RustMinimal {
    fn drop(&mut self) {
        pr_info!("Rust minimal sample (exit)\n");
        Container::deinit(&mut self.cont);
    }
}
