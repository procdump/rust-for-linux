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
use core::iter::{IntoIterator, Iterator};
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::ptr::null_mut;
use kernel::bindings::{
    dev_add_pack, dev_queue_xmit, dev_remove_pack, get_net_track, init_net, kfree_skb, net,
    net_device, netdev_get_by_name, netdev_put, netdevice_tracker, netif_rx, netns_tracker,
    packet_type, put_net_track, sk_buff, skb_copy, skb_reset_network_header, skb_set_dev, ETH_HLEN,
    GFP_ATOMIC, GFP_KERNEL, PACKET_LOOPBACK, PACKET_OUTGOING,
};
use kernel::c_str;
use kernel::prelude::*;
use kernel::sync::{LockClassKey, Mutex};
use kernel::types::Opaque;
use kernel::uapi::ETH_P_ALL;
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
    dev: Vec<*mut net_device>,
    packet_type: packet_type,
    netns_tracker: netns_tracker,
    netdev_tracker: netdevice_tracker,
    stopped: bool,
}

impl Container {
    fn get_ns(&self) -> *mut net {
        self.net
    }

    fn set_ns(&mut self, ns: *mut net) {
        self.net = ns;
    }

    fn set_dev(&mut self, dev: *mut net_device) {
        let _ = self.dev.try_push(dev);
    }

    fn add_packet_type(&mut self) {
        let ether_type = ETH_P_ALL as u16;
        self.packet_type.type_ = ether_type.to_be();
        self.packet_type.func = Some(Container::eth_rcv);
        unsafe {
            dev_add_pack(&mut self.packet_type);
        }
    }

    fn remove_packet_type(&mut self) {
        unsafe {
            dev_remove_pack(&mut self.packet_type);
        }
    }

    pub fn get_netns_tracker(&mut self) -> &mut netns_tracker {
        &mut self.netns_tracker
    }

    pub fn get_netdev_tracker(&mut self) -> &mut netdevice_tracker {
        &mut self.netdev_tracker
    }

    pub fn acquire_net(&mut self) {
        if self.get_ns().is_null() {
            let net = unsafe { get_net_track(&mut init_net, self.get_netns_tracker(), GFP_KERNEL) };
            self.set_ns(net);
        }
    }

    pub fn acquire_dev(&mut self, name: &str) {
        self.acquire_net();
        let dev_name = CString::try_from_fmt(fmt!("{}", name)).unwrap();
        let dev = unsafe {
            netdev_get_by_name(
                self.net,
                dev_name.as_char_ptr(),
                self.get_netdev_tracker(),
                GFP_KERNEL,
            )
        };

        if dev.is_null() {
            pr_info!("Can't find dev: {}\n", name);
            return;
        }

        pr_info!("Acquiring netns_tracker: {:p}\n", unsafe {
            self.get_netns_tracker()
        });
        let c_str = unsafe { CStr::from_char_ptr(&(*dev).name as *const [i8; 16] as *const i8) };
        pr_info!("Got a netdev by name: {}\n", c_str.to_str().unwrap());
        self.set_dev(dev);
    }

    pub fn release_dev(&mut self) {
        let devs_num = self.dev.len();
        pr_info!("Releasing {} netdevs !\n", devs_num);
        for i in 0..devs_num {
            unsafe { netdev_put(self.dev[i], self.get_netdev_tracker()) };
        }
        pr_info!("Releasing netns_tracker: {:p}\n", unsafe {
            self.get_netns_tracker()
        });
        unsafe { put_net_track(self.get_ns(), self.get_netns_tracker()) };
        pr_info!("Releasing net namespace!\n");
    }

    pub fn new() -> Pin<&'static mut Opaque<Container>> {
        let lock = Arc::pin_init(new_mutex!((), "Container Mutex")).unwrap();
        unsafe { MTX = Some(lock) };
        let mg = unsafe { MTX.as_mut().unwrap().lock() };

        let our_cont = unsafe { &mut CONTAINER };
        unsafe {
            (*our_cont.get()).acquire_dev("eth0");
            (*our_cont.get()).acquire_dev("eth1");
            (*our_cont.get()).add_packet_type();
            (*our_cont.get()).stopped = false;
        }

        let cont = Pin::static_mut(our_cont);
        cont
    }

    pub fn deinit(cont: &mut Pin<&'static mut Opaque<Container>>) {
        let mg = unsafe { MTX.as_mut().unwrap().lock() };

        unsafe {
            (*cont.get()).stopped = true;
            (*cont.get()).release_dev();
            (*cont.get()).remove_packet_type();
        }
    }

    pub fn get_egress_devs(dev_in: *mut net_device) -> Vec<*mut net_device> {
        let mut egress_devs = Vec::new();
        let mg = unsafe { MTX.as_mut().unwrap().lock() };
        let stopped = unsafe { (*CONTAINER.get()).stopped };
        if stopped == false {
            unsafe {
                (*CONTAINER.get()).dev.iter_mut().for_each(|dev| {
                    if dev_in != *dev {
                        egress_devs.try_push(*dev).unwrap();
                    }
                });
            };
        }
        egress_devs
    }

    pub fn get_dev_name<'a>(dev: *mut net_device) -> &'a CStr {
        unsafe { CStr::from_char_ptr(&(*dev).name as *const [i8; 16] as *const i8) }
    }

    pub unsafe extern "C" fn eth_rcv(
        frame: *mut sk_buff,
        dev_in: *mut net_device,
        packet_type: *mut packet_type,
        orig_dev: *mut net_device,
    ) -> i32 {
        let frame = unsafe { SkBuff::from_raw(frame) };
        unsafe {
            let pkt_type = frame.get_pkt_type();
            // if we don't filter these there's a loop
            if pkt_type == PACKET_LOOPBACK || pkt_type == PACKET_OUTGOING {
                frame.drop_skb();
                return 0;
            }
        }

        // pr_info!("Received frame!\n");
        let egress_devs = Container::get_egress_devs(dev_in);
        egress_devs.into_iter().for_each(|dev| {
            let nskb = frame.clone();
            nskb.set_dev(dev);
            // let dev_out_name = Container::get_dev_name(dev);
            // let dev_in_name = Container::get_dev_name(dev_in);
            // pr_info!(
            //     "Forwarding packet from: {} to: {}\n",
            //     dev_in_name.to_str().unwrap(),
            //     dev_out_name.to_str().unwrap()
            // );
            nskb.undo_skb_pull(ETH_HLEN as usize);
            nskb.skb_dev_queue_xmit();
        });
        // unsafe { netif_rx(frame) }
        frame.drop_skb();
        0
    }
}

#[repr(transparent)]
pub struct SkBuff(Opaque<sk_buff>);

impl SkBuff {
    /// Creates a new [`SkBuff`] instance from a raw pointer.
    ///
    /// # Safety
    ///
    /// For the duration of 'a, the pointer must point at a valid `sk_buff`,
    /// and the caller must be in a context where all methods defined on this struct
    /// are safe to call.
    pub unsafe fn from_raw<'a>(ptr: *mut sk_buff) -> &'a mut Self {
        // CAST: `Self` is a `repr(transparent)` wrapper around `sk_buff`.
        let ptr = ptr.cast::<Self>();
        // SAFETY: by the function requirements the pointer is valid and we have unique access for
        // the duration of `'a`.
        unsafe { &mut *ptr }
    }

    pub fn get_raw(&self) -> *mut sk_buff {
        self.0.get()
    }

    pub fn undo_skb_pull(&mut self, how_many: usize) {
        let frame = self.get_raw();
        unsafe {
            (*frame).data = (*frame).data.offset(-(how_many as isize));
            (*frame).len += how_many as u32;
        }
    }

    pub fn get_pkt_type(&self) -> u32 {
        let frame = self.get_raw();
        unsafe {
            let pkt_type = (*frame)
                .__bindgen_anon_5
                .__bindgen_anon_1
                .as_ref()
                .pkt_type() as u32;
            // let offset = -(ETH_HLEN as isize);
            // pr_info!("pkt_type: {}\n", pkt_type);
            // pr_info!("skb->data: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}\n",
            //      *((*frame).data.offset(offset+0)), *((*frame).data.offset(offset+1)), *((*frame).data.offset(offset+2)),
            //      *((*frame).data.offset(offset+3)), *((*frame).data.offset(offset+4)), *((*frame).data.offset(offset+5)),
            //      *((*frame).data.offset(offset+6)), *((*frame).data.offset(offset+7)));

            pkt_type
        }
    }

    pub fn drop_skb(&mut self) {
        let frame = self.get_raw();
        unsafe {
            kfree_skb(frame);
        }
    }

    // for a lifetime of 'a give me a reference to the cloned skb
    pub fn clone<'a, 'b>(&'b self) -> &'a mut SkBuff {
        unsafe {
            let frame = self.get_raw();
            let nskb = skb_copy(frame, GFP_ATOMIC);
            SkBuff::from_raw(nskb)
        }
    }

    pub fn set_dev(&mut self, dev: *mut net_device) {
        let frame = self.get_raw();
        unsafe {
            (*frame)
                .__bindgen_anon_1
                .__bindgen_anon_1
                .__bindgen_anon_1
                .dev = dev;
        }
    }

    pub fn skb_dev_queue_xmit(&mut self) {
        let frame = self.get_raw();
        unsafe {
            dev_queue_xmit(frame);
        }
    }
}

static mut CONTAINER: Opaque<Container> = Opaque::new(Container {
    net: null_mut(),
    dev: Vec::new(),
    netns_tracker: netns_tracker {},
    netdev_tracker: netdevice_tracker {},
    packet_type: unsafe { MaybeUninit::zeroed().assume_init() },
    stopped: true,
});

struct RustMinimal {
    cont: Pin<&'static mut Opaque<Container>>,
}

unsafe impl Sync for RustMinimal {}

use kernel::new_mutex;
use kernel::sync::Arc;
static mut MTX: Option<Arc<Mutex<()>>> = None;

impl kernel::Module for RustMinimal {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust minimal sample (init)\n");
        pr_info!("Am I built-in? {}\n", !cfg!(MODULE));

        let cont = Container::new();
        Ok(RustMinimal { cont })
    }
}

impl Drop for RustMinimal {
    fn drop(&mut self) {
        pr_info!("Rust minimal sample (exit)\n");
        Container::deinit(&mut self.cont);
    }
}
