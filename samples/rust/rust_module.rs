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
// use core::iter::IntoIterator;
use core::mem::MaybeUninit;
// use core::ptr::null_mut;
use kernel::bindings::{
    /* dev_add_pack, dev_remove_pack, get_net_track */ init_net, /* net, */ net_device,
    /* netdev_get_by_name, netdev_put */ netdevice_tracker, netns_tracker,
    packet_type, /* put_net_track, */
    sk_buff, /* ETH_HLEN, GFP_KERNEL, */ PACKET_LOOPBACK, PACKET_OUTGOING,
};
use kernel::prelude::*;
use kernel::sync::{Arc /*, Mutex*/};
use kernel::uapi::ETH_P_ALL;
// use kernel::{fmt, str::CString};

mod rust_skb;
use rust_skb::SkBuffOwned;

module! {
    type: RustModule,
    name: "rust_module",
    author: "Rust for Linux Contributors",
    description: "Play with rust in the kernel",
    license: "GPL",
}

// pub struct Container {
//     net: *mut net,
//     dev: Vec<*mut net_device>,
//     // packet_type: packet_type,
//     netns_tracker: netns_tracker,
//     netdev_tracker: netdevice_tracker,
//     stopped: bool,
// }

// impl Container {
// fn get_ns(&self) -> *mut net {
//     self.net
// }

// fn set_ns(&mut self, ns: *mut net) {
//     self.net = ns;
// }

// fn set_dev(&mut self, dev: *mut net_device) {
//     let _ = self.dev.try_push(dev);
// }

// #[allow(dead_code)]
// fn add_packet_type(&mut self) {
//     let ether_type = ETH_P_ALL as u16;
//     self.packet_type.type_ = ether_type.to_be();
//     self.packet_type.func = Some(Container::eth_rcv);
//     unsafe {
//         dev_add_pack(&mut self.packet_type);
//     }
// }

// #[allow(dead_code)]
// fn remove_packet_type(&mut self) {
//     unsafe {
//         dev_remove_pack(&mut self.packet_type);
//     }
// }

// pub fn get_netns_tracker(&mut self) -> &mut netns_tracker {
//     &mut self.netns_tracker
// }

// pub fn get_netdev_tracker(&mut self) -> &mut netdevice_tracker {
//     &mut self.netdev_tracker
// }

// pub fn acquire_net(&mut self) {
//     if self.get_ns().is_null() {
//         let net = unsafe { get_net_track(&mut init_net, self.get_netns_tracker(), GFP_KERNEL) };
//         self.set_ns(net);
//     }
// }

// pub fn acquire_dev(&mut self, name: &str) {
//     self.acquire_net();
//     let dev_name = CString::try_from_fmt(fmt!("{}", name)).unwrap();
//     let dev = unsafe {
//         netdev_get_by_name(
//             self.net,
//             dev_name.as_char_ptr(),
//             self.get_netdev_tracker(),
//             GFP_KERNEL,
//         )
//     };

//     if dev.is_null() {
//         pr_info!("Can't find dev: {}\n", name);
//         return;
//     }

//     pr_info!("Acquiring netns_tracker: {:p}\n", self.get_netns_tracker());
//     let c_str = unsafe { CStr::from_char_ptr(&(*dev).name as *const [i8; 16] as *const i8) };
//     pr_info!("Got a netdev by name: {}\n", c_str.to_str().unwrap());
//     self.set_dev(dev);
// }

// pub fn release_dev(&mut self) {
//     let devs_num = self.dev.len();
//     pr_info!("Releasing {} netdevs !\n", devs_num);
//     for i in 0..devs_num {
//         unsafe { netdev_put(self.dev[i], self.get_netdev_tracker()) };
//     }
//     pr_info!("Releasing netns_tracker: {:p}\n", self.get_netns_tracker());
//     unsafe { put_net_track(self.get_ns(), self.get_netns_tracker()) };
//     pr_info!("Releasing net namespace!\n");
// }

// pub fn new() -> Pin<&'static mut Container> {
//     let lock = Arc::pin_init(new_mutex!((), "Container Mutex")).unwrap();
//     unsafe { MTX = Some(lock) };
//     let _mg = unsafe { MTX.as_mut().unwrap().lock() };

//     let our_cont = unsafe { &mut CONTAINER };
//     our_cont.acquire_dev("eth0");
//     our_cont.acquire_dev("eth1");
//     // our_cont.add_packet_type();
//     our_cont.stopped = false;

//     let cont = Pin::static_mut(our_cont);
//     cont
// }

// pub fn deinit(cont: &mut Pin<&'static mut Container>) {
//     let _mg = unsafe { MTX.as_mut().unwrap().lock() };

//     cont.stopped = true;
//     cont.release_dev();
//     // cont.remove_packet_type();
// }

// pub fn get_egress_devs(dev_in: *mut net_device) -> Vec<*mut net_device> {
//     let mut egress_devs = Vec::new();
//     let _mg = unsafe { MTX.as_mut().unwrap().lock() };
//     let stopped = unsafe { CONTAINER.stopped };
//     if stopped == false {
//         unsafe {
//             CONTAINER.dev.iter_mut().for_each(|dev| {
//                 if dev_in != *dev {
//                     egress_devs.try_push(*dev).unwrap();
//                 }
//             });
//         };
//     }
//     egress_devs
// }

// pub fn get_dev_name<'a>(dev: *mut net_device) -> &'a CStr {
//     unsafe { CStr::from_char_ptr(&(*dev).name as *const [i8; 16] as *const i8) }
// }

// #[allow(dead_code)]
// pub unsafe extern "C" fn eth_rcv(
//     skb: *mut sk_buff,
//     dev_in: *mut net_device,
//     _packet_type: *mut packet_type,
//     _orig_dev: *mut net_device,
// ) -> i32 {
//     let skb = unsafe { SkBuffOwned::from_raw(skb) };
//     let pkt_type = skb.get_pkt_type();

//     // if we don't filter these there's a loop
//     if pkt_type == PACKET_LOOPBACK || pkt_type == PACKET_OUTGOING {
//         return 0;
//     }

//     // pr_info!("Received frame!\n");
//     let egress_devs = Container::get_egress_devs(dev_in);
//     egress_devs.into_iter().for_each(|dev| {
//         let mut nskb = skb.clone();
//         nskb.set_dev(dev);
//         // let dev_out_name = Container::get_dev_name(dev);
//         // let dev_in_name = Container::get_dev_name(dev_in);
//         // pr_info!(
//         //     "Forwarding packet from: {} to: {}\n",
//         //     dev_in_name.to_str().unwrap(),
//         //     dev_out_name.to_str().unwrap()
//         // );
//         nskb.undo_skb_pull(ETH_HLEN as usize);
//         nskb.dev_queue_xmit();
//     });
//     0
// }
// }

pub unsafe extern "C" fn eth_rcv(
    skb: *mut sk_buff,
    _dev_in: *mut net_device,
    _packet_type: *mut packet_type,
    _orig_dev: *mut net_device,
) -> i32 {
    let skb = unsafe { SkBuffOwned::from_raw(skb) };
    let pkt_type = skb.get_pkt_type();

    // if we don't filter these there's a loop
    if pkt_type == PACKET_LOOPBACK || pkt_type == PACKET_OUTGOING {
        return 0;
    }

    // pr_info!("Received frame!\n");
    // let egress_devs = Container::get_egress_devs(dev_in);
    // egress_devs.into_iter().for_each(|dev| {
    //     let mut nskb = skb.clone();
    //     nskb.set_dev(dev);
    //     // let dev_out_name = Container::get_dev_name(dev);
    //     // let dev_in_name = Container::get_dev_name(dev_in);
    //     // pr_info!(
    //     //     "Forwarding packet from: {} to: {}\n",
    //     //     dev_in_name.to_str().unwrap(),
    //     //     dev_out_name.to_str().unwrap()
    //     // );
    //     nskb.undo_skb_pull(ETH_HLEN as usize);
    //     nskb.dev_queue_xmit();
    // });
    0
}

// static mut CONTAINER: Container = Container {
//     net: null_mut(),
//     dev: Vec::new(),
//     netns_tracker: netns_tracker {},
//     netdev_tracker: netdevice_tracker {},
//     // packet_type: unsafe { MaybeUninit::zeroed().assume_init() },
//     stopped: true,
// };

mod rust_netdevice;
use rust_netdevice::{NetDevice, PacketType};
static mut PACKET_TYPE: MaybeUninit<packet_type> = MaybeUninit::zeroed();
static mut NETDEV_TRACKER: MaybeUninit<netdevice_tracker> = MaybeUninit::zeroed();
mod rust_namespace;
use rust_namespace::NetNamespace;
static mut NET_NS_TRACKER: MaybeUninit<netns_tracker> = MaybeUninit::zeroed();

struct RustModule {
    // cont: Pin<&'static mut Container>,
    #[allow(dead_code)]
    net_devs: Vec<NetDevice>,
    #[allow(dead_code)]
    packet_type: PacketType,
}

unsafe impl Sync for RustModule {}

// use kernel::new_mutex;
// static mut MTX: Option<Arc<Mutex<()>>> = None;

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
        // let cont = Container::new();
        let packet_type = PacketType::new(unsafe { &mut PACKET_TYPE }, ETH_P_ALL, eth_rcv);

        Ok(RustModule {
            net_devs,
            // cont,
            packet_type,
        })
    }
}

impl Drop for RustModule {
    fn drop(&mut self) {
        pr_info!("Rust minimal sample (exit)\n");
        // Container::deinit(&mut self.cont);
    }
}
