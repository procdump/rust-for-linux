// SPDX-License-Identifier: GPL-2.0
#![allow(
    // unused_imports,
    // dead_code,
    // unused_variables,
    // unused_unsafe,
    // non_upper_case_globals,
    missing_docs
)]

use core::clone::Clone;
use core::mem::MaybeUninit;
use kernel::bindings::{
    init_net, net_device, netdevice_tracker, netns_tracker, packet_type, sk_buff, ETH_HLEN,
    PACKET_LOOPBACK, PACKET_OUTGOING,
};
use kernel::prelude::*;
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

pub unsafe extern "C" fn eth_rcv(
    skb: *mut sk_buff,
    dev_in: *mut net_device,
    packet_type: *mut packet_type,
    _orig_dev: *mut net_device,
) -> i32 {
    let skb = unsafe { SkBuffOwned::from_raw(skb) };
    let priv_data: ArcBorrow<'_, Vec<NetDevice>> =
        unsafe { Arc::borrow((*packet_type).af_packet_priv) };

    let pkt_type = skb.get_pkt_type();
    // if we don't filter these there's a loop
    if pkt_type == PACKET_LOOPBACK || pkt_type == PACKET_OUTGOING {
        return 0;
    }

    // pr_info!("Received frame!\n");
    priv_data.iter().for_each(|dev| {
        let dev = dev.get_dev();
        if dev != dev_in {
            let mut nskb = skb.clone();
            nskb.set_dev(dev);
            nskb.undo_skb_pull(ETH_HLEN as usize);
            nskb.dev_queue_xmit();
        }
    });
    0
}

mod rust_netdevice;
use rust_netdevice::{NetDevice, PacketType};
static mut PACKET_TYPE: MaybeUninit<packet_type> = MaybeUninit::zeroed();
static mut NETDEV_TRACKER: MaybeUninit<netdevice_tracker> = MaybeUninit::zeroed();
mod rust_namespace;
use rust_namespace::NetNamespace;
static mut NET_NS_TRACKER: MaybeUninit<netns_tracker> = MaybeUninit::zeroed();

struct RustModule {
    #[allow(dead_code)]
    packet_type: PacketType<Vec<NetDevice>>,
}

unsafe impl Sync for RustModule {}

impl kernel::Module for RustModule {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust minimal sample (init)\n");
        pr_info!("Am I built-in? {}\n", !cfg!(MODULE));

        let net_ns = Arc::try_new(NetNamespace::new(unsafe { &mut init_net }, unsafe {
            &mut NET_NS_TRACKER
        }))
        .unwrap();
        let eth0 = NetDevice::new(net_ns.clone(), "eth0", unsafe { &mut NETDEV_TRACKER }).unwrap();
        let eth1 = NetDevice::new(net_ns, "eth1", unsafe { &mut NETDEV_TRACKER }).unwrap();
        let mut net_devs = Vec::new();
        net_devs.try_push(eth0).unwrap();
        net_devs.try_push(eth1).unwrap();
        let packet_type =
            PacketType::new(unsafe { &mut PACKET_TYPE }, ETH_P_ALL, eth_rcv, net_devs);

        Ok(RustModule { packet_type })
    }
}

impl Drop for RustModule {
    fn drop(&mut self) {
        pr_info!("Rust minimal sample (exit)\n");
    }
}
