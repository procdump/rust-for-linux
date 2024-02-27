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
use kernel::bindings::{
    jiffies, net_device, packet_type, sk_buff, ETH_HLEN, PACKET_LOOPBACK, PACKET_OUTGOING,
};
use kernel::prelude::*;
use kernel::rbtree::RBTree;
use kernel::sync::lock::mutex::Mutex;
use kernel::sync::{Arc, ArcBorrow};
use kernel::time::{msecs_to_jiffies, Jiffies};
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
        skb.dev_queue_xmit();
    }

    #[inline]
    fn flood(priv_data: &mut PrivateData, skb: SkBuffOwned<'_>, dev_in: *mut net_device) -> i32 {
        let mut it = priv_data
            .net_devs
            .iter()
            .map(|arc| arc.as_ref())
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

    pub(crate) unsafe extern "C" fn eth_rcv(
        skb: *mut sk_buff,
        dev_in: *mut net_device,
        packet_type: *mut packet_type,
        _orig_dev: *mut net_device,
    ) -> i32 {
        let mut skb = unsafe { SkBuffOwned::from_raw(skb) };
        let priv_data: ArcBorrow<'_, Mutex<RefCell<PrivateData>>> =
            unsafe { Arc::borrow((*packet_type).af_packet_priv) };

        let pkt_type = skb.get_pkt_type();
        // if we don't filter these there's a loop
        if pkt_type == PACKET_LOOPBACK || pkt_type == PACKET_OUTGOING {
            return 0;
        }

        // pr_info!("Received frame!\n");

        skb.push(ETH_HLEN as usize);
        // check if broadcast or multicast
        if HUB_MODE == true || skb.is_ether_broadcast() || skb.is_ether_multicast() {
            // pr_info!("HUB/broadcast or multicast -> FLOOD frame\n");
            let locked = priv_data.lock();
            let mut priv_data = locked.borrow_mut();
            RustModule::flood(&mut *priv_data, skb, dev_in)
        } else {
            let ether_dhost = skb.get_ether_dhost();
            let ether_shost = skb.get_ether_shost();

            let locked = priv_data.lock();
            let mut priv_data = locked.borrow_mut();
            let dev_rcvd = priv_data
                .net_devs
                .iter()
                .filter(|dev| dev.get_dev() == dev_in)
                .last()
                .map(|dev| Some(dev.clone()))
                .unwrap_or(None);
            if dev_rcvd.is_none() {
                // pr_info!("Frame received on unconfigured dev\n");
                return 0;
            }
            let dev_rcvd = dev_rcvd.unwrap();

            // update fdb first
            let mac_en = MacEntry::new(dev_rcvd.clone(), msecs_to_jiffies(MAX_AGE_MSEC));
            let found = priv_data.fdb.get_mut(&ether_shost);
            match found {
                Some(mac_entry) => {
                    // dev_in might have changed
                    // pr_info!("Update dev_rcvd!\n");
                    *mac_entry = mac_en;
                }
                None => {
                    // TODO: expiry? fdb size limit?
                    // pr_info!("Insert ether shost into fdb\n");
                    let _ = priv_data.fdb.try_create_and_insert(ether_shost, mac_en);
                }
            }

            // now lookup by dest address and choose dev(s) out
            let found = priv_data.fdb.get(&ether_dhost);
            match found {
                Some(e) => {
                    // pr_info!("Send to specific dev\n");
                    skb.set_dev(e.get_dev());
                    skb.dev_queue_xmit();
                    0
                }
                None => {
                    // pr_info!("Flood!\n");
                    RustModule::flood(&mut *priv_data, skb, dev_in)
                }
            }
        }
    }
}

unsafe impl Sync for RustModule {}

struct PrivateData {
    net_devs: Vec<Arc<NetDevice>>,
    #[allow(dead_code)]
    fdb: RBTree<[u8; 6], MacEntry>,
}

struct MacEntry {
    dev: Arc<NetDevice>,
    #[allow(dead_code)]
    expires_in: Jiffies,
}

impl MacEntry {
    pub(crate) fn new(dev: Arc<NetDevice>, expires_in: Jiffies) -> Self {
        let now = unsafe { jiffies };
        let expires_in = now + expires_in;
        // pr_info!("Now {}, new jiffies: {}\n", now, expires_in);
        Self { dev, expires_in }
    }

    #[allow(dead_code)]
    pub(crate) fn get_dev(&self) -> *mut net_device {
        self.dev.get_dev()
    }

    #[allow(dead_code)]
    pub(crate) fn get_expires_in(&self) -> Jiffies {
        self.expires_in
    }
}

const HUB_MODE: bool = true;
const ETH0: &'static str = "eth0";
const ETH1: &'static str = "eth1";
const MAX_AGE_MSEC: u32 = 180 * 1000;

impl kernel::Module for RustModule {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust minimal sample (init)\n");
        pr_info!("Am I built-in? {}\n", !cfg!(MODULE));

        let netns_tracker = Arc::pin_init(NetNsTracker::new()).unwrap();
        let net_ns = Arc::try_new(NetNamespace::new(
            NetNamespace::default_net(),
            netns_tracker,
        ))
        .unwrap();
        let netdevice_tracker = Arc::pin_init(NetDeviceTracker::new()).unwrap();
        let eth0 =
            Arc::try_new(NetDevice::new(net_ns.clone(), ETH0, netdevice_tracker.clone()).unwrap())
                .unwrap();
        let eth1 = Arc::try_new(NetDevice::new(net_ns, ETH1, netdevice_tracker).unwrap()).unwrap();
        let mut net_devs = Vec::new();
        net_devs.try_push(eth0).unwrap();
        net_devs.try_push(eth1).unwrap();

        let fdb = RBTree::new();
        let packet_type = PacketType::new(
            ETH_P_ALL,
            RustModule::eth_rcv,
            PrivateData { net_devs, fdb },
        );

        let _priv_data = packet_type.get_private();

        Ok(RustModule { packet_type })
    }
}

impl Drop for RustModule {
    fn drop(&mut self) {
        pr_info!("Rust minimal sample (exit)\n");
    }
}
