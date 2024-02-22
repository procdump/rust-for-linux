// SPDX-License-Identifier: GPL-2.0
#![allow(
    // unused_imports,
    // dead_code,
    // unused_variables,
    // unused_unsafe,
    // non_upper_case_globals,
    missing_docs
)]

use core::cell::RefCell;
use core::clone::Clone;
use core::mem::MaybeUninit;
use kernel::bindings::{
    init_net, net_device, packet_type, sk_buff, ETH_HLEN, PACKET_LOOPBACK, PACKET_OUTGOING,
};
use kernel::prelude::*;
use kernel::sync::lock::mutex::Mutex;
use kernel::sync::{Arc, ArcBorrow};
use kernel::types::ForeignOwnable;
use kernel::uapi::ETH_P_ALL;

mod rust_skb;
use rust_skb::SkBuffOwned;

module! {
    type: RustModule,
    name: "rust_module",
    author: "Rust for Linux Contributors",
    description: "Play with rust in the kernel",
    license: "GPL",
}

mod rust_netdevice;
use rust_netdevice::{NetDevice, NetDeviceTracker, PacketType};
static mut PACKET_TYPE: MaybeUninit<packet_type> = MaybeUninit::zeroed();
mod rust_namespace;
use rust_namespace::{NetNamespace, NetNsTracker};

struct RustModule {
    #[allow(dead_code)]
    packet_type: PacketType<PrivateData>,
}

impl RustModule {
    #[inline]
    fn xmit(mut skb: SkBuffOwned<'_>, dev: &NetDevice) {
        skb.set_dev(dev.get_dev());
        skb.push(ETH_HLEN as usize);
        skb.dev_queue_xmit();
    }

    pub(crate) unsafe extern "C" fn eth_rcv(
        skb: *mut sk_buff,
        dev_in: *mut net_device,
        packet_type: *mut packet_type,
        _orig_dev: *mut net_device,
    ) -> i32 {
        let skb = unsafe { SkBuffOwned::from_raw(skb) };
        let priv_data: ArcBorrow<'_, Mutex<RefCell<PrivateData>>> =
            unsafe { Arc::borrow((*packet_type).af_packet_priv) };

        let pkt_type = skb.get_pkt_type();
        // if we don't filter these there's a loop
        if pkt_type == PACKET_LOOPBACK || pkt_type == PACKET_OUTGOING {
            return 0;
        }

        // pr_info!("Received frame!\n");
        let locked = priv_data.lock();
        let borrowed_mut = locked.borrow_mut();
        let mut it = borrowed_mut
            .net_devs
            .iter()
            .filter(|dev| dev.get_dev() != dev_in)
            .peekable();
        while it.peek() != None {
            let dev = it.next().unwrap();

            if it.peek() == None {
                RustModule::xmit(skb, dev);
                return 0;
            }

            let nskb = skb.clone();
            RustModule::xmit(nskb, dev);
        }
        0
    }
}

unsafe impl Sync for RustModule {}

struct PrivateData {
    net_devs: Vec<NetDevice>,
}

const ETH0: &'static str = "eth0";
const ETH1: &'static str = "eth1";

impl kernel::Module for RustModule {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust minimal sample (init)\n");
        pr_info!("Am I built-in? {}\n", !cfg!(MODULE));

        let netns_tracker = Arc::pin_init(NetNsTracker::new()).unwrap();
        let net_ns =
            Arc::try_new(NetNamespace::new(unsafe { &mut init_net }, netns_tracker)).unwrap();
        let netdevice_tracker = Arc::pin_init(NetDeviceTracker::new()).unwrap();
        let eth0 = NetDevice::new(net_ns.clone(), ETH0, netdevice_tracker.clone()).unwrap();
        let eth1 = NetDevice::new(net_ns, ETH1, netdevice_tracker).unwrap();
        let mut net_devs = Vec::new();
        net_devs.try_push(eth0).unwrap();
        net_devs.try_push(eth1).unwrap();
        let packet_type = PacketType::new(
            unsafe { &mut PACKET_TYPE },
            ETH_P_ALL,
            RustModule::eth_rcv,
            PrivateData { net_devs },
        );

        Ok(RustModule { packet_type })
    }
}

impl Drop for RustModule {
    fn drop(&mut self) {
        pr_info!("Rust minimal sample (exit)\n");
    }
}
