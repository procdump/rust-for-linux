// SPDX-License-Identifier: GPL-2.0

//! Rust play module.

use kernel::alloc::allocator::Kmalloc;
use kernel::bindings::ETH_P_ALL;
use kernel::bindings::{net_device, packet_type, sk_buff};
use kernel::new_spinlock;
use kernel::sk_buff::SkBuff;
use kernel::sync::SpinLock;
use kernel::{
    c_str, current_net_ns, net_device::NetDevice, packet_type::PacketType, prelude::*,
    str::CString, types::ARef,
};
use kernel::pkt_handler;

module! {
    type: RustPlay,
    name: "rust_play",
    author: "Boris Astardzhiev",
    description: "Rust play module",
    license: "GPL",
}

#[pin_data]
struct PacketTypePrivateData {
    #[pin]
    data: SpinLock<PacketTypePrivateDataInner>,
}

struct PacketTypePrivateDataInner {
    _devs: Vec<ARef<NetDevice>, Kmalloc>,
}

impl PacketTypePrivateData {
    fn new(devs: Vec<ARef<NetDevice>, Kmalloc>) -> impl PinInit<Self> {
        pin_init!(Self {
            data <- new_spinlock!(PacketTypePrivateDataInner { _devs: devs }),
        })
    }
}

struct RustPlay {
    _pt: PacketType<Pin<KBox<PacketTypePrivateData>>>,
}

impl kernel::Module for RustPlay {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust play module (init)\n");
        pr_info!("Am I built-in? {}\n", !cfg!(MODULE));

        let pid = current!().pid();
        pr_info!("In process context from pid: {}\n", pid);
        {
            let net_ns = current_net_ns!();
            pr_info!("NetNamespace for pid: {} is: {:p}", pid, net_ns);
        }

        let net_ns = current!().get_net_ns();
        pr_info!(
            "NetNamespace for pid: {} is here: {}\n",
            pid,
            net_ns.is_some()
        );

        let _cstring = CString::try_from_fmt(fmt!("{}", "my_module")).unwrap();
        let netdev = NetDevice::get_by_name(current_net_ns!(), c_str!("eth0")).unwrap();

        let mut devs = KVec::new();
        devs.push(netdev, GFP_KERNEL)?;

        let private = PacketTypePrivateData::new(devs);
        let private = KBox::pin_init(private, GFP_ATOMIC | GFP_KERNEL)?;

        let pt =
            unsafe { PacketType::new((&net_ns).as_ref(), ETH_P_ALL as u16, eth_rcv, private)? };

        Ok(RustPlay { _pt: pt })
    }
}

impl Drop for RustPlay {
    fn drop(&mut self) {
        pr_info!("Rust play module (exit)\n");
    }
}

// Declare and glue the packet handler function to the provided callback.
pkt_handler!(eth_rcv, eth_rcv_inner);

#[inline]
fn eth_rcv_inner(
    skb: ARef<SkBuff<'_>>,
    dev_in: &NetDevice,
    private_data: Pin<&PacketTypePrivateData>,
    orig_dev: &NetDevice,
) -> Result<i32> {
    let pkt_type = skb.get_pkt_type()?;

    pr_info!("pkt_type: {:?}\n", pkt_type);
    match skb.get_dev() {
        None => {
            pr_info!("Missing dev in sk_buff!\n");
        }
        Some(dev) => {
            pr_info!("skb->dev: {}\n", dev.name())
        }
    }
    // Filter these.
    if pkt_type == kernel::sk_buff::PacketType::Loopback
        || pkt_type == kernel::sk_buff::PacketType::Outgoing
    {
        return Ok(kernel::bindings::NET_RX_DROP as i32);
    }

    let _deep_clone = skb.deep_clone(GFP_ATOMIC)?;
    let _shallow_clone = _deep_clone.shallow_clone(GFP_ATOMIC)?;
    // drop(_deep_clone);
    let _data = _shallow_clone.head_data();
    pr_info!("data: {:02x?}\n", _data);

    let orig_dev_name = orig_dev.name().to_str()?;
    let dev_in_name = dev_in.name().to_str()?;
    pr_info!(
        "orig_dev_name: {}, dev_in_name: {}\n",
        orig_dev_name,
        dev_in_name
    );

    let priv_inner = private_data.data.lock();
    for dev in &priv_inner._devs {
        let db_dev_name = dev.name().to_str()?;
        if orig_dev_name == db_dev_name {
            pr_info!(
                "Got a packet from a net device in our DB -> {}\n",
                db_dev_name
            );
        }
    }
    Ok(kernel::bindings::NET_RX_DROP as i32)
}
