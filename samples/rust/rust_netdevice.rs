use core::cell::UnsafeCell;
use core::clone::Clone;
use core::ffi::c_void;
use core::mem::MaybeUninit;
use kernel::bindings::{
    dev_add_pack, dev_remove_pack, net_device, netdev_get_by_name, netdev_put, netdevice_tracker,
    packet_type, sk_buff, GFP_ATOMIC, GFP_KERNEL,
};
use kernel::sync::lock::spinlock::SpinLock;
use kernel::sync::Arc;
use kernel::types::ForeignOwnable;
use kernel::{fmt, str::CString};
use kernel::{new_spinlock, prelude::*};

use crate::rust_namespace::NetNamespace;

pub(crate) struct PacketType<T>
where
    T: 'static,
{
    #[allow(dead_code)]
    inner: Pin<Box<PacketTypeInner>>,
    private: Arc<SpinLock<T>>,
}

impl<T> PacketType<T> {
    pub(crate) fn new(
        ether_type: u32,
        pkt_handler: unsafe extern "C" fn(
            skb: *mut sk_buff,
            dev_in: *mut net_device,
            _packet_type: *mut packet_type,
            _orig_dev: *mut net_device,
        ) -> i32,
        private: T,
    ) -> Self {
        let packet_type = Box::pin_init(PacketTypeInner::new()).unwrap();
        let ether_type = ether_type as u16;
        unsafe {
            (*(*packet_type).get_raw()).type_ = ether_type.to_be();
            (*(*packet_type).get_raw()).func = Some(pkt_handler);
            let a = Arc::pin_init(new_spinlock!(
                private,
                "Wrap the private data in a spinlock"
            ))
            .unwrap();
            let priv_data = a.clone().into_foreign();
            (*(*packet_type).get_raw()).af_packet_priv = priv_data as *mut c_void;
            dev_add_pack((*packet_type).get_raw());
            Self {
                inner: packet_type,
                private: a,
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn get_private(&self) -> Arc<SpinLock<T>> {
        self.private.clone()
    }
}

impl<T> Drop for PacketType<T> {
    fn drop(&mut self) {
        unsafe {
            let priv_data = (*(*self.inner).get_raw()).af_packet_priv;
            dev_remove_pack((*self.inner).get_raw());
            let _d: Arc<SpinLock<T>> = Arc::from_foreign(priv_data);
            pr_info!("PacketType dropped\n");
        }
    }
}

#[pin_data]
pub struct PacketTypeInner {
    #[pin]
    inner: UnsafeCell<MaybeUninit<packet_type>>,
}

impl PacketTypeInner {
    pub(crate) fn new() -> impl PinInit<Self> {
        pin_init!(Self {
            inner: unsafe { UnsafeCell::new(MaybeUninit::zeroed().assume_init()) }
        })
    }

    pub(crate) fn get_raw(&self) -> *mut packet_type {
        unsafe { (*self.inner.get()).as_mut_ptr() }
    }
}

pub(crate) struct NetDevice {
    #[allow(dead_code)]
    ns: Arc<NetNamespace>,
    #[allow(dead_code)]
    net_device: *mut net_device,
    netdev_tracker: Arc<NetDeviceTracker>,
}

impl NetDevice {
    pub(crate) fn new(
        namespace: Arc<NetNamespace>,
        name: &str,
        netdev_tracker: Arc<NetDeviceTracker>,
    ) -> Option<Self> {
        let dev_name = CString::try_from_fmt(fmt!("{}", name)).unwrap();
        let dev = unsafe {
            netdev_get_by_name(
                namespace.get_net(),
                dev_name.as_char_ptr(),
                netdev_tracker.get_raw(),
                GFP_KERNEL | GFP_ATOMIC,
            )
        };

        if dev.is_null() {
            pr_info!("Can't find dev: {}\n", name);
            return None;
        }

        pr_info!("Acquiring netdev_tracker: {:p}\n", netdev_tracker.get_raw());
        let c_str = unsafe { CStr::from_char_ptr(&(*dev).name as *const [i8; 16] as *const i8) };
        pr_info!("Got a netdev by name: {}\n", c_str.to_str().unwrap());
        Some(Self {
            ns: namespace,
            net_device: dev,
            netdev_tracker,
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
        pr_info!(
            "NetDevice dropped, tracker: {:p}\n",
            self.netdev_tracker.get_raw()
        );
        unsafe { netdev_put(self.net_device, self.netdev_tracker.get_raw()) };
    }
}

#[pin_data]
pub struct NetDeviceTracker {
    #[pin]
    tracker: UnsafeCell<MaybeUninit<netdevice_tracker>>,
}
impl NetDeviceTracker {
    pub(crate) fn new() -> impl PinInit<Self> {
        pin_init!(Self {
            tracker: unsafe { UnsafeCell::new(MaybeUninit::zeroed().assume_init()) }
        })
    }

    pub(crate) fn get_raw(&self) -> *mut netdevice_tracker {
        unsafe { (*self.tracker.get()).as_mut_ptr() }
    }
}
