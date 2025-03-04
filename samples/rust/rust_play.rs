// SPDX-License-Identifier: GPL-2.0

//! Rust play module.

use kernel::alloc::allocator::Kmalloc;
use kernel::bindings::ETH_P_ALL;
use kernel::bindings::{net_device, packet_type, sk_buff};
use kernel::new_spinlock;
use kernel::sync::SpinLock;
use kernel::{
    c_str, current_net_ns, net_device::NetDevice, packet_type::PacketType, prelude::*,
    str::CString, types::ARef,
};

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

        let pt = unsafe { PacketType::new(ETH_P_ALL as u16, eth_rcv, private)? };

        Ok(RustPlay { _pt: pt })
    }
}

impl Drop for RustPlay {
    fn drop(&mut self) {
        pr_info!("Rust play module (exit)\n");
    }
}

unsafe extern "C" fn eth_rcv(
    skb: *mut sk_buff,
    dev_in: *mut net_device,
    packet_type: *mut packet_type,
    orig_dev: *mut net_device,
) -> i32 {
    assert!(!skb.is_null());
    assert!(!dev_in.is_null());
    assert!(!packet_type.is_null());
    assert!(!orig_dev.is_null());

    let dev_in = unsafe { NetDevice::from_ptr(dev_in) };
    let orig_dev = unsafe { NetDevice::from_ptr(orig_dev) };
    let private_data: Pin<&PacketTypePrivateData> =
        unsafe { PacketType::<Pin<KBox<PacketTypePrivateData>>>::borrow_private(packet_type) };

    match eth_rcv_wrapper(dev_in, private_data, orig_dev) {
        Err(e) => e.to_errno(),
        Ok(res) => res,
    }
}

fn eth_rcv_wrapper(
    // skb: TODO
    dev_in: &NetDevice,
    private_data: Pin<&PacketTypePrivateData>,
    orig_dev: &NetDevice,
) -> Result<i32> {
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
    Ok(0)
}
