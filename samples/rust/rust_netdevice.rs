use core::ffi::c_void;
use core::mem::MaybeUninit;
use kernel::bindings::{
    dev_add_pack, dev_remove_pack, net_device, netdev_get_by_name, netdev_put, netdevice_tracker,
    packet_type, sk_buff, GFP_KERNEL,
};
use kernel::prelude::*;
use kernel::sync::Arc;
use kernel::types::ForeignOwnable;
use kernel::{fmt, str::CString};

use crate::rust_namespace::NetNamespace;

pub(crate) struct PacketType {
    #[allow(dead_code)]
    inner: Pin<&'static mut MaybeUninit<packet_type>>,
}

impl PacketType {
    pub(crate) fn new(
        holder: &'static mut MaybeUninit<packet_type>,
        ether_type: u32,
        pkt_handler: unsafe extern "C" fn(
            skb: *mut sk_buff,
            dev_in: *mut net_device,
            _packet_type: *mut packet_type,
            _orig_dev: *mut net_device,
        ) -> i32,
        private: *const c_void,
    ) -> Self {
        let mut packet_type = Pin::static_mut(holder);
        let ether_type = ether_type as u16;
        unsafe {
            (*(*packet_type).as_mut_ptr()).type_ = ether_type.to_be();
            (*(*packet_type).as_mut_ptr()).func = Some(pkt_handler);
            (*(*packet_type).as_mut_ptr()).af_packet_priv = private as *mut c_void;
            dev_add_pack((*packet_type).as_mut_ptr());
        }
        Self { inner: packet_type }
    }
}

impl Drop for PacketType {
    fn drop(&mut self) {
        unsafe {
            let priv_data = (*(*self.inner).as_mut_ptr()).af_packet_priv;
            dev_remove_pack((*self.inner).as_mut_ptr());
            let _d: Arc<Vec<NetDevice>> = Arc::from_foreign(priv_data);
            pr_info!("PacketType dropped\n");
        }
    }
}

pub(crate) struct NetDevice {
    #[allow(dead_code)]
    ns: Arc<NetNamespace>,
    #[allow(dead_code)]
    net_device: *mut net_device,
    netdev_tracker: Pin<&'static mut MaybeUninit<netdevice_tracker>>,
}

impl NetDevice {
    pub(crate) fn new(
        namespace: Arc<NetNamespace>,
        name: &str,
        netdev_tracker: &'static mut MaybeUninit<netdevice_tracker>,
    ) -> Option<Self> {
        let dev_name = CString::try_from_fmt(fmt!("{}", name)).unwrap();
        let dev = unsafe {
            netdev_get_by_name(
                namespace.get_net(),
                dev_name.as_char_ptr(),
                (*netdev_tracker).as_mut_ptr(),
                GFP_KERNEL,
            )
        };

        if dev.is_null() {
            pr_info!("Can't find dev: {}\n", name);
            return None;
        }

        pr_info!(
            "Acquiring netns_tracker: {:p}\n",
            (*netdev_tracker).as_mut_ptr()
        );
        let c_str = unsafe { CStr::from_char_ptr(&(*dev).name as *const [i8; 16] as *const i8) };
        pr_info!("Got a netdev by name: {}\n", c_str.to_str().unwrap());
        Some(Self {
            ns: namespace,
            net_device: dev,
            netdev_tracker: Pin::static_mut(netdev_tracker),
        })
    }

    pub(crate) fn get_dev(&self) -> *mut net_device {
        self.net_device
    }
}

impl PartialEq for NetDevice {
    fn eq(&self, other: &Self) -> bool {
        self.net_device == other.net_device
    }

    fn ne(&self, other: &Self) -> bool {
        self.net_device != other.net_device
    }
}

impl Drop for NetDevice {
    fn drop(&mut self) {
        unsafe { netdev_put(self.net_device, (*self.netdev_tracker).as_mut_ptr()) };
        pr_info!("NetDevice dropped\n");
    }
}
